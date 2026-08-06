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
//  CpuSetManager — 主控制器
//  动态调整进程的 CPU 核心绑定：通过 /dev/cpuset 或
//  /sys/fs/cgroup/cpuset 控制各 cgroup 组可使用的 CPU 核心，
//  实现前台绑大核、后台绑小核的省电调度。
// ════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::fluent_args;
use crate::i18n::{t, t_with_args};
use crate::utils::FastWriter;

use super::config::CpuSetConfig;

/// CPUSet 管理器
pub struct CpuSetManager {
    /// 共享配置（支持热重载，config watcher 线程会更新）
    config: Arc<RwLock<CpuSetConfig>>,
    /// 当前生效的模式名
    current_mode: String,
    /// cpuset 根路径
    cpuset_root: PathBuf,
    /// 各组 cpus 文件的写入器（键为组名，如 "top-app"）
    writers: HashMap<String, FastWriter>,
    /// 是否已初始化
    initialized: bool,
}

impl CpuSetManager {
    pub fn new(config: Arc<RwLock<CpuSetConfig>>) -> Self {
        Self {
            config,
            current_mode: String::new(),
            cpuset_root: PathBuf::new(),
            writers: HashMap::new(),
            initialized: false,
        }
    }

    /// 初始化管理器：探测 cpuset 挂载点并建立各组写入器
    pub fn init(&mut self) -> anyhow::Result<()> {
        let root = Self::detect_cpuset_root()?;
        self.cpuset_root = root;

        for group in ["top-app", "foreground", "background", "system-background", "restricted"] {
            let cpus_path = self.cpuset_root.join(group).join("cpus");
            if cpus_path.exists() {
                let writer = FastWriter::new(&cpus_path);
                if writer.is_valid() {
                    self.writers.insert(group.to_string(), writer);
                }
            }
        }

        if self.writers.is_empty() {
            log::warn!("{}", t("cpuset-no-groups"));
        } else {
            self.initialized = true;
            log::info!("{}", t_with_args("cpuset-init", &fluent_args!(
                "path" => self.cpuset_root.display().to_string(),
                "count" => self.writers.len().to_string()
            )));
        }
        Ok(())
    }

    /// 探测 cpuset 挂载点（/dev/cpuset 优先，其次是 /sys/fs/cgroup/cpuset）
    fn detect_cpuset_root() -> anyhow::Result<PathBuf> {
        for path in ["/dev/cpuset", "/sys/fs/cgroup/cpuset"] {
            if Path::new(path).exists() {
                return Ok(PathBuf::from(path));
            }
        }
        anyhow::bail!("{}", t("cpuset-no-root"))
    }

    /// 应用指定模式的 CPUSet 配置
    pub fn apply_mode(&mut self, mode: &str) -> anyhow::Result<()> {
        let config = self.config.read().unwrap_or_else(|e| e.into_inner());
        if !config.enabled {
            return Ok(());
        }
        if !self.initialized {
            log::warn!("{}", t("cpuset-not-initialized"));
            return Ok(());
        }

        let policy = match mode {
            "powersave" => config.modes.powersave.clone(),
            "balance" => config.modes.balance.clone(),
            "performance" => config.modes.performance.clone(),
            "fast" => config.modes.fast.clone(),
            "doze" => config.modes.doze.clone(),
            _ => config.modes.balance.clone(),
        };
        drop(config);
        let mut applied = Vec::new();
        let mut failed = 0usize;

        for (group, writer) in self.writers.iter_mut() {
            if let Some(value) = policy.value_for_group(group).filter(|v| !v.is_empty()) {
                if writer.write_value_force_str(value) {
                    applied.push(format!("{}={}", group, value));
                } else {
                    failed += 1;
                }
            }
        }

        self.current_mode = mode.to_string();
        if failed > 0 {
            log::warn!("{}", t_with_args("cpuset-partial-failed", &fluent_args!(
                "mode" => mode,
                "failed" => failed.to_string()
            )));
        }
        if !applied.is_empty() {
            log::debug!("{}", t_with_args("cpuset-applied", &fluent_args!(
                "mode" => mode,
                "detail" => applied.join(" ")
            )));
        }
        Ok(())
    }

    /// 将调度模式映射为 CPUSet 模式
    /// 例如：fas -> performance（游戏模式使用性能策略）
    pub fn mode_to_cpuset_mode(mode: &str) -> &str {
        if mode == "fas" {
            "performance"
        } else {
            mode
        }
    }

