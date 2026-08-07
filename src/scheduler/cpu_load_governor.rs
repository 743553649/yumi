/*
 * Copyright (C) 2026 yuki
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::scheduler::config::CpuLoadGovernorConfig;
use crate::utils::FastWriter;
use log::{debug, info, warn};
use std::fs;

use crate::fluent_args;
use crate::i18n::{t, t_with_args};

// ════════════════════════════════════════════════════════════════
//  ClusterState — 单 cluster 运行时状态
// ════════════════════════════════════════════════════════════════

struct ClusterState {
    policy_id: i32,
    affected_cpus: Vec<usize>,
    available_freqs: Vec<u32>,
    cached_ratios: Vec<f32>,
    _freq_min: f32,
    _freq_max: f32,
    max_writer: FastWriter,
    min_writer: FastWriter,
    current_perf: f32,
    current_freq: u32,
    down_wait: u32,
    up_wait: u32,
    prev_util: f32,
    // 接管前状态快照 (3.3)
    pre_takeover_gov: String,
    pre_takeover_min_freq: u32,
    pre_takeover_max_freq: u32,
}

impl ClusterState {
    fn find_nearest_freq(&self, target_ratio: f32) -> u32 {
        let idx = self.cached_ratios.partition_point(|&r| r < target_ratio);
        if idx == 0 {
            self.available_freqs[0]
        } else if idx >= self.available_freqs.len() {
            // available_freqs 由 init_policies 的 freqs.is_empty() 守卫保证非空；防御性回退到 current_freq（不变频）
            *self.available_freqs.last().unwrap_or(&self.current_freq)
        } else {
            let lo = idx - 1;
            let hi = idx;
            if (self.cached_ratios[hi] - target_ratio).abs()
                < (self.cached_ratios[lo] - target_ratio).abs()
            {
                self.available_freqs[hi]
            } else {
                self.available_freqs[lo]
            }
        }
    }

    fn write_freq(&mut self, freq: u32) {
        if freq == self.current_freq {
            return;
        }
        let ok = if freq >= self.current_freq {
            // 升频：先拉高 max 再拉高 min
            let ok_max = self.max_writer.write_value_force(freq);
            let ok_min = self.min_writer.write_value_force(freq);
            ok_max && ok_min
        } else {
            // 降频：先降 min 再降 max
            let ok_min = self.min_writer.write_value_force(freq);
            let ok_max = self.max_writer.write_value_force(freq);
            ok_max && ok_min
        };
        // 仅在两端均写入成功时更新缓存，失败则下次 tick 自动重试
        if ok {
            self.current_freq = freq;
        }
    }

    fn max_util(&self, core_utils: &[f32]) -> f32 {
        self.affected_cpus
            .iter()
            .filter_map(|&cpu| core_utils.get(cpu))
            .copied()
            .fold(0.0_f32, f32::max)
    }
}

// ════════════════════════════════════════════════════════════════
//  CpuLoadGovernor — 主控制器
// ════════════════════════════════════════════════════════════════

pub struct CpuLoadGovernor {
    clusters: Vec<ClusterState>,
    cfg: CpuLoadGovernorConfig,
    active: bool,
    log_counter: u32,
}

impl CpuLoadGovernor {
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
            cfg: CpuLoadGovernorConfig::default(),
            active: false,
            log_counter: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn init_policies(&mut self, gov_cfg: &CpuLoadGovernorConfig) {
        self.release();
        self.cfg = gov_cfg.clone();

        let clusters = crate::scheduler::get_cpu_policies();

        for policy in &clusters {
            let pid = policy.id;
            let gov_path = format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_governor",
                pid
            );
            let _ = crate::utils::try_write_file(&gov_path, "performance");

            let freq_path = format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_available_frequencies",
                pid
            );
            let mut freqs: Vec<u32> = fs::read_to_string(&freq_path)
                .unwrap_or_default()
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if freqs.is_empty() {
                continue;
            }
            freqs.sort_unstable();
            freqs.dedup();

            // 合并 boost 频率（部分平台额外暴露的高频点），去重排序
            if !policy.boost_frequencies.is_empty() {
                freqs.extend(&policy.boost_frequencies);
                freqs.sort_unstable();
                freqs.dedup();
            }

            let affected = Self::read_affected_cpus(pid);
            if affected.is_empty() {
                continue;
            }

            let fmin = *freqs.first().unwrap_or(&0) as f32;
            let fmax = *freqs.last().unwrap_or(&0) as f32;
            let range = (fmax - fmin).max(1.0);
            let cached_ratios: Vec<f32> =
                freqs.iter().map(|&f| (f as f32 - fmin) / range).collect();

            let max_writer = FastWriter::new(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_max_freq",
                pid
            ));
            let min_writer = FastWriter::new(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq",
                pid
            ));

            if !max_writer.is_valid() || !min_writer.is_valid() {
                warn!(
                    "{}",
                    t_with_args(
                        "clg-writer-invalid",
                        &fluent_args!(
                            "pid" => pid.to_string(),
                            "max_valid" => max_writer.is_valid().to_string(),
                            "min_valid" => min_writer.is_valid().to_string()
                        )
                    )
                );
                continue;
            }

            let (safe_floor, safe_ceil) = if self.cfg.perf_floor > self.cfg.perf_ceil {
                (self.cfg.perf_ceil, self.cfg.perf_ceil)
            } else {
                (self.cfg.perf_floor, self.cfg.perf_ceil)
            };
            let init_perf = self.cfg.perf_init.clamp(safe_floor, safe_ceil);
            // 保存接管前状态快照 (3.3)
            let pre_gov = fs::read_to_string(&gov_path)
                .unwrap_or_default()
                .trim()
                .to_string();
            let pre_min = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq",
                pid
            ))
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);
            let pre_max = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_max_freq",
                pid
            ))
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);

            let mut cluster = ClusterState {
                policy_id: pid,
                affected_cpus: affected.clone(),
                available_freqs: freqs,
                cached_ratios,
                _freq_min: fmin,
                _freq_max: fmax,
                max_writer,
                min_writer,
                current_perf: init_perf,
                current_freq: 0,
                down_wait: 0,
                up_wait: 0,
                prev_util: 0.0,
                pre_takeover_gov: pre_gov,
                pre_takeover_min_freq: pre_min,
                pre_takeover_max_freq: pre_max,
            };

            let init_freq = cluster.find_nearest_freq(init_perf);
            cluster.max_writer.write_value_force(init_freq);
            cluster.min_writer.write_value_force(init_freq);
            cluster.current_freq = init_freq;

            info!(
                "{}",
                t_with_args(
                    "clg-init",
                    &fluent_args!(
                        "pid" => pid.to_string(),
                        "cpus" => format!("{:?}", affected),
                        "fmin" => (fmin / 1000.0).to_string(),
                        "fmax" => (fmax / 1000.0).to_string(),
                        "perf" => format!("{:.2}", init_perf),
                        "freq" => (init_freq / 1000).to_string()
                    )
                )
            );

            self.clusters.push(cluster);
        }

        self.active = !self.clusters.is_empty();
        if self.active {
            info!(
                "{}",
                t_with_args(
                    "clg-activated",
                    &fluent_args!("count" => self.clusters.len().to_string())
                )
            );
        } else {
            warn!("{}", t("clg-no-clusters"));
        }
    }

    pub fn release(&mut self) {
        if self.active {
            // 恢复接管前的 governor 和频率
            for c in &mut self.clusters {
                let gov_path = format!(
                    "/sys/devices/system/cpu/cpufreq/policy{}/scaling_governor",
                    c.policy_id
                );
                if !c.pre_takeover_gov.is_empty() {
                    let _ = crate::utils::try_write_file(&gov_path, c.pre_takeover_gov.as_bytes());
                }
                // 恢复频率范围，读取失败不写退化值
                if c.pre_takeover_min_freq > 0 {
                    let _ = c.min_writer.write_value_force(c.pre_takeover_min_freq);
                }
                if c.pre_takeover_max_freq > 0 {
                    let _ = c.max_writer.write_value_force(c.pre_takeover_max_freq);
                }
            }
            info!("{}", t("clg-deactivated"));
        }
        self.clusters.clear();
        self.active = false;
        self.log_counter = 0;
    }

    pub fn reload_config(&mut self, gov_cfg: &CpuLoadGovernorConfig) {
        self.cfg = gov_cfg.clone();
        debug!("{}", t("clg-config-reloaded"));
    }

    pub fn on_load_update(&mut self, core_utils: &[f32]) {
        if !self.active {
            return;
        }

        for cluster in &mut self.clusters {
            let util = cluster.max_util(core_utils);
            let old_perf = cluster.current_perf;
            let raw_util = util;

            // ── 尖峰抑制 ──
            // 单 tick 负载跳升超过阈值时衰减其增量，
            // 孤立瞬时尖峰（如单核 0↔100%）不再瞬间拉满性能；
            // 持续负载下一 tick 即全量生效
            let util = {
                let delta = util - cluster.prev_util;
                if delta > self.cfg.spike_jump_threshold {
                    cluster.prev_util + delta * self.cfg.spike_decay
                } else {
                    util
                }
            };
            cluster.prev_util = raw_util;

            // ── headroom 平滑过渡 ──
            // 在 up_threshold 附近线性渐变，消除负载临界时的频率振荡
            let ramp = self.cfg.headroom_ramp.max(0.01);
            let hr_factor = if util >= self.cfg.up_threshold {
                self.cfg.headroom_factor
            } else if util >= self.cfg.up_threshold - ramp {
                let t = (util - (self.cfg.up_threshold - ramp)) / ramp;
                1.0 + (self.cfg.headroom_factor - 1.0) * t
            } else {
                1.0
            };

            let target_perf = (util * hr_factor).clamp(self.cfg.perf_floor, self.cfg.perf_ceil);

            if target_perf > old_perf {
                // ── 升频路径 ──
                cluster.down_wait = 0;
                cluster.up_wait += 1;

                if cluster.up_wait < self.cfg.up_rate_limit_ticks {
                    continue;
                }

                let is_high_load = util >= self.cfg.up_threshold;
                let is_significant_jump = target_perf > old_perf + self.cfg.up_jump_threshold;

                if is_high_load || is_significant_jump {
                    // 高负载或大跳变：正常升频
                    cluster.current_perf += (target_perf - old_perf) * self.cfg.smoothing_up;
                } else {
                    // 滞回带内升频：速率随 util 接近 up_threshold 线性提升
                    let proximity = ((util - self.cfg.down_threshold)
                        / (self.cfg.up_threshold - self.cfg.down_threshold).max(0.01))
                    .clamp(0.0, 1.0);
                    let slow_scale = self.cfg.slow_up_scale
                        + (self.cfg.smoothing_up - self.cfg.slow_up_scale) * proximity;
                    cluster.current_perf += (target_perf - old_perf) * slow_scale;
                }
            } else {
                // ── 降频路径 (Race-to-Idle 极速下线) ──
                cluster.up_wait = 0;
                cluster.down_wait += 1;

                let fast_down = util < self.cfg.down_fast_threshold;
                let is_load_collapsing = (util < self.cfg.down_threshold && target_perf < old_perf)
                    || util < cluster.prev_util * 0.85;
                let can_down = fast_down
                    || is_load_collapsing
                    || cluster.down_wait >= self.cfg.down_rate_limit_ticks;

                if can_down && target_perf < old_perf {
                    let smooth = if fast_down || is_load_collapsing {
                        // 极速归位：使用陡峭降频倍数 (Race-to-Idle)
                        self.cfg.smoothing_down * self.cfg.down_fast_mult
                    } else {
                        // 滞回带内 (down_threshold ~ up_threshold)：按慢速回落
                        self.cfg.smoothing_down * self.cfg.slow_down_scale
                    };
                    cluster.current_perf += (target_perf - old_perf) * smooth;
                    if fast_down || is_load_collapsing {
                        cluster.down_wait = 0;
                    }
                }
            }

            cluster.current_perf = cluster
                .current_perf
                .clamp(self.cfg.perf_floor, self.cfg.perf_ceil);
            let target_freq = cluster.find_nearest_freq(cluster.current_perf);
            cluster.write_freq(target_freq);
        }

        self.log_counter += 1;
        if self.log_counter % 25 == 0 {
            for c in &mut self.clusters {
                debug!(
                    "{}",
                    t_with_args(
                        "clg-tick-log",
                        &fluent_args!(
                            "pid" => c.policy_id.to_string(),
                            "util" => format!("{:.0}", c.max_util(core_utils) * 100.0),
                            "perf" => format!("{:.2}", c.current_perf),
                            "freq" => (c.current_freq / 1000).to_string(),
                            "boost" => format!("{:.0}", c.available_freqs.last().copied().unwrap_or(0) as f32 / 1000.0)
                        )
                    )
                );
            }
        }
    }

    fn read_affected_cpus(policy_id: i32) -> Vec<usize> {
        let path = format!(
            "/sys/devices/system/cpu/cpufreq/policy{}/affected_cpus",
            policy_id
        );
        fs::read_to_string(&path)
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .collect()
    }
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个单 cluster 测试用 governor，绕开 init_policies 的 sysfs 依赖。
    /// FastWriter 指向不存在的路径 → file=None，write_value_force 返回 false，
    /// current_freq 不会更新，但 current_perf 的计算逻辑仍完整执行——这正是要测的。
    fn make_test_governor(init_perf: f32) -> CpuLoadGovernor {
        // 频率档位: 300k / 600k / 1000k / 1500k / 2000k
        let freqs = vec![300_000, 600_000, 1_000_000, 1_500_000, 2_000_000];
        let fmin = 300_000.0_f32;
        let fmax = 2_000_000.0_f32;
        let range = fmax - fmin;
        let cached_ratios: Vec<f32> = freqs.iter().map(|&f| (f as f32 - fmin) / range).collect();

        let cluster = ClusterState {
            policy_id: 0,
            affected_cpus: vec![0, 1],
            available_freqs: freqs,
            cached_ratios,
            _freq_min: fmin,
            _freq_max: fmax,
            max_writer: FastWriter::new("/tmp/yumi_test_clg_max_freq"),
            min_writer: FastWriter::new("/tmp/yumi_test_clg_min_freq"),
            current_perf: init_perf,
            current_freq: 0,
            down_wait: 0,
            up_wait: 0,
            prev_util: 0.0,
            pre_takeover_gov: String::new(),
            pre_takeover_min_freq: 0,
            pre_takeover_max_freq: 0,
        };

        let mut gov = CpuLoadGovernor::new();
        gov.cfg = CpuLoadGovernorConfig::default();
        gov.clusters.push(cluster);
        gov.active = true;
        gov
    }

    #[test]
    fn test_find_nearest_freq_picks_closest() {
        // ratios: [0.0, 0.176, 0.412, 0.706, 1.0]
        // 频率:   [300k, 600k,  1000k, 1500k, 2000k]
        let gov = make_test_governor(0.5);
        let c = &gov.clusters[0];
        assert_eq!(c.find_nearest_freq(0.0), 300_000, "下界应映射到最低档");
        assert_eq!(c.find_nearest_freq(1.0), 2_000_000, "上界应映射到最高档");
        // 0.412 对应 1000k
        assert_eq!(c.find_nearest_freq(0.412), 1_000_000);
        // 0.5 离 0.412(1000k) 更近
        assert_eq!(c.find_nearest_freq(0.5), 1_000_000);
        // 0.6 离 0.706(1500k) 更近
        assert_eq!(c.find_nearest_freq(0.6), 1_500_000);
    }

    #[test]
    fn test_max_util_picks_max_across_affected_cpus() {
        let gov = make_test_governor(0.5);
        let c = &gov.clusters[0];
        // affected_cpus = [0, 1]
        let utils = vec![0.3, 0.7, 0.9, 0.1]; // cpu0=0.3, cpu1=0.7
        assert!((c.max_util(&utils) - 0.7).abs() < 1e-6);
        // 越界 cpu 安全跳过
        let short_utils = vec![0.5]; // 只有 cpu0
        assert!((c.max_util(&short_utils) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_on_load_update_high_load_rises_perf() {
        // 高负载 (util >= up_threshold=0.80) 应升频。
        // up_rate_limit_ticks=2：第 1 tick 计数未达，第 2 tick 才真正升。
        let mut gov = make_test_governor(0.30);
        let utils = vec![0.95, 0.95];
        gov.on_load_update(&utils); // up_wait=1，未达 rate limit，不变
        let perf_after_1 = gov.clusters[0].current_perf;
        gov.on_load_update(&utils); // up_wait=2，达 rate limit，高负载升频
        let perf_after_2 = gov.clusters[0].current_perf;
        assert!(
            perf_after_2 > 0.30,
            "高负载下 perf 应上升: 0.30 -> {} -> {}",
            perf_after_1,
            perf_after_2
        );
    }

    #[test]
    fn test_on_load_update_low_load_falls_perf() {
        // 极低负载触发 fast_down（util < down_fast_threshold=0.15），Race-to-Idle 降频
        let mut gov = make_test_governor(0.80);
        gov.on_load_update(&vec![0.05, 0.05]);
        let perf_after = gov.clusters[0].current_perf;
        assert!(
            perf_after < 0.80,
            "极低负载下 perf 应下降: 0.80 -> {}",
            perf_after
        );
    }

    #[test]
    fn test_on_load_update_mid_load_stable_in_hysteresis_band() {
        // 中等负载 0.65 处于滞回带 (down_threshold=0.50 ~ up_threshold=0.80)，
        // 预热 prev_util 避免首 tick 尖峰抑制干扰；perf 应稳定在 0.65 附近不漂移
        let mut gov = make_test_governor(0.65);
        gov.clusters[0].prev_util = 0.65;
        let utils = vec![0.65, 0.65];
        for _ in 0..10 {
            gov.on_load_update(&utils);
        }
        let perf = gov.clusters[0].current_perf;
        assert!(
            (perf - 0.65).abs() < 0.05,
            "滞回带内 perf 应稳定在 0.65 附近: perf={}",
            perf
        );
    }

    #[test]
    fn test_on_load_update_very_low_load_race_to_idle() {
        // 极低负载 (util < 0.15) 触发 Race-to-Idle 极速降频：
        // smoothing_down * down_fast_mult = 0.30 * 3.0 = 0.9，单 tick 即大幅下降
        let mut gov = make_test_governor(0.90);
        gov.on_load_update(&vec![0.02, 0.02]);
        let perf = gov.clusters[0].current_perf;
        assert!(perf < 0.60, "Race-to-Idle 应极速降频: 0.90 -> {}", perf);
    }

    #[test]
    fn test_on_load_update_inactive_is_noop() {
        // active=false 时 on_load_update 应立即返回，perf 不变
        let mut gov = make_test_governor(0.50);
        gov.active = false;
        let perf_before = gov.clusters[0].current_perf;
        gov.on_load_update(&vec![0.99, 0.99]);
        assert!(
            (gov.clusters[0].current_perf - perf_before).abs() < 1e-6,
            "未激活时 on_load_update 应是空操作"
        );
    }

    #[test]
    fn test_reload_config_updates_cfg() {
        let mut gov = make_test_governor(0.5);
        let mut new_cfg = CpuLoadGovernorConfig::default();
        new_cfg.up_threshold = 0.95;
        new_cfg.perf_ceil = 0.9;
        gov.reload_config(&new_cfg);
        assert!((gov.cfg.up_threshold - 0.95).abs() < 1e-6);
        assert!((gov.cfg.perf_ceil - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_release_clears_clusters_and_deactivates() {
        // pre_takeover 字段全为 0/空，release 不会触发任何 sysfs 写入，安全
        let mut gov = make_test_governor(0.5);
        assert!(gov.is_active());
        assert_eq!(gov.clusters.len(), 1);
        gov.release();
        assert!(!gov.is_active(), "release 后应失活");
        assert!(gov.clusters.is_empty(), "release 后 clusters 应清空");
    }

    #[test]
    fn test_new_governor_is_inactive_by_default() {
        let gov = CpuLoadGovernor::new();
        assert!(!gov.is_active());
        assert!(gov.clusters.is_empty());
    }
}
