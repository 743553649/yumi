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

use std::fs::{self, File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::mpsc::Sender;

use anyhow::Result;
use log::{info, warn};

use crate::fluent_args;
use crate::i18n::{t, t_with_args};
use crate::touch_boost::config::TouchBoostConfig;

const BTN_TOUCH: u16 = 0x14a;
const ABS_MT_TRACKING_ID: u16 = 0x39;
const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;

#[derive(Debug, Clone)]
pub enum TouchEvent {
    Start,
    End,
}

pub struct TouchMonitor {
    epoll_fd: RawFd,
    input_files: Vec<File>,
}

impl TouchMonitor {
    pub fn new(config: TouchBoostConfig) -> Result<Self> {
        let devices = if config.input_device.is_empty() {
            Self::find_touch_devices()?
        } else {
            vec![config.input_device.clone()]
        };

        if devices.is_empty() {
            return Err(anyhow::anyhow!("no touch input devices found"));
        }

        let epoll_fd = unsafe { libc::epoll_create1(0) };
        if epoll_fd < 0 {
            return Err(anyhow::anyhow!("epoll_create1 failed"));
        }

        let mut input_files = Vec::new();
        for dev in &devices {
            match OpenOptions::new().read(true).open(dev) {
                Ok(f) => {
                    let fd = f.as_raw_fd();
                    let mut ev = libc::epoll_event {
                        events: libc::EPOLLIN as u32,
                        u64: fd as u64,
                    };
                    let ret =
                        unsafe { libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut ev) };
                    if ret < 0 {
                        warn!(
                            "{}",
                            t_with_args(
                                "sysfs-open-failed",
                                &fluent_args!(
                                    "path" => dev.clone(), "error" => "epoll_ctl failed".to_string()
                                )
                            )
                        );
                        continue;
                    }
                    input_files.push(f);
                }
                Err(e) => {
                    warn!(
                        "{}",
                        t_with_args(
                            "sysfs-open-failed",
                            &fluent_args!(
                                "path" => dev.clone(), "error" => e.to_string()
                            )
                        )
                    );
                }
            }
        }

        if input_files.is_empty() {
            unsafe {
                libc::close(epoll_fd);
            }
            return Err(anyhow::anyhow!("no input devices could be opened"));
        }

        info!(
            "{}",
            t_with_args(
                "touch-boost-listener-started",
                &fluent_args!("count" => input_files.len().to_string())
            )
        );

        Ok(Self {
            epoll_fd,
            input_files,
        })
    }

    pub fn run(&self, tx: Sender<TouchEvent>) -> Result<()> {
        info!("{}", t("touch-boost-thread-started"));

        let mut events = [libc::epoll_event { events: 0, u64: 0 }; 16];
        let mut touching = false;

        loop {
            let n = unsafe {
                libc::epoll_wait(self.epoll_fd, events.as_mut_ptr(), events.len() as i32, -1)
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(anyhow::anyhow!("epoll_wait failed: {}", err));
            }

            for i in 0..n as usize {
                let fd = events[i].u64 as RawFd;
                let mut buf = [0u8; 256];
                let bytes =
                    unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
                if bytes < 0 {
                    continue;
                }

                let count = bytes as usize / std::mem::size_of::<InputEvent>();
                for j in 0..count {
                    let offset = j * std::mem::size_of::<InputEvent>();
                    let ev = unsafe {
                        std::ptr::read_unaligned(buf[offset..].as_ptr() as *const InputEvent)
                    };

                    if ev.type_ == EV_KEY && ev.code == BTN_TOUCH {
                        if ev.value > 0 && !touching {
                            touching = true;
                            let _ = tx.send(TouchEvent::Start);
                        } else if ev.value == 0 && touching {
                            touching = false;
                            let _ = tx.send(TouchEvent::End);
                        }
                    } else if ev.type_ == EV_ABS && ev.code == ABS_MT_TRACKING_ID {
                        if ev.value >= 0 && !touching {
                            touching = true;
                            let _ = tx.send(TouchEvent::Start);
                        } else if ev.value < 0 && touching {
                            touching = false;
                            let _ = tx.send(TouchEvent::End);
                        }
                    }
                }
            }
        }
    }

    fn find_touch_devices() -> Result<Vec<String>> {
        let mut devices = Vec::new();
        let base = "/dev/input";
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name.starts_with("event") {
                    devices.push(path.to_string_lossy().to_string());
                }
            }
        }
        devices.sort();
        Ok(devices)
    }
}

impl Drop for TouchMonitor {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.epoll_fd);
        }
    }
}

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}