    /// 处理模式变更事件
    pub fn on_mode_change(&mut self, new_mode: &str) {
        if !self.config.read().unwrap_or_else(|e| e.into_inner()).enabled {
            return;
        }
        if new_mode == self.current_mode {
            return;
        }
        if let Err(e) = self.apply_mode(new_mode) {
            log::error!("{}", t_with_args("cpuset-apply-failed", &fluent_args!("error" => e.to_string())));
        }
    }

    /// 处理息屏事件
    pub fn on_screen_off(&mut self) {
        if !self.config.read().unwrap_or_else(|e| e.into_inner()).enabled {
            return;
        }
        if let Err(e) = self.apply_mode("doze") {
            log::error!("{}", t_with_args("cpuset-apply-failed", &fluent_args!("error" => e.to_string())));
        }
    }

    /// 处理亮屏事件（恢复当前模式）
    pub fn on_screen_on(&mut self, mode: &str) {
        if !self.config.read().unwrap_or_else(|e| e.into_inner()).enabled {
            return;
        }
        if let Err(e) = self.apply_mode(mode) {
            log::error!("{}", t_with_args("cpuset-apply-failed", &fluent_args!("error" => e.to_string())));
        }
    }

    /// 当前生效的模式
    pub fn current_mode(&self) -> &str {
        &self.current_mode
    }

    /// 对指定线程名分类 QoS 分组
    pub fn classify_thread_qos(comm: &str) -> Option<ThreadQosGroup> {
        let comm_trimmed = comm.trim();
        if comm_trimmed == "UI Thread"
            || comm_trimmed == "RenderThread"
            || comm_trimmed.starts_with("mali-")
            || comm_trimmed.starts_with("KGSL-")
        {
            Some(ThreadQosGroup::Foreground)
        } else if comm_trimmed.contains("async")
            || comm_trimmed.contains("log")
            || comm_trimmed.contains("Rx")
        {
            Some(ThreadQosGroup::SystemBackground)
        } else {
            None
        }
    }

    /// 读取前台进程的线程并分类绑定 QoS
    pub fn apply_ui_qos(&self, pid: i32) {
        if pid <= 0 {
            return;
        }
        let cpuset_root = if !self.cpuset_root.as_os_str().is_empty() {
            self.cpuset_root.clone()
        } else if let Ok(root) = Self::detect_cpuset_root() {
            root
        } else {
            return;
        };

        let task_dir = format!("/proc/{}/task", pid);
        let entries = match std::fs::read_dir(&task_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        let fg_tasks = cpuset_root.join("foreground/tasks");
        let sys_bg_tasks = cpuset_root.join("system-background/tasks");

        for entry in entries.flatten() {
            let tid_str = entry.file_name();
            let comm_path = entry.path().join("comm");
            if let Ok(comm) = std::fs::read_to_string(comm_path) {
                match Self::classify_thread_qos(&comm) {
                    Some(ThreadQosGroup::Foreground) => {
                        let _ = std::fs::write(&fg_tasks, tid_str.to_string_lossy().as_bytes());
                    }
                    Some(ThreadQosGroup::SystemBackground) => {
                        let _ = std::fs::write(&sys_bg_tasks, tid_str.to_string_lossy().as_bytes());
                    }
                    None => {}
                }
            }
        }
    }
}

/// 线程 QoS 隔离分组
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadQosGroup {
    Foreground,
    SystemBackground,
}

// ════════════════════════════════════════════════════════════════
//  单元测试
// ════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证线程 QoS 分割逻辑
    #[test]
    fn test_classify_thread_qos() {
        assert_eq!(
            CpuSetManager::classify_thread_qos("UI Thread"),
            Some(ThreadQosGroup::Foreground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("RenderThread"),
            Some(ThreadQosGroup::Foreground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("mali-cmar-worker"),
            Some(ThreadQosGroup::Foreground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("KGSL-3D-Context"),
            Some(ThreadQosGroup::Foreground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("async_task_1"),
            Some(ThreadQosGroup::SystemBackground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("logcat_writer"),
            Some(ThreadQosGroup::SystemBackground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("RxCachedThreadS"),
            Some(ThreadQosGroup::SystemBackground)
        );
        assert_eq!(
            CpuSetManager::classify_thread_qos("other_worker"),
            None
        );
    }
}
