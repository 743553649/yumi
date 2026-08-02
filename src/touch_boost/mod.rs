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
//  TouchBoost — 触摸提频
//
//  当用户触摸屏幕时，临时提升 CPU scaling_min_freq，确保触摸
//  操作的响应速度。松手后，频率逐步衰减恢复到正常调度状态。
//  通过 epoll 直接监听 /dev/input/event* 设备，比 Android 框架
//  层更快地捕获触摸事件。
//
//  实现方案见 docs/TouchBoost实现方案.md。
// ════════════════════════════════════════════════════════════════

use std::fs;
use std::os::unix::io::RawFd;
use std::path::Path;
use std::sync::{Arc, RwLock, mpsc};
use std::time::Instant;

use serde::Deserialize;

use crate::i18n::{t, t_with_args};
use crate::fluent_args;
use crate::scheduler::CpuPolicy;
use crate::utils::FastWriter;

// ════════════════════════════════════════════════════════════════
//  配置结构 (对应 config/touch_boost.yaml)
// ════════════════════════════════════════════════════════════════

/// TouchBoost 配置
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct TouchBoostConfig {
    /// 是否启用 TouchBoost
    pub enabled: bool,
    /// 各集群的 boost 目标频率 (kHz)，按 policy id 索引
    /// 例如: [2500000, 0, 2000000] 表示 Policy 0 → 2.5GHz，Policy 2 → 2.0GHz
    pub boost_freqs: Vec<u32>,
    /// 松手后恢复延迟 (ms)，防止快速点击时频繁切换
    pub release_delay_ms: u64,
    /// 恢复阶段的衰减步长 (每次 tick 降低当前 boost 频率的比例)
    pub recover_decay: f32,
    /// 最小 boost 持续时间 (ms)，防止误触
    pub min_boost_duration_ms: u64,
    /// 触摸设备路径，留空则自动检测
    pub input_device: String,
}

impl Default for TouchBoostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            boost_freqs: vec![2500000, 0, 2000000],
            release_delay_ms: 100,
            recover_decay: 0.15,
            min_boost_duration_ms: 50,
            input_device: String::new(),
        }
    }
}

// ════════════════════════════════════════════════════════════════
//  TouchBoostController — 状态机与频率控制
// ════════════════════════════════════════════════════════════════

