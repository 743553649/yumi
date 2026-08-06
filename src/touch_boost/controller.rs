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

// ════════════════════════════════════════════════════════════════
//  TouchBoostController — 状态机与频率控制
//  当用户触摸屏幕时，临时提升 CPU scaling_min_freq，确保触摸
//  操作的响应速度。松手后，频率逐步衰减恢复到正常调度状态。
// ════════════════════════════════════════════════════════════════

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::i18n::t;
use crate::scheduler::CpuPolicy;
use crate::utils::FastWriter;

use super::config::TouchBoostConfig;

/// TouchBoost 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoostState {
    /// 空闲，无触摸
    Idle,
    /// 触摸中，已 boost
    Touching,
    /// 松手后恢复中
    Recovering,
    /// 脉冲超时冷却中（触摸仍处于按下状态，但已释放锁频）
    Cooldown,
}

/// TouchBoost 控制器
pub struct TouchBoostController {
    /// 共享配置 (支持热重载)
    config: Arc<RwLock<TouchBoostConfig>>,
    /// 当前状态
    state: BoostState,
    /// 各集群的 policy ID (与 writers 索引对齐)
    policy_ids: Vec<i32>,
    /// 各集群的 min_freq 写入器 (按 policy 索引)
    min_freq_writers: Vec<Option<FastWriter>>,
    /// 各集群的原始 min_freq (恢复时使用)
    original_min_freqs: Vec<u32>,
    /// 各集群当前的 boost 频率
    current_boost_freqs: Vec<u32>,
    /// 触摸开始时间
    touch_start: Instant,
    /// 松手时间
    release_time: Instant,
    /// 是否已初始化
    initialized: bool,
    /// 上次观察到的 enabled 值，用于检测"启用 → 禁用"边沿并清理状态
    last_enabled: bool,
    /// 性能大核集群 Policy (如 Policy 0)
    perf_policy: Option<i32>,
    /// 超级大核集群 Policy (如 Policy 6)
    prime_policy: Option<i32>,
    /// FAS 模式静默标志：FAS 激活时 TouchBoost 自动停止提频，避免冲突
    fas_silenced: Option<Arc<AtomicBool>>,
    /// 上次观察到的 fas_silenced 值，用于检测 false→true 边沿
    last_fas_silenced: bool,
}

impl TouchBoostController {
    pub fn new(config: Arc<RwLock<TouchBoostConfig>>) -> Self {
        Self {
            config,
            state: BoostState::Idle,
            policy_ids: Vec::new(),
            min_freq_writers: Vec::new(),
            original_min_freqs: Vec::new(),
            current_boost_freqs: Vec::new(),
            touch_start: Instant::now(),
            release_time: Instant::now(),
            initialized: false,
            last_enabled: false,
            perf_policy: None,
            prime_policy: None,
            fas_silenced: None,
            last_fas_silenced: false,
        }
    }

    /// 设置 FAS 静默标志，由调度器线程在模式切换时传入。
    /// FAS 激活后 TouchBoost 自动停止提频，避免与 FAS 频率控制冲突。
    pub fn set_fas_silenced_flag(&mut self, flag: Arc<AtomicBool>) {
        self.fas_silenced = Some(flag.clone());
        self.last_fas_silenced = flag.load(Ordering::Relaxed);
    }

    /// 同步 FAS 静默状态并检测 false→true 边沿。
    /// 返回 true 表示 FAS 处于激活状态，TouchBoost 应静默。
    fn sync_fas_silenced(&mut self) -> bool {
        let Some(flag) = &self.fas_silenced else { return false };
        let silenced = flag.load(Ordering::Relaxed);
        if silenced && !self.last_fas_silenced {
            // false→true 边沿：FAS 刚激活，立即释放所有正在进行的 boost
            self.recover_all();
            self.state = BoostState::Idle;
        }
        self.last_fas_silenced = silenced;
        silenced
    }

