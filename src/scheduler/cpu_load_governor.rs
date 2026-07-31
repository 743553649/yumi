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
use log::{info, debug, warn};
use std::fs;

use crate::i18n::{t, t_with_args};
use crate::fluent_args;

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
            *self.available_freqs.last().unwrap()
        } else {
            let lo = idx - 1;
            let hi = idx;
            if (self.cached_ratios[hi] - target_ratio).abs()
                < (self.cached_ratios[lo] - target_ratio).abs()
            { self.available_freqs[hi] } else { self.available_freqs[lo] }
        }
    }

    fn write_freq(&mut self, freq: u32) {
        if freq == self.current_freq { return; }
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
        self.affected_cpus.iter()
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
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_governor", pid);
            let _ = crate::utils::try_write_file(&gov_path, "performance");

            let freq_path = format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_available_frequencies", pid);
            let mut freqs: Vec<u32> = fs::read_to_string(&freq_path)
                .unwrap_or_default()
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if freqs.is_empty() { continue; }
            freqs.sort_unstable();
            freqs.dedup();

            // 合并 boost 频率（部分平台额外暴露的高频点），去重排序
            if !policy.boost_frequencies.is_empty() {
                freqs.extend(&policy.boost_frequencies);
                freqs.sort_unstable();
                freqs.dedup();
            }

            let affected = Self::read_affected_cpus(pid);
            if affected.is_empty() { continue; }

            let fmin = *freqs.first().unwrap() as f32;
            let fmax = *freqs.last().unwrap() as f32;
            let range = (fmax - fmin).max(1.0);
            let cached_ratios: Vec<f32> = freqs.iter()
                .map(|&f| (f as f32 - fmin) / range)
                .collect();

            let max_writer = FastWriter::new(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_max_freq", pid));
            let min_writer = FastWriter::new(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq", pid));

            if !max_writer.is_valid() || !min_writer.is_valid() {
                warn!("{}", t_with_args("clg-writer-invalid", &fluent_args!(
                    "pid" => pid.to_string(),
                    "max_valid" => max_writer.is_valid().to_string(),
                    "min_valid" => min_writer.is_valid().to_string()
                )));
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
                .unwrap_or_default().trim().to_string();
            let pre_min = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq", pid))
                .unwrap_or_default().trim().parse::<u32>().unwrap_or(0);
            let pre_max = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_max_freq", pid))
                .unwrap_or_default().trim().parse::<u32>().unwrap_or(0);

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

            info!("{}", t_with_args("clg-init", &fluent_args!(
                "pid" => pid.to_string(),
                "cpus" => format!("{:?}", affected),
                "fmin" => (fmin / 1000.0).to_string(),
                "fmax" => (fmax / 1000.0).to_string(),
                "perf" => format!("{:.2}", init_perf),
                "freq" => (init_freq / 1000).to_string()
            )));

            self.clusters.push(cluster);
        }

        self.active = !self.clusters.is_empty();
        if self.active {
            info!("{}", t_with_args("clg-activated", &fluent_args!("count" => self.clusters.len().to_string())));
        } else {
            warn!("{}", t("clg-no-clusters"));
        }
    }

    pub fn release(&mut self) {
        if self.active {
            // 恢复接管前的 governor 和频率
            for c in &self.clusters {
                let gov_path = format!(
                    "/sys/devices/system/cpu/cpufreq/policy{}/scaling_governor",
                    c.policy_id);
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
        if !self.active { return; }

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

            let target_perf = (util * hr_factor)
                .clamp(self.cfg.perf_floor, self.cfg.perf_ceil);

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
                // ── 降频路径 ──
                cluster.up_wait = 0;
                cluster.down_wait += 1;

                // 极低负载快速降频：跳过确认期
                let fast_down = util < self.cfg.down_fast_threshold;
                let can_down = fast_down
                    || cluster.down_wait >= self.cfg.down_rate_limit_ticks;

                if can_down && target_perf < old_perf {
                    let smooth = if fast_down {
                        // 极低负载：快速回落
                        self.cfg.smoothing_down * self.cfg.down_fast_mult
                    } else if util < self.cfg.down_threshold {
                        // 低于 down_threshold：正常降频
                        self.cfg.smoothing_down
                    } else {
                        // 滞回带内 (down_threshold ~ up_threshold)：
                        // 目标低于当前即可降频，按慢速回落
                        self.cfg.smoothing_down * self.cfg.slow_down_scale
                    };
                    cluster.current_perf += (target_perf - old_perf) * smooth;
                    if fast_down { cluster.down_wait = 0; }
                }
            }

            cluster.current_perf = cluster.current_perf.clamp(self.cfg.perf_floor, self.cfg.perf_ceil);
            let target_freq = cluster.find_nearest_freq(cluster.current_perf);
            cluster.write_freq(target_freq);
        }

        self.log_counter += 1;
        if self.log_counter % 25 == 0 {
            for c in &self.clusters {
                debug!("{}", t_with_args("clg-tick-log", &fluent_args!(
                    "pid" => c.policy_id.to_string(),
                    "util" => format!("{:.0}", c.max_util(core_utils) * 100.0),
                    "perf" => format!("{:.2}", c.current_perf),
                    "freq" => (c.current_freq / 1000).to_string(),
                    "boost" => format!("{:.0}", c.available_freqs.last().copied().unwrap_or(0) as f32 / 1000.0)
                )));
            }
        }
    }

    fn read_affected_cpus(policy_id: i32) -> Vec<usize> {
        let path = format!(
            "/sys/devices/system/cpu/cpufreq/policy{}/affected_cpus", policy_id);
        fs::read_to_string(&path)
            .unwrap_or_default()
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .collect()
    }
}