/// TouchBoost 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoostState {
    /// 空闲，无触摸
    Idle,
    /// 触摸中，已 boost
    Touching,
    /// 松手后恢复中
    Recovering,
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
        }
    }

    /// 初始化控制器：获取各集群的频率写入器
    pub fn init(&mut self, policies: &[CpuPolicy]) -> anyhow::Result<()> {
        let cfg = self.config.read().unwrap();
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
        let enabled = self.config.read().unwrap().enabled;
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
        if !self.initialized || !self.sync_enabled() {
            return;
        }
        let now = Instant::now();
        let cfg = self.config.read().unwrap();

        match (self.state, touching) {
            // IDLE → TOUCHING: 触摸按下
            (BoostState::Idle, true) => {
                drop(cfg);
                self.state = BoostState::Touching;
                self.touch_start = now;
                self.apply_boost();
                log::debug!("{}", t("touch-boost-start"));
            }
            // TOUCHING → RECOVERING: 松手
            (BoostState::Touching, false) => {
                let min_dur = cfg.min_boost_duration_ms;
                drop(cfg);
                if now.duration_since(self.touch_start).as_millis() as u64 >= min_dur {
                    self.state = BoostState::Recovering;
                    self.release_time = now;
                    log::debug!("{}", t("touch-boost-release"));
                } else {
                    // 触摸时间过短（误触），直接恢复
                    self.state = BoostState::Idle;
                    self.recover_all();
                }
            }
            // RECOVERING → IDLE: 恢复完成
            (BoostState::Recovering, false) => {
                let delay = cfg.release_delay_ms;
                drop(cfg);
                if now.duration_since(self.release_time).as_millis() as u64 >= delay {
                    self.state = BoostState::Idle;
                    self.recover_all();
                    log::debug!("{}", t("touch-boost-recovered"));
                }
            }
            // RECOVERING → TOUCHING: 恢复中再次触摸
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
        let cfg = self.config.read().unwrap();
        for (i, writer_opt) in self.min_freq_writers.iter_mut().enumerate() {
            if let Some(writer) = writer_opt {
                let policy_id = self.policy_ids[i] as usize;
                // boost_freqs 按 policy id 索引（如 policy_id=0 对应 boost_freqs[0]）
                // 如果 policy_id >= boost_freqs.len()，则跳过该集群的 boost
                if let Some(&boost_freq) = cfg.boost_freqs.get(policy_id) {
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

    /// 定时 tick：处理恢复阶段的衰减
    pub fn tick(&mut self) {
        if !self.initialized || self.state != BoostState::Recovering {
            return;
        }

        let now = Instant::now();
        let cfg = self.config.read().unwrap();
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
//  TouchListener — epoll 触摸事件监听器
// ════════════════════════════════════════════════════════════════

/// Linux input_event 结构 (arm64)
/// 注意：硬编码 arm64 布局，tv_sec 和 tv_usec 都是 i64
/// 在 32 位平台上会错位，但项目只针对 aarch64-android，影响为零
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct InputEvent {
    tv_sec: i64,
    tv_usec: i64,
    ev_type: u16,
    ev_code: u16,
    ev_value: i32,
}

const EV_ABS: u16 = 3;
const ABS_MT_TRACKING_ID: u16 = 57;
const EPOLLIN: u32 = 0x001;
const EPOLL_CTL_ADD: i32 = 1;

/// 触摸事件监听器
pub struct TouchListener {
    epoll_fd: RawFd,
    device_fds: Vec<RawFd>,
}

impl TouchListener {
    /// 创建新的监听器，自动检测触摸设备或使用配置的设备路径
    pub fn new(config: &TouchBoostConfig) -> anyhow::Result<Self> {
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if epoll_fd < 0 {
            anyhow::bail!("{}", t("touch-boost-epoll-failed"));
        }
        let mut device_fds = Vec::new();

        let devices = if !config.input_device.is_empty() {
            vec![config.input_device.clone()]
        } else {
            Self::detect_touch_devices()?
        };

        if devices.is_empty() {
            unsafe { libc::close(epoll_fd); }
            anyhow::bail!("{}", t("touch-boost-no-device"));
        }

        for device_path in &devices {
            let fd = Self::open_device(device_path)?;
            device_fds.push(fd);

            let mut event = libc::epoll_event { events: EPOLLIN, u64: fd as u64 };
            let ret = unsafe {
                libc::epoll_ctl(epoll_fd, EPOLL_CTL_ADD, fd, &mut event)
            };
            if ret < 0 {
                unsafe { libc::close(fd); }
                device_fds.pop();
                continue;
            }
        }

        if device_fds.is_empty() {
            unsafe { libc::close(epoll_fd); }
            anyhow::bail!("{}", t("touch-boost-no-device"));
        }

        log::info!("{}", t_with_args("touch-boost-listener-started",
            &fluent_args!("count" => device_fds.len().to_string())));
        Ok(Self { epoll_fd, device_fds })
    }

    /// 打开输入设备
    fn open_device(path: &str) -> anyhow::Result<RawFd> {
        use std::ffi::CString;
        let c_path = CString::new(path)?;
        let fd = unsafe {
            libc::open(c_path.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC)
        };
        if fd < 0 {
            anyhow::bail!("open {} failed", path);
        }
        Ok(fd)
    }

    /// 检测系统中的触摸设备
    fn detect_touch_devices() -> anyhow::Result<Vec<String>> {
        let input_dir = Path::new("/sys/class/input");
        if !input_dir.exists() {
            anyhow::bail!("{}", t("touch-boost-no-device"));
        }

        let mut devices = Vec::new();
        let entries = match fs::read_dir(input_dir) {
            Ok(e) => e,
            Err(_) => anyhow::bail!("{}", t("touch-boost-no-device")),
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with("event") {
                continue;
            }

            let base = entry.path();
            // 检查 ABS_MT_TRACKING_ID 能力位 (bit 57)
            let abs_path = base.join("capabilities").join("abs");
            if let Ok(abs_str) = fs::read_to_string(&abs_path) {
                if let Some(hex_str) = abs_str.trim().split_whitespace().last() {
                    if let Ok(abs_mask) = u64::from_str_radix(hex_str, 16) {
                        if abs_mask & (1u64 << (ABS_MT_TRACKING_ID as u64 % 64)) != 0 {
                            let dev_path = format!("/dev/input/{}", name_str);
                            devices.push(dev_path);
                        }
                    }
                }
            }
        }

        if devices.is_empty() {
            anyhow::bail!("{}", t("touch-boost-no-device"));
        }
        Ok(devices)
    }

    /// 轮询触摸事件，返回 (touching, 是否有变化)
    /// timeout_ms: 超时时间，-1 表示阻塞等待
    pub fn poll(&self, timeout_ms: i32) -> Option<bool> {
        let mut events = [libc::epoll_event { events: 0, u64: 0 }; 4];
        let n = unsafe {
            libc::epoll_wait(self.epoll_fd, events.as_mut_ptr(), 4, timeout_ms)
        };

        if n <= 0 {
            return None;
        }

        for event in &events[..n as usize] {
            let fd = event.u64 as RawFd;
            match self.read_touch_state(fd) {
                Some(touching) => return Some(touching),
                None => continue,
            }
        }
        None
    }

    /// 从 fd 读取所有待处理事件，返回最新的触摸状态
    fn read_touch_state(&self, fd: RawFd) -> Option<bool> {
        let mut touching = false;
        let mut got_event = false;

        loop {
            let mut event = InputEvent::default();
            let size = std::mem::size_of::<InputEvent>();
            let ret = unsafe {
                libc::read(
                    fd,
                    &mut event as *mut InputEvent as *mut libc::c_void,
                    size,
                )
            };

            if ret == -1 {
                let errno = std::io::Error::last_os_error().raw_os_error();
                if errno == Some(libc::EAGAIN) {
                    break;
                }
                return None;
            }
            if ret != size as isize {
                break;
            }

            got_event = true;
            if event.ev_type == EV_ABS && event.ev_code == ABS_MT_TRACKING_ID {
                touching = event.ev_value != -1;
            }
        }

        if got_event { Some(touching) } else { None }
    }
}

impl Drop for TouchListener {
    fn drop(&mut self) {
        for &fd in &self.device_fds {
            unsafe { libc::close(fd); }
        }
        unsafe { libc::close(self.epoll_fd); }
    }
}

// ════════════════════════════════════════════════════════════════
//  线程启动函数
// ════════════════════════════════════════════════════════════════

/// 启动 TouchBoost 监听线程
pub fn start_touch_listener_thread(
    config: Arc<RwLock<TouchBoostConfig>>,
    policies: Vec<CpuPolicy>,
    touch_tx: mpsc::Sender<bool>,
) {
    std::thread::Builder::new()
        .name("touch_boost".to_string())
        .spawn(move || {
            // 创建监听器需要在子线程中完成（epoll 阻塞）
            let cfg_snapshot = config.read().unwrap().clone();
            let listener = match TouchListener::new(&cfg_snapshot) {
                Ok(l) => l,
                Err(e) => {
                    log::error!("{}", t_with_args("touch-boost-init-failed",
                        &fluent_args!("error" => e.to_string())));
                    return;
                }
            };

            let mut controller = TouchBoostController::new(config);
            if let Err(e) = controller.init(&policies) {
                log::error!("{}", t_with_args("touch-boost-init-failed",
                    &fluent_args!("error" => e.to_string())));
                return;
            }

            log::info!("{}", t("touch-boost-thread-started"));

            loop {
                // epoll_wait 超时 100ms，用于处理恢复衰减
                match listener.poll(100) {
                    Some(touching) => {
                        controller.on_touch_event(touching);
                        // 通知调度器触摸状态（用于 FAS/CLG 联动）
                        let _ = touch_tx.send(touching);
                    }
                    None => {
                        // 超时或无事件，执行 tick 处理恢复衰减
                        controller.tick();
                    }
                }
            }
        })
        .ok();
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证配置解析：扁平顶层结构
    #[test]
    fn test_config_parses_flat_top_level() {
        let yaml = r#"
enabled: true
boost_freqs:
  - 2500000
  - 0
  - 2000000
release_delay_ms: 100
recover_decay: 0.15
min_boost_duration_ms: 50
input_device: ""
"#;
        let cfg: TouchBoostConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(cfg.enabled);
        assert_eq!(cfg.boost_freqs.len(), 3);
        assert_eq!(cfg.boost_freqs[0], 2500000);
        assert_eq!(cfg.boost_freqs[2], 2000000);
        assert_eq!(cfg.release_delay_ms, 100);
    }

    /// 验证配置缺失字段时使用默认值
    #[test]
    fn test_config_defaults_for_missing_fields() {
        let yaml = "enabled: false\n";
        let cfg: TouchBoostConfig = serde_yaml::from_str(yaml).expect("解析失败");
        assert!(!cfg.enabled);
        assert_eq!(cfg.boost_freqs, vec![2500000, 0, 2000000]);
        assert_eq!(cfg.release_delay_ms, 100);
        assert_eq!(cfg.recover_decay, 0.15);
    }

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
}