    /// 识别 骁龙 8 Elite 架构 Cluster (Policy 0 性能大核, Policy 6 超级大核)
    pub fn setup_8_elite_clusters(&mut self, policies: &[CpuPolicy]) {
        for p in policies {
            if p.cpus.contains(&6) || p.cpus.contains(&7) || p.id == 6 {
                self.prime_policy = Some(p.id);
            }
            if p.id == 0 || p.cpus.iter().any(|&c| (0..=5).contains(&c)) {
                self.perf_policy = Some(p.id);
            }
        }
    }

    pub fn perf_policy_id(&self) -> Option<i32> {
        self.perf_policy
    }

    pub fn prime_policy_id(&self) -> Option<i32> {
        self.prime_policy
    }

    /// 初始化控制器：获取各集群的频率写入器
    pub fn init(&mut self, policies: &[CpuPolicy]) -> anyhow::Result<()> {
        self.setup_8_elite_clusters(policies);

        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        self.last_enabled = cfg.enabled;

        for policy in policies {
            let min_path = format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq",
                policy.id
            );
            if Path::new(&min_path).exists() {
                let mut writer = FastWriter::new(&min_path);
                if writer.is_valid() {
                    let current_min = Self::read_current_min_freq(policy.id);
                    // 首次写入验证
                    writer.write_value_force(current_min);
                    self.policy_ids.push(policy.id);
                    self.min_freq_writers.push(Some(writer));
                    self.original_min_freqs.push(current_min);
                    self.current_boost_freqs.push(0);
                    continue;
                }
            }
            // 节点不可用时占位
            self.policy_ids.push(policy.id);
            self.min_freq_writers.push(None);
            self.original_min_freqs.push(0);
            self.current_boost_freqs.push(0);
        }
        drop(cfg);

