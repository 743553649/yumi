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
//  IdleDiveController — 主控制器
//  CPU 静止下潜 (Idle Dive)：在 CPU 平均负载极低时，通过切换 cpuidle
//  governor 并放宽 latency_us 上限，允许 CPU 进入更深的空闲状态，
//  降低待机功耗。实现方案见 docs/TouchBoost实现方案.md (CPU 静止下潜部分)。
// ════════════════════════════════════════════════════════════════

use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::i18n::t;
use crate::utils::FastWriter;

use super::config::IdleDiveConfig;
use super::latency::LatencyWriter;

/// 下潜状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiveState {
    /// 正常状态
    Normal,
    /// 下潜中 (低负载下潜)
    Diving,
    /// 息屏下潜
    DozeDiving,
}

/// CPU 静止下潜控制器
pub struct IdleDiveController {
    /// 共享配置 (支持热重载，config watcher 线程会更新)
    config: Arc<RwLock<IdleDiveConfig>>,
    /// 当前状态
    state: DiveState,
    /// 低负载开始时间
    low_load_start: Option<Instant>,
    /// 高负载开始时间
    high_load_start: Option<Instant>,
    /// cpuidle governor 写入器
    governor_writer: Option<FastWriter>,
    /// cpuidle latency 写入器 (支持 PM-QoS 降级)
    latency_writer: Option<LatencyWriter>,
    /// 是否已初始化
    initialized: bool,
    /// 上次观察到的 enabled 值，用于检测"启用 → 禁用"边沿并清理状态
    last_enabled: bool,
}

impl IdleDiveController {
    /// 创建新的控制器
    pub fn new(config: Arc<RwLock<IdleDiveConfig>>) -> Self {
        Self {
            config,
            state: DiveState::Normal,
            low_load_start: None,
            high_load_start: None,
            governor_writer: None,
            latency_writer: None,
            initialized: false,
            last_enabled: false,
        }
    }

    /// 初始化：探测 cpuidle 节点；仅当启用时应用正常状态配置
    pub fn init(&mut self) -> anyhow::Result<()> {
        let governor_path = "/sys/devices/system/cpu/cpuidle/current_governor";
        let latency_path = "/sys/devices/system/cpu/cpuidle/latency_us";

        // 探测 governor 节点是否存在
        if !Path::new(governor_path).exists() {
            anyhow::bail!("{}", t("idle-dive-unavailable"));
        }

        let mut governor_writer = FastWriter::new(governor_path);
        let mut latency_writer = LatencyWriter::new(latency_path);
        if !governor_writer.is_valid() || !latency_writer.is_valid() {
            anyhow::bail!("{}", t("idle-dive-unavailable"));
        }

        // 未启用时不写入任何节点，保持系统原始 cpuidle 状态
        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        self.last_enabled = cfg.enabled;
        if cfg.enabled {
            governor_writer.write_value_force_str(&cfg.governors.normal);
            latency_writer.write_latency(cfg.params.normal_latency_us);
        }
        drop(cfg);

        self.governor_writer = Some(governor_writer);
        self.latency_writer = Some(latency_writer);
        self.initialized = true;
        log::info!("{}", t("idle-dive-init"));
        Ok(())
    }

    /// 触摸快速退出：如果在下潜状态 (DiveState::Diving)，立即触发退出下潜，在 1ms 内恢复 Normal 状态
    pub fn on_touch_fast_exit(&mut self) {
        if self.state == DiveState::Diving {
            self.exit_dive();
        }
    }

    /// 同步 enabled 状态并处理"启用 → 禁用"边沿。
    /// 禁用边沿时清理残留的下潜状态并恢复 normal 配置，
    /// 避免关闭功能后系统仍停留在 Diving/DozeDiving 的 sysfs 状态。
    /// 返回当前是否应继续执行 (enabled)。
    fn sync_enabled(&mut self) -> bool {
        let enabled = self
            .config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .enabled;
        if !enabled && self.last_enabled {
            // 禁用边沿：重置状态机并恢复系统正常状态
            self.state = DiveState::Normal;
            self.low_load_start = None;
            self.high_load_start = None;
            self.apply_normal_config();
        }
        self.last_enabled = enabled;
        enabled
    }

