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

use anyhow::Result;
use inotify::{Inotify, WatchMask};
use log;
use nix::unistd::{AccessFlags, access};
use serde::de::DeserializeOwned;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::fluent_args;
use crate::i18n::t_with_args;

/// 向文件写入内容，并处理可能的错误
pub fn write_to_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> Result<()> {
    let path = path.as_ref();

    // 尝试修改权限以便写入
    if path.exists() {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o664));
    }

    fs::write(path, content)?;
    Ok(())
}

// 尝试写入内容 (不抛出错误，只记录警告)
pub fn try_write_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> Result<()> {
    if let Err(e) = write_to_file(path.as_ref(), content) {
        log::warn!("Failed to write to {}: {}.", path.as_ref().display(), e);
    }
    Ok(())
}

/// 写入文件内容并回读确认。写入失败或回读不匹配均记录警告。
/// 使用取巧比较：对于 scheduler 节点，回读可能是 "[none] mq-deadline" 格式，
/// 检查 expected 是否出现在回读内容中。
pub fn write_and_verify<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, expected: C) -> bool {
    let path_ref = path.as_ref();
    let expected_bytes = expected.as_ref();
    let expected_str = String::from_utf8_lossy(expected_bytes).trim_end().to_string();

    if let Err(e) = write_to_file(path_ref, expected_bytes) {
        log::warn!(
            "[IO] write failed to {}: {}",
            path_ref.display(),
            e
        );
        return false;
    }

    match read_file_content(path_ref.to_str().unwrap_or_default()) {
        Ok(actual) => {
            let actual_trimmed = actual.trim();
            // scheduler 节点返回 "none" 或 "[none] mq-deadline"
            // read_ahead_kb/nomerges/iostats 直接返回数值
            if actual_trimmed.contains(&expected_str) || actual_trimmed == expected_str {
                true
            } else {
                log::warn!(
                    "[IO] {}: wrote '{}', read back '{}'",
                    path_ref.display(),
                    expected_str,
                    actual_trimmed
                );
                false
            }
        }
        Err(e) => {
            log::warn!("[IO] {}: write OK but read-back failed: {}", path_ref.display(), e);
            false
        }
    }
}

pub fn enable_perm<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o664))?;
    }
    Ok(())
}

/// 监控指定路径的文件/目录事件
pub fn watch_path<P: AsRef<Path>>(path_to_watch: P) -> Result<()> {
    let mut inotify = Inotify::init()?;
    inotify
        .watches()
        .add(path_to_watch, WatchMask::CLOSE_WRITE | WatchMask::MODIFY)?;

    let mut buffer = [0u8; 1024];
    inotify.read_events_blocking(&mut buffer)?;
    Ok(())
}

/// 监听目录变更，返回变更的文件名
/// 支持 IN_CREATE 以便捕获 sed -i（rename 策略）替换文件后的事件
pub fn watch_path_for_file<P: AsRef<Path>>(path_to_watch: P) -> Result<String> {
    let mut inotify = Inotify::init()?;
    inotify
        .watches()
        .add(
            &path_to_watch,
            WatchMask::CLOSE_WRITE | WatchMask::MODIFY | WatchMask::CREATE,
        )?;

    let mut buffer = [0u8; 1024];
    let events = inotify.read_events_blocking(&mut buffer)?;

    for event in events {
        if let Some(name) = event.name {
            return Ok(name.to_string_lossy().to_string());
        }
    }

    // 如果没有文件名（理论上不会发生），返回空字符串
    Ok(String::new())
}

// 通用的读取文件为 f64 的函数
pub fn read_f64_from_file(path: &str) -> Result<f64> {
    let mut content = String::new();
    File::open(path)?.read_to_string(&mut content)?;
    let val: f64 = content.trim().parse()?;
    Ok(val)
}

// 辅助函数：读取文件内容为 String
pub fn read_file_content(path: &str) -> Result<String> {
    let mut content = String::new();
    File::open(path)?.read_to_string(&mut content)?;
    Ok(content.trim().to_string())
}

