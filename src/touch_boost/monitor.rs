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
//  TouchListener — epoll 触摸事件监听器与监听线程
//  通过 epoll 直接监听 /dev/input/event* 设备，比 Android 框架
//  层更快地捕获触摸事件。实现方案见 docs/TouchBoost实现方案.md。
// ════════════════════════════════════════════════════════════════

use std::fs;
use std::os::unix::io::RawFd;
use std::path::Path;
use std::sync::{Arc, RwLock, mpsc};

use crate::fluent_args;
use crate::i18n::{t, t_with_args};
use crate::scheduler::CpuPolicy;

use super::config::TouchBoostConfig;
use super::controller::TouchBoostController;

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
            let cfg_snapshot = config.read().unwrap_or_else(|e| e.into_inner()).clone();
            let listener = match TouchListener::new(&cfg_snapshot) {
                Ok(l) => l,
                Err(e) => {
                    log::warn!("{}", t_with_args("touch-boost-init-failed",
                        &fluent_args!("error" => e.to_string())));
                    // 优雅降级：当设备不支持 TouchBoost 监听或初始化失败时，保持线程存活（持有 touch_tx），避免通道断开
                    loop {
                        std::thread::park();
                    }
                }
            };

            let mut controller = TouchBoostController::new(config);
            if let Err(e) = controller.init(&policies) {
                log::warn!("{}", t_with_args("touch-boost-init-failed",
                    &fluent_args!("error" => e.to_string())));
                // 优雅降级：初始化失败时保持线程存活（持有 touch_tx），避免通道断开
                loop {
                    std::thread::park();
                }
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