    /// 更新状态，由调度器在系统负载事件时调用
    pub fn update(&mut self, avg_util: f32) {
        if !self.initialized || !self.sync_enabled() {
            return;
        }
        let now = Instant::now();
        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());

        match self.state {
            DiveState::Normal => {
                if avg_util < cfg.dive_threshold {
                    // 低负载，开始计时
                    if self.low_load_start.is_none() {
                        self.low_load_start = Some(now);
                        self.high_load_start = None;
                    }
                    if let Some(start) = self.low_load_start
                        && now.duration_since(start).as_millis() as u64 >= cfg.dive_delay_ms
                    {
                        drop(cfg);
                        self.enter_dive();
                    }
                } else {
                    // 负载恢复，重置计时
                    self.low_load_start = None;
                }
            }
            DiveState::Diving => {
                if avg_util > cfg.exit_threshold {
                    // 高负载，开始计时
                    if self.high_load_start.is_none() {
                        self.high_load_start = Some(now);
                        self.low_load_start = None;
                    }
                    if let Some(start) = self.high_load_start
                        && now.duration_since(start).as_millis() as u64 >= cfg.exit_delay_ms
                    {
                        drop(cfg);
                        self.exit_dive();
                    }
                } else {
                    // 负载降低，重置计时
                    self.high_load_start = None;
                }
            }
            DiveState::DozeDiving => {
                // 息屏下潜状态，只在亮屏时退出
            }
        }
    }

    /// 进入下潜状态
    pub(crate) fn enter_dive(&mut self) {
        if self.state == DiveState::Diving {
            return;
        }
        self.state = DiveState::Diving;
        self.low_load_start = None;

        // 切换到下潜 governor
        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = &mut self.governor_writer {
            w.write_value_force_str(&cfg.governors.diving);
        }
        if let Some(w) = &mut self.latency_writer {
            w.write_latency(cfg.params.diving_latency_us);
        }
        drop(cfg);

        log::debug!("{}", t("idle-dive-enter"));
    }

    /// 退出下潜状态
    fn exit_dive(&mut self) {
        if self.state != DiveState::Diving {
            return;
        }
        self.state = DiveState::Normal;
        self.high_load_start = None;
        self.apply_normal_config();
        log::debug!("{}", t("idle-dive-exit"));
    }

    /// 进入息屏下潜
    pub fn enter_doze(&mut self) {
        if !self.initialized || !self.sync_enabled() {
            return;
        }
        if self.state == DiveState::DozeDiving {
            return;
        }
        self.state = DiveState::DozeDiving;
        self.low_load_start = None;
        self.high_load_start = None;

        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = &mut self.governor_writer {
            w.write_value_force_str(&cfg.governors.doze);
        }
        if let Some(w) = &mut self.latency_writer {
            w.write_latency(cfg.params.doze_latency_us);
        }
        drop(cfg);

        log::debug!("{}", t("idle-dive-enter-dozed"));
    }

    /// 退出息屏下潜
    pub fn exit_doze(&mut self) {
        if !self.initialized {
            return;
        }
        // 无论 enabled 与否都恢复状态，避免禁用后 DozeDiving 永久卡死；
        // 写入仅按当前 enabled 门控（禁用时保持系统原始状态）
        self.sync_enabled();
        self.state = DiveState::Normal;
        self.low_load_start = None;
        self.high_load_start = None;
        if self.last_enabled {
            self.apply_normal_config();
        }
        log::debug!("{}", t("idle-dive-exit-dozed"));
    }

    /// 应用正常配置
    fn apply_normal_config(&mut self) {
        let cfg = self.config.read().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = &mut self.governor_writer {
            w.write_value_force_str(&cfg.governors.normal);
        }
        if let Some(w) = &mut self.latency_writer {
            w.write_latency(cfg.params.normal_latency_us);
        }
    }

    /// 获取当前状态
    pub fn state(&self) -> DiveState {
        self.state
    }

    /// 是否可用 (节点探测成功)
    pub fn is_available(&self) -> bool {
        self.initialized
    }
}