        self.initialized = true;
        log::info!("{}", t("touch-boost-init"));
        Ok(())
    }

    /// 同步 enabled 状态并处理"启用 → 禁用"边沿。
    /// 禁用边沿时清理残留的 boost 状态并恢复原始频率。
    fn sync_enabled(&mut self) -> bool {
        let enabled = self.config.read().unwrap_or_else(|e| e.into_inner()).enabled;
        if !enabled && self.last_enabled {
            // 禁用边沿：重置状态机并恢复原始频率
            self.state = BoostState::Idle;
            self.recover_all();
        }
        self.last_enabled = enabled;
        enabled
    }

    /// 处理触摸事件
    pub fn on_touch_event(&mut self, touching: bool) {
        if !self.initialized || !self.sync_enabled() || self.sync_fas_silenced() {
            return;
        }
        let now = Instant::now();
        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        let min_dur = cfg.min_boost_duration_ms.min(50);

        match (self.state, touching) {
            // IDLE → TOUCHING: 按下瞬间触发 50ms 脉冲
            (BoostState::Idle, true) => {
                drop(cfg);
                self.state = BoostState::Touching;
                self.touch_start = now;
                self.apply_boost();
                log::debug!("{}", t("touch-boost-start"));
            }
            // TOUCHING 状态下持续触摸：若超过 50ms 自动释放高频锁，进入 Cooldown 状态
            (BoostState::Touching, true) => {
                drop(cfg);
                if now.duration_since(self.touch_start).as_millis() as u64 >= min_dur {
                    self.state = BoostState::Cooldown;
                    self.recover_all();
                    log::debug!("{}", t("touch-boost-release"));
                }
            }
            // TOUCHING 状态松手：直接恢复原始频率
            (BoostState::Touching, false) => {
                drop(cfg);
                self.state = BoostState::Idle;
                self.recover_all();
                log::debug!("{}", t("touch-boost-release"));
            }
            // COOLDOWN 状态持续触摸：保持 Cooldown，不重新 Boost
            (BoostState::Cooldown, true) => {
                drop(cfg);
            }
            // COOLDOWN 状态松手：恢复到 Idle
            (BoostState::Cooldown, false) => {
                drop(cfg);
                self.state = BoostState::Idle;
                self.recover_all();
                log::debug!("{}", t("touch-boost-recovered"));
            }
            // RECOVERING 状态松手或恢复完成
            (BoostState::Recovering, false) => {
                let delay = cfg.release_delay_ms;
                drop(cfg);
                if now.duration_since(self.release_time).as_millis() as u64 >= delay {
                    self.state = BoostState::Idle;
                    self.recover_all();
                    log::debug!("{}", t("touch-boost-recovered"));
                }
            }
            // RECOVERING 状态再次触摸
            (BoostState::Recovering, true) => {
                drop(cfg);
                self.state = BoostState::Touching;
                self.touch_start = now;
                self.apply_boost();
                log::debug!("{}", t("touch-boost-reapply"));
            }
            _ => { drop(cfg); }
        }
    }

    /// 应用 boost 频率
    fn apply_boost(&mut self) {
        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        for (i, writer_opt) in self.min_freq_writers.iter_mut().enumerate() {
            if let Some(writer) = writer_opt {
                let policy_id = self.policy_ids[i];
                // 封印超大核：Prime Core Policy 保持最小频率，不响应日常 TouchBoost
                if Some(policy_id) == self.prime_policy {
                    let target = self.original_min_freqs[i];
                    writer.write_value_force(target);
                    self.current_boost_freqs[i] = 0;
                    continue;
                }

                let policy_id_idx = policy_id as usize;
                // boost_freqs 按 policy id 索引（如 policy_id=0 对应 boost_freqs[0]）
                // 如果 policy_id >= boost_freqs.len()，则跳过该集群的 boost
                if let Some(&boost_freq) = cfg.boost_freqs.get(policy_id_idx) {
                    if boost_freq > 0 {
                        writer.write_value_force(boost_freq);
                        self.current_boost_freqs[i] = boost_freq;
                    }
                }
            }
        }
    }

    /// 恢复所有集群到原始频率
    fn recover_all(&mut self) {
        for (i, writer_opt) in self.min_freq_writers.iter_mut().enumerate() {
            if let Some(writer) = writer_opt {
                let target = self.original_min_freqs[i];
                writer.write_value_force(target);
                self.current_boost_freqs[i] = 0;
            }
        }
    }

    /// 定时 tick：处理 50ms 脉冲超时和恢复阶段衰减
    pub fn tick(&mut self) {
        if !self.initialized || self.sync_fas_silenced() {
            return;
        }

        let now = Instant::now();

        // TOUCHING 状态按住静止（无 epoll 事件），超时 50ms 自动切入 Cooldown
        if self.state == BoostState::Touching {
            let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
            let min_dur = cfg.min_boost_duration_ms.min(50);
            drop(cfg);
            if now.duration_since(self.touch_start).as_millis() as u64 >= min_dur {
                self.state = BoostState::Cooldown;
                self.recover_all();
                log::debug!("{}", t("touch-boost-release"));
                return;
            }
        }

        if self.state != BoostState::Recovering {
            return;
        }

        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        let elapsed = now.duration_since(self.release_time).as_millis() as u64;

        if elapsed < cfg.release_delay_ms {
            return;
        }

        // 指数衰减：每次 tick 降低剩余距离的 decay 比例（如 0.15 = 15%）
        // 行为：频率逐渐接近目标值，但理论上永远不会完全恢复到 0
        // 依赖 current <= target 条件退出循环
        let decay = cfg.recover_decay;
        let mut all_recovered = true;

        for (i, writer_opt) in self.min_freq_writers.iter_mut().enumerate() {
            if let Some(writer) = writer_opt {
                let current = self.current_boost_freqs[i];
                let target = self.original_min_freqs[i];

                if current > target {
                    let step = ((current - target) as f32 * decay) as u32;
                    let new_freq = (current - step).max(target);
                    writer.write_value_force(new_freq);
                    self.current_boost_freqs[i] = new_freq;
                    if new_freq > target {
                        all_recovered = false;
                    }
                }
            }
        }
        drop(cfg);

        if all_recovered {
            self.state = BoostState::Idle;
            log::debug!("{}", t("touch-boost-recovered"));
        }
    }

    /// 当前状态
    pub fn state(&self) -> BoostState {
        self.state
    }

    /// 读取指定 policy 的当前 scaling_min_freq
    fn read_current_min_freq(policy_id: i32) -> u32 {
        let path = format!(
            "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq",
            policy_id
        );
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(300000)
    }
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证状态机：触摸按下 → 松手 → 恢复
    #[test]
    fn test_state_machine_touch_and_release() {
        let cfg = TouchBoostConfig {
            release_delay_ms: 0,
            min_boost_duration_ms: 0,
            ..Default::default()
        };
        let mut controller = TouchBoostController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;
        controller.last_enabled = true;

        // 初始状态: Idle
        assert_eq!(controller.state(), BoostState::Idle);

        // 触摸按下 → Touching
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching);

        // 松手 → Recovering (release_delay_ms=0 会立即恢复)
        controller.on_touch_event(false);
        // 由于 release_delay_ms=0，tick 后应恢复到 Idle
        controller.tick();
        assert_eq!(controller.state(), BoostState::Idle);
    }

    /// 验证恢复中再次触摸
    #[test]
    fn test_recovering_to_touching() {
        let cfg = TouchBoostConfig {
            release_delay_ms: 10000, // 很长的恢复延迟
            min_boost_duration_ms: 0,
            ..Default::default()
        };
        let mut controller = TouchBoostController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;
        controller.last_enabled = true;

        controller.on_touch_event(true);
        controller.on_touch_event(false);
        assert_eq!(controller.state(), BoostState::Recovering);

        // 恢复中再次触摸
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching);
    }

    /// 验证 disabled 配置下不动作
    #[test]
    fn test_disabled_no_action() {
        let cfg = TouchBoostConfig {
            enabled: false,
            ..Default::default()
        };
        let mut controller = TouchBoostController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;
        controller.last_enabled = false;

        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Idle);
    }

    /// 验证 50ms 脉冲 Boost 与 Cooldown 冷却状态机制
    #[test]
    fn test_pulse_boost_50ms_cooldown() {
        let cfg = Arc::new(RwLock::new(TouchBoostConfig {
            enabled: true,
            boost_freqs: vec![2000000],
            release_delay_ms: 50,
            min_boost_duration_ms: 50,
            ..Default::default()
        }));
        let mut controller = TouchBoostController::new(cfg);
        controller.initialized = true;
        controller.last_enabled = true;

        // 1. 按下触控: 应进入 Touching 状态
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching);

        // 2. 触摸持续超过 50ms 后再次收到 touching=true (滑动/按住):
        // 应自动结束 Boost，进入 Cooldown 状态，而非保持 Touching
        std::thread::sleep(std::time::Duration::from_millis(60));
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Cooldown);

        // 3. 在 Cooldown 状态下，持续收到 touching=true: 应保持 Cooldown 状态，不重新 Boost
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Cooldown);

        // 4. 只有当 touching=false (松手) 时: 才从 Cooldown 恢复到 Idle 状态
        controller.on_touch_event(false);
        assert_eq!(controller.state(), BoostState::Idle);
    }

    /// 验证 tick 在 50ms 超时后将 Touching 状态切入 Cooldown
    #[test]
    fn test_pulse_boost_tick_cooldown() {
        let cfg = Arc::new(RwLock::new(TouchBoostConfig {
            enabled: true,
            boost_freqs: vec![2000000],
            release_delay_ms: 50,
            min_boost_duration_ms: 50,
            ..Default::default()
        }));
        let mut controller = TouchBoostController::new(cfg);
        controller.initialized = true;
        controller.last_enabled = true;

        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching);

        // sleep 60ms 且没有新的 touch event，但 tick 被调用
        std::thread::sleep(std::time::Duration::from_millis(60));
        controller.tick();
        assert_eq!(controller.state(), BoostState::Cooldown);

        // 松手恢复 Idle
        controller.on_touch_event(false);
        assert_eq!(controller.state(), BoostState::Idle);
    }

    /// 验证 8 Elite 动态 Cluster 识别与 Policy 分组
    #[test]
    fn test_snapdragon_8_elite_cluster_classification() {
        let mut controller =
            TouchBoostController::new(Arc::new(RwLock::new(TouchBoostConfig::default())));
        controller.initialized = true;

        let policies = vec![
            CpuPolicy {
                id: 0,
                cpus: vec![0, 1, 2, 3, 4, 5],
                boost_frequencies: vec![],
            },
            CpuPolicy {
                id: 6,
                cpus: vec![6, 7],
                boost_frequencies: vec![],
            },
        ];
        controller.setup_8_elite_clusters(&policies);

        assert_eq!(controller.perf_policy_id(), Some(0));
        assert_eq!(controller.prime_policy_id(), Some(6));
    }

    /// 验证 50ms 脉冲边界：短暂触摸（<50ms）应保持 Touching，超时后自动切入 Cooldown
    /// 与 test_pulse_boost_50ms_cooldown 不同，此测试不依赖 sleep，
    /// 而是通过连续两次 on_touch_event(true) 验证 0ms 边界
    #[test]
    fn test_50ms_pulse_boundary_immediate_repeat() {
        let cfg = Arc::new(RwLock::new(TouchBoostConfig {
            enabled: true,
            boost_freqs: vec![2000000],
            release_delay_ms: 0,
            min_boost_duration_ms: 50,
            ..Default::default()
        }));
        let mut controller = TouchBoostController::new(cfg);
        controller.initialized = true;
        controller.last_enabled = true;

        // 1. 触摸按下 → Touching
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching,
            "触摸按下应进入 Touching");

        // 2. 立即再次收到 touching=true（<50ms 边界）
        // 此时未满 50ms，应保持 Touching，不能提前释放
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching,
            "50ms 脉冲未到期前应保持 Touching");

        // 3. 等待超过 50ms 后收到 touching=true → 应切入 Cooldown
        std::thread::sleep(std::time::Duration::from_millis(60));
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Cooldown,
            "50ms 脉冲到期后应自动切入 Cooldown");
    }

    /// 验证 TouchBoost 默认脉冲宽度为 50ms（阶段十二要求）
    /// 防止未来误修改 min_boost_duration_ms 默认值
    #[test]
    fn test_touch_boost_default_min_duration_is_50ms() {
        let cfg = TouchBoostConfig::default();
        assert_eq!(cfg.min_boost_duration_ms, 50,
            "min_boost_duration_ms 默认值应为 50ms（阶段十二要求）");
        assert_eq!(cfg.release_delay_ms, 100,
            "release_delay_ms 默认值应为 100ms");
        assert_eq!(cfg.recover_decay, 0.15,
            "recover_decay 默认值应为 0.15");
    }

    /// 验证 FAS 模式下 TouchBoost 自动静默
    #[test]
    fn test_fas_mode_silences_touch_boost() {
        let fas_silenced = Arc::new(AtomicBool::new(false));
        let cfg = Arc::new(RwLock::new(TouchBoostConfig {
            enabled: true,
            boost_freqs: vec![2000000],
            release_delay_ms: 0,
            min_boost_duration_ms: 0,
            ..Default::default()
        }));
        let mut controller = TouchBoostController::new(cfg);
        controller.initialized = true;
        controller.last_enabled = true;
        controller.set_fas_silenced_flag(fas_silenced.clone());

        // 1. FAS 未激活时触摸 → Touching 并应用 boost
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching);

        // 2. 激活 FAS → 边沿检测触发 recover_all，强制回到 Idle
        fas_silenced.store(true, Ordering::Relaxed);
        controller.on_touch_event(true); // sync_fas_silenced 检测边沿并恢复
        assert_eq!(controller.state(), BoostState::Idle,
            "FAS 激活后应强制回到 Idle");

        // 3. FAS 激活时触摸 → 被静默，状态保持 Idle
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Idle,
            "FAS 激活时触摸不应提频");

        // 4. 退出 FAS → 恢复正常提频
        fas_silenced.store(false, Ordering::Relaxed);
        controller.on_touch_event(true);
        assert_eq!(controller.state(), BoostState::Touching,
            "FAS 退出后应恢复提频");
    }
}
