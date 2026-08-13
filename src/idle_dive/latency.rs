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

use std::fs;
use std::os::unix::io::RawFd;
use anyhow::Result;
use log::warn;

use crate::i18n::t_with_args;
use crate::fluent_args;

pub struct LatencyWriter {
    pm_qos_fd: Option<RawFd>,
    governor_paths: Vec<String>,
    latency_paths: Vec<String>,
}

impl LatencyWriter {
    pub fn new() -> Result<Self> {
        let pm_qos_fd = Self::open_pm_qos();

        let mut governor_paths = Vec::new();
        let mut latency_paths = Vec::new();

        if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu/cpuidle") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("policy") {
                        let base = entry.path();
                        let gov_path = base.join("current_governor_ro");
                        if gov_path.exists() {
                            governor_paths.push(gov_path.to_string_lossy().to_string());
                        } else {
                            let gov_path_rw = base.join("current_governor");
                            if gov_path_rw.exists() {
                                governor_paths.push(gov_path_rw.to_string_lossy().to_string());
                            }
                        }
                        let lat_path = base.join("current_latency_us");
                        if lat_path.exists() {
                            latency_paths.push(lat_path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        if governor_paths.is_empty() && latency_paths.is_empty() && pm_qos_fd.is_none() {
            return Err(anyhow::anyhow!("cpuidle nodes unavailable"));
        }

        Ok(Self {
            pm_qos_fd,
            governor_paths,
            latency_paths,
        })
    }

    pub fn disabled() -> Self {
        Self {
            pm_qos_fd: None,
            governor_paths: Vec::new(),
            latency_paths: Vec::new(),
        }
    }

    fn open_pm_qos() -> Option<RawFd> {
        match fs::OpenOptions::new()
            .write(true)
            .open("/dev/cpu_dma_latency")
        {
            Ok(f) => {
                use std::os::unix::io::IntoRawFd;
                Some(f.into_raw_fd())
            }
            Err(e) => {
                warn!("{}", t_with_args("sysfs-open-failed", &fluent_args!(
                    "path" => "/dev/cpu_dma_latency".to_string(),
                    "error" => e.to_string()
                )));
                None
            }
        }
    }

    pub fn set_governor(&self, governor: &str) -> Result<()> {
        for path in &self.governor_paths {
            if let Err(e) = crate::utils::write_to_file(path, governor.as_bytes()) {
                warn!("{}", t_with_args("sysfs-open-failed", &fluent_args!(
                    "path" => path.clone(),
                    "error" => e.to_string()
                )));
            }
        }
        Ok(())
    }

    pub fn set_latency(&self, latency_us: i32) -> Result<()> {
        if let Some(fd) = self.pm_qos_fd {
            let value = latency_us.to_ne_bytes();
            unsafe {
                let ret = libc::write(
                    fd,
                    value.as_ptr() as *const libc::c_void,
                    value.len(),
                );
                if ret < 0 {
                    warn!("{}", t_with_args("sysfs-open-failed", &fluent_args!(
                        "path" => "/dev/cpu_dma_latency".to_string(),
                        "error" => std::io::Error::last_os_error().to_string()
                    )));
                }
            }
        }

        for path in &self.latency_paths {
            if let Err(e) = crate::utils::write_to_file(path, latency_us.to_string().as_bytes()) {
                warn!("{}", t_with_args("sysfs-open-failed", &fluent_args!(
                    "path" => path.clone(),
                    "error" => e.to_string()
                )));
            }
        }

        Ok(())
    }
}

impl Drop for LatencyWriter {
    fn drop(&mut self) {
        if let Some(fd) = self.pm_qos_fd {
            unsafe {
                libc::close(fd);
            }
        }
    }
}