impl Drop for IdleDiveController {
    /// 优雅退出时恢复 normal 配置，避免残留放宽的 idle 延迟。
    /// 仅当控制器处于启用状态（可能写过 sysfs）时执行。
    fn drop(&mut self) {
        if self.initialized && self.last_enabled {
            self.apply_normal_config();
        }
    }
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证状态机：低负载触发下潜，高负载退出下潜
    #[test]
    fn test_state_machine_dive_and_exit() {
        let mut cfg = IdleDiveConfig::default();
        cfg.enabled = true;
        cfg.dive_threshold = 0.5;
        cfg.exit_threshold = 0.8;
        cfg.dive_delay_ms = 1;
        cfg.exit_delay_ms = 1;

        let mut controller = IdleDiveController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true; // 绕过 sysfs 探测（测试环境无 Android 节点）

        // 初始为正常状态
        assert_eq!(controller.state(), DiveState::Normal);

        // 低负载持续足够时间 → 进入下潜
        controller.update(0.1); // 开始计时
        std::thread::sleep(std::time::Duration::from_millis(5));
        controller.update(0.1); // 达到下潜延迟
        assert_eq!(controller.state(), DiveState::Diving);

        // 负载中途回落 → 重置计时，仍保持下潜
        controller.update(0.2);
        controller.update(0.2);
        assert_eq!(controller.state(), DiveState::Diving);

        // 高负载持续足够时间 → 退出下潜
        controller.update(0.9); // 开始计时
        std::thread::sleep(std::time::Duration::from_millis(5));
        controller.update(0.9); // 达到退出延迟
        assert_eq!(controller.state(), DiveState::Normal);
    }

    /// 验证息屏下潜：进入后负载变化不退出，亮屏后恢复正常
    #[test]
    fn test_state_machine_doze_diving() {
        let mut cfg = IdleDiveConfig::default();
        cfg.enabled = true;

        let mut controller = IdleDiveController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;

        controller.enter_doze();
        assert_eq!(controller.state(), DiveState::DozeDiving);

        // 息屏期间即使高负载也不退出
        controller.update(0.95);
        controller.update(0.95);
        assert_eq!(controller.state(), DiveState::DozeDiving);

        // 亮屏后恢复正常
        controller.exit_doze();
        assert_eq!(controller.state(), DiveState::Normal);
    }

    /// 验证 disabled 配置下不动作
    #[test]
    fn test_disabled_no_action() {
        let mut cfg = IdleDiveConfig::default();
        cfg.enabled = false;

        let mut controller = IdleDiveController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;

        controller.enter_doze();
        assert_eq!(controller.state(), DiveState::Normal);

        controller.update(0.1);
        assert_eq!(controller.state(), DiveState::Normal);
    }