// 查找 CPU 温度路径的逻辑
pub fn find_cpu_temp_path() -> Result<String> {
    let thermal_path = "/sys/class/thermal";
    let thermal_dir = Path::new(thermal_path);

    if !thermal_dir.exists() {
        return Err(anyhow::anyhow!("Thermal directory not found"));
    }

    for entry in fs::read_dir(thermal_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                if dir_name.starts_with("thermal_zone") {
                    let type_path = path.join("type");
                    // 修复 E0532 模式匹配错误: 直接使用 if let Ok(...)
                    if let Ok(type_content) =
                        read_file_content(type_path.to_str().unwrap_or_default())
                    {
                        if type_content.contains("soc_max")
                            || type_content.contains("mtktscpu")
                            || type_content.contains("cpu-1-")
                            || type_content.contains("cpu-0-0-usr")
                        {
                            let temp_path = path.join("temp");
                            if temp_path.exists() {
                                return Ok(temp_path.to_str().unwrap().to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    Err(anyhow::anyhow!("Valid CPU thermal zone not found"))
}

// --- SysPathExist 结构体 ---
pub struct SysPathExist {
    pub qcom_feas_exist: bool,
    pub mtk_feas_exist: bool,
    pub walt_exist: bool,
    pub stune_exist: bool,
    pub hi6220_ufs_exist: bool,
    pub cpuctl_top_app_exist: bool,
    pub cpuctl_foreground_exist: bool,
    pub cpuctl_background_exist: bool,
    pub cpuset_top_app_exist: bool,
    pub cpuset_foreground_exist: bool,
    pub cpuset_background_exist: bool,
    pub cpuset_system_background_exist: bool,
    pub cpuset_restricted_exist: bool,
    pub cpuset_root_exist: bool,
    pub cpuidle_governor_exist: bool,
    pub sda_scheduler_exist: bool,
}

impl SysPathExist {
    pub fn new() -> Self {
        Self {
            qcom_feas_exist: Self::path_exists("/sys/module/perfmgr/parameters/perfmgr_enable"),
            mtk_feas_exist: Self::path_exists("/sys/module/mtk_fpsgo/parameters/perfmgr_enable"),
            walt_exist: Self::path_exists("/proc/sys/walt"),
            stune_exist: Self::path_exists("/dev/stune"),
            hi6220_ufs_exist: Self::path_exists(
                "/sys/bus/platform/devices/hi6220-ufs/ufs_clk_gate_disable",
            ),
            cpuctl_top_app_exist: Self::path_exists("/dev/cpuctl/top-app"),
            cpuctl_foreground_exist: Self::path_exists("/dev/cpuctl/foreground"),
            cpuctl_background_exist: Self::path_exists("/dev/cpuctl/background"),
            cpuset_top_app_exist: Self::path_exists("/dev/cpuset/top-app"),
            cpuset_foreground_exist: Self::path_exists("/dev/cpuset/foreground"),
            cpuset_background_exist: Self::path_exists("/dev/cpuset/background"),
            cpuset_system_background_exist: Self::path_exists("/dev/cpuset/system-background"),
            cpuset_restricted_exist: Self::path_exists("/dev/cpuset/restricted"),
            cpuset_root_exist: Self::path_exists("/dev/cpuset"),
            cpuidle_governor_exist: Self::path_exists(
                "/sys/devices/system/cpu/cpuidle/current_governor",
            ),
            sda_scheduler_exist: Self::path_exists("/sys/block/sda/queue/scheduler"),
        }
    }

    fn path_exists(path: &str) -> bool {
        access(path, AccessFlags::F_OK).is_ok()
    }
}

// ════════════════════════════════════════════════════════════════
//  FastWriter — 带去重 + unmount 的 sysfs 写入器
// ════════════════════════════════════════════════════════════════

pub struct FastWriter {
    // fallback: some sysfs nodes reject persistent FD writes (EOPNOTSUPP),
    // switch to fresh open+write+close via fs::write
    use_fs_write: bool,
    file: Option<File>,
    // buf 容量 64 字节，为 cpuset 掩码极端格式留足冗余（str 写入路径按 self.buf.len() 动态限长）
    buf: [u8; 64],
    path: PathBuf,
}

impl FastWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path_ref = path.as_ref();
        Self::try_unmount(path_ref);
        let _ = enable_perm(path_ref);
        let file = OpenOptions::new().write(true).open(path_ref)
            .map_err(|e| log::error!("{}", t_with_args("sysfs-open-failed", &fluent_args!("path" => path_ref.display().to_string(), "error" => e.to_string()))))
            .ok();
        Self {
            file,
            buf: [0u8; 64],
            path: path_ref.to_path_buf(),
            use_fs_write: false,
        }
    }

    fn try_unmount(path: &Path) {
        if let Some(path_str) = path.to_str() {
            if let Ok(cpath) = std::ffi::CString::new(path_str) {
                let ret = unsafe { libc::umount2(cpath.as_ptr(), libc::MNT_DETACH) };
                if ret != 0 {
                    let errno = std::io::Error::last_os_error();
                    if errno.raw_os_error() != Some(libc::EINVAL)
                        && errno.raw_os_error() != Some(libc::ENOENT)
                    {
                        log::debug!(
                            "{}",
                            t_with_args(
                                "sysfs-umount2-failed",
                                &fluent_args!("path" => path_str, "error" => errno.to_string())
                            )
                        );
                    }
                }
            }
        }
    }

    pub fn re_unmount(&self) {
        Self::try_unmount(&self.path);
    }

    pub fn write_value_force(&mut self, value: u32) -> bool {
        let len = Self::u32_to_buf(value, &mut self.buf);
        let mut local = [0u8; 64];
        local[..len].copy_from_slice(&self.buf[..len]);
        self.do_write_bytes(&local[..len], false)
    }

    /// 写入字符串值（自动追加换行），用于 cpuset 等需要文本内容的节点
    /// EINVAL 对文本节点是永久性错误（非法掩码），按 warn 级别记录
    pub fn write_value_force_str(&mut self, value: &str) -> bool {
        let bytes = value.as_bytes();
        if bytes.is_empty() || bytes.len() > self.buf.len() - 1 {
            log::warn!(
                "write str to {:?} skipped: length overflow ({})",
                self.path,
                bytes.len()
            );
            return false;
        }
        self.buf[..bytes.len()].copy_from_slice(bytes);
        self.buf[bytes.len()] = b'\n';
        let mut local = [0u8; 64];
        local[..bytes.len() + 1].copy_from_slice(&self.buf[..bytes.len() + 1]);
        self.do_write_bytes(&local[..bytes.len() + 1], true)
    }

    pub fn is_valid(&self) -> bool {
        self.file.is_some()
    }

    fn do_write_bytes(&mut self, bytes: &[u8], text_node: bool) -> bool {
        // Take file temporarily — allows self.file = None in error path for permanent disable
        let mut file = match self.file.take() {
            Some(f) => f,
            None => {
                if self.use_fs_write {
                    return fs::write(&self.path, bytes).is_ok();
                }
                return false;
            }
        };
        let _ = file.seek(SeekFrom::Start(0));
        match file.write_all(bytes) {
            Ok(()) => {
                self.file = Some(file);
                true
            }
            Err(e) => {
                match e.raw_os_error() {
                    Some(libc::EINVAL) if text_node => {
                        log::warn!(
                            "{}",
                            t_with_args(
                                "sysfs-write-text-failed",
                                &fluent_args!("value" => String::from_utf8_lossy(bytes).trim_end().to_string(), "error" => e.to_string())
                            )
                        );
                        self.file = Some(file);
                    }
                    Some(libc::EINVAL) | Some(libc::EBUSY) => {
                        log::debug!("write to {:?} skipped: {}", self.path, e);
                        self.file = Some(file);
                    }
                    Some(libc::EOPNOTSUPP) => {
                        log::debug!(
                            "[FastWriter] node {:?} got EOPNOTSUPP with persistent FD, switching to fs::write fallback",
                            self.path
                        );
                        drop(file);
                        // Retry with fs::write (open+write+close)
                        if fs::write(&self.path, bytes).is_ok() {
                            self.use_fs_write = true;
                            return true;
                        }
                        log::warn!(
                            "[FastWriter] node {:?} rejects writes even with fs::write, permanently disabling",
                            self.path
                        );
                    }
                    _ => {
                        log::warn!(
                            "{}",
                            t_with_args(
                                "sysfs-write-freq-failed",
                                &fluent_args!("freq" => String::from_utf8_lossy(bytes).trim_end().to_string(), "error" => e.to_string())
                            )
                        );
                        self.file = Some(file);
                    }
                }
                false
            }
        }
    }

    fn u32_to_buf(mut v: u32, buf: &mut [u8; 64]) -> usize {
        if v == 0 {
            buf[0] = b'0';
            buf[1] = b'\n';
            return 2;
        }
        let mut pos = 18;
        while v > 0 {
            buf[pos] = b'0' + (v % 10) as u8;
            v /= 10;
            pos -= 1;
        }
        let start = pos + 1;
        let digit_len = 19 - start;
        buf.copy_within(start..19, 0);
        buf[digit_len] = b'\n';
        digit_len + 1
    }
}

// ════════════════════════════════════════════════════════════════
//  通用跨模块工具函数
// ════════════════════════════════════════════════════════════════

/// Serde 默认值辅助函数：始终返回 true
pub fn default_true() -> bool {
    true
}

/// 读取文件内容并解析为 i32
pub fn read_i32_from_file(path: &str) -> Result<i32> {
    let mut content = String::new();
    File::open(path)?.read_to_string(&mut content)?;
    Ok(content.trim().parse()?)
}

/// 获取与 BPF ktime_get_ns() 绝对对齐的单调时钟时间 (纳秒)
pub fn get_ktime_ns() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
}

/// Serde 反序列化辅助：从 YAML 文件读取配置，解析失败时返回 Default
pub fn read_config<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned + Default,
    P: AsRef<Path>,
{
    let path_ref = path.as_ref();
    match File::open(path_ref) {
        Ok(mut file) => {
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            serde_yaml::from_str(&s).or_else(|e| {
                log::warn!(
                    "[Config] Parse error {}: {}. Default.",
                    path_ref.display(),
                    e
                );
                Ok(T::default())
            })
        }
        Err(_) => {
            log::warn!("[Config] Not found: {}. Default.", path_ref.display());
            Ok(T::default())
        }
    }
}
