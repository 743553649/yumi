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

use crate::monitor::config::RulesConfig;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

/// 模块根目录缓存。首次探测后全局复用，避免高频调用重复执行系统调用。
static MODULE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// 守护进程全局事件总线
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    /// 低频事件：前台应用切换或环境温度变化引起的模式改变
    ModeChange {
        package_name: String,
        pid: i32,
        mode: String,
        temperature: f64,
    },
    /// 高频事件：eBPF 捕获到的底层渲染帧数据
    FrameUpdate {
        frame_delta_ns: u64, // 纳秒级帧间隔
    },
    /// eBPF 全局系统负载更新 (每 X 毫秒触发一次)
    SystemLoadUpdate {
        /// 每个 CPU 核心的真实利用率 (0.0 ~ 1.0)，数组索引即 cpu_id
        core_utils: Vec<f32>,
        /// 如果当前有前台应用，这是该应用最吃 CPU 的那 1 个线程的利用率
        foreground_max_util: f32,
    },

    ConfigReload(RulesConfig),

    ScreenStateChange(bool),
}

/// 获取模块根目录的绝对路径（首次调用探测，之后缓存复用）
///
/// 被 logger、i18n、app_detect 等高频调用，探测逻辑含多次
/// `Path::exists()` 系统调用与 exe 回溯，缓存后整段探测仅执行一次。
pub fn get_module_root() -> PathBuf {
    MODULE_ROOT.get_or_init(detect_module_root).clone()
}

/// 实际探测模块根目录（仅在 `get_module_root()` 首次调用时执行一次）
fn detect_module_root() -> PathBuf {
    // 1. 优先校验当前工作目录 (cwd)
    if let Ok(cwd) = env::current_dir() {
        if cwd.join("rules.yaml").exists() || cwd.join("config/config.yaml").exists() {
            return cwd;
        }
        if cwd.join("module/rules.yaml").exists() || cwd.join("module/config/config.yaml").exists()
        {
            return cwd.join("module");
        }
    }

    // 2. 检查常见部署与运行路径
    let candidate_paths = [
        PathBuf::from("/data/adb/modules/yumi"),
        PathBuf::from("/storage/emulated/0/yumi"),
        PathBuf::from("/sdcard/yumi"),
        PathBuf::from("/mnt/sdcard/yumi"),
        PathBuf::from("/storage/emulated/0/yumi/module"),
    ];
    for p in &candidate_paths {
        if p.join("rules.yaml").exists() || p.join("config/config.yaml").exists() {
            return p.clone();
        }
    }

    // 3. 回溯 exe 路径
    let exe_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("/"));
    let mut curr = exe_path.as_path();
    while let Some(parent) = curr.parent() {
        if parent.join("rules.yaml").exists() || parent.join("config/config.yaml").exists() {
            return parent.to_path_buf();
        }
        if parent == curr {
            break;
        }
        curr = parent;
    }

    // 4. 默认退回到 Magisk/KernelSU 部署路径
    PathBuf::from("/data/adb/modules/yumi")
}