    /// 回归测试：下潜 (Diving) 中热重载禁用配置，状态机必须清理并恢复 Normal，
    /// 不能停留在 Diving 导致 sysfs 残留放宽的 latency
    #[test]
    fn test_disable_while_diving_recovers_normal() {
        let mut cfg = IdleDiveConfig::default();
        cfg.enabled = true;
        cfg.dive_threshold = 0.5;
        cfg.exit_threshold = 0.8;
        cfg.dive_delay_ms = 1;

        let mut controller = IdleDiveController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;
        controller.last_enabled = true; // 模拟 init 时 enabled=true

        // 低负载持续足够时间 → 进入下潜
        controller.update(0.1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        controller.update(0.1);
        assert_eq!(controller.state(), DiveState::Diving);

        // 热重载禁用配置
        controller.config.write().unwrap().enabled = false;

        // 下一次 update 触发禁用边沿清理 → 恢复 Normal
        controller.update(0.1);
        assert_eq!(controller.state(), DiveState::Normal);
    }

    /// 回归测试：息屏下潜 (DozeDiving) 中禁用配置，亮屏退出时状态必须恢复，
    /// 不能永久卡死在 DozeDiving
    #[test]
    fn test_disable_while_doze_diving_exits_on_screen_on() {
        let mut cfg = IdleDiveConfig::default();
        cfg.enabled = true;

        let mut controller = IdleDiveController::new(Arc::new(RwLock::new(cfg)));
        controller.initialized = true;
        controller.last_enabled = true;

        controller.enter_doze();
        assert_eq!(controller.state(), DiveState::DozeDiving);

        // 息屏期间热重载禁用配置
        controller.config.write().unwrap().enabled = false;

        // 亮屏退出 → 状态必须恢复正常（写入被门控跳过，状态不被卡死）
        controller.exit_doze();
        assert_eq!(controller.state(), DiveState::Normal);
    }

    /// 验证 1ms 触摸快速退出：在 Diving 状态下触发 on_touch_fast_exit 即刻恢复 Normal
    #[test]
    fn test_idle_dive_1ms_fast_exit() {
        let cfg = Arc::new(RwLock::new(IdleDiveConfig::default()));
        let mut controller = IdleDiveController::new(cfg);
        controller.initialized = true;
        controller.enter_dive();
        assert_eq!(controller.state(), DiveState::Diving);

        // 触发触摸信号，必须 1ms 内退出下潜并恢复 Normal
        controller.on_touch_fast_exit();
        assert_eq!(controller.state(), DiveState::Normal);
    }

    /// 验证默认配置的 300ms 下潜延迟真正生效
    /// 阶段十二将延迟从 2000ms 压缩至 300ms，确保状态机实际使用该值
    #[test]
    fn test_dive_delay_300ms_uses_default_config() {
        // 使用默认配置（dive_delay_ms = 300）
        let cfg = Arc::new(RwLock::new(IdleDiveConfig::default()));
        let mut controller = IdleDiveController::new(cfg);
        controller.initialized = true;

        // 初始为 Normal
        assert_eq!(controller.state(), DiveState::Normal);

        // 喂入低负载，开始计时
        controller.update(0.05);

        // 计时未达到 300ms，应保持 Normal
        assert_eq!(
            controller.state(),
            DiveState::Normal,
            "300ms 延迟未到期不应进入下潜"
        );

        // 等待 310ms（略超 300ms 边界），再次喂入低负载触发计时检查
        std::thread::sleep(std::time::Duration::from_millis(310));
        controller.update(0.05);

        // 应进入 Diving 状态
        assert_eq!(
            controller.state(),
            DiveState::Diving,
            "默认 300ms 下潜延迟应生效，超时后进入下潜"
        );
    }

    /// 验证 IdleDive 配置中关键省电参数与阶段十二一致
    /// 防止未来误修改默认值
    #[test]
    fn test_idle_dive_key_params_match_stage12() {
        let cfg = IdleDiveConfig::default();

        // 阶段十二：亮屏下潜延迟从 2000ms 压缩至 300ms
        assert_eq!(
            cfg.dive_delay_ms, 300,
            "dive_delay_ms 应为 300ms（阶段十二要求）"
        );

        // 阶段十二：唤醒延迟从 500ms 降至 50ms
        assert_eq!(
            cfg.exit_delay_ms, 50,
            "exit_delay_ms 应为 50ms（阶段十二要求）"
        );

        // 阶段十二：下潜后放宽 latency 至 500μs，息屏 1000μs
        assert_eq!(
            cfg.params.diving_latency_us, 500,
            "diving_latency_us 应为 500μs（阶段十二要求）"
        );
        assert_eq!(
            cfg.params.doze_latency_us, 1000,
            "doze_latency_us 应为 1000μs（阶段十二要求）"
        );
    }
}
