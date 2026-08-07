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
//  LatencyWriter — 支持 sysfs 与 PM-QoS (/dev/cpu_dma_latency) 降级
// ════════════════════════════════════════════════════════════════

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::utils::{FastWriter, enable_perm};

/// Idle latency 写入器，支持 `/sys/devices/system/cpu/cpuidle/latency_us`
/// 与 PM-QoS `/dev/cpu_dma_latency` 自动降级与回退
pub struct LatencyWriter {
    sysfs_writer: Option<FastWriter>,
    pm_qos_file: Option<File>,
    path: PathBuf,
}

impl LatencyWriter {
    pub fn new(sysfs_path: &str) -> Self {
        let sysfs_writer = if Path::new(sysfs_path).exists() {
            let writer = FastWriter::new(sysfs_path);
            if writer.is_valid() {
                Some(writer)
            } else {
                None
            }
        } else {
            None
        };

        let pm_qos_file = if sysfs_writer.is_none() {
            Self::open_pm_qos()
        } else {
            None
        };

        Self {
            sysfs_writer,
            pm_qos_file,
            path: PathBuf::from(sysfs_path),
        }
    }

    fn open_pm_qos() -> Option<File> {
        let pm_qos_path = "/dev/cpu_dma_latency";
        if Path::new(pm_qos_path).exists() {
            let _ = enable_perm(pm_qos_path);
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(pm_qos_path)
                .ok()
        } else {
            None
        }
    }

    pub fn is_valid(&self) -> bool {
        self.sysfs_writer.is_some() || self.pm_qos_file.is_some()
    }

    pub fn write_latency(&mut self, latency_us: u32) -> bool {
        if let Some(writer) = &mut self.sysfs_writer {
            if writer.write_value_force(latency_us) {
                return true;
            }
            log::warn!(
                "Writing to cpuidle latency_us {:?} failed, falling back to PM-QoS /dev/cpu_dma_latency",
                self.path
            );
            self.sysfs_writer = None;
        }

        if self.pm_qos_file.is_none() {
            self.pm_qos_file = Self::open_pm_qos();
        }

        if let Some(file) = &mut self.pm_qos_file {
            let latency_i32 = latency_us as i32;
            let bytes = latency_i32.to_ne_bytes();
            if file.seek(SeekFrom::Start(0)).is_ok() && file.write_all(&bytes).is_ok() {
                return true;
            }
            log::warn!("Writing to PM-QoS /dev/cpu_dma_latency failed");
        }

        false
    }
}
