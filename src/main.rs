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

mod common;
pub mod cpuset_manager;
pub mod ebpf_monitor;
pub mod fas_types;
pub mod gpu_manager;
pub mod i18n;
pub mod idle_dive;
mod logger;
mod monitor;
mod scheduler;
pub mod touch_boost;
pub mod utils;
use crate::i18n::{load_language, t, t_with_args};
use crate::scheduler::config::Config;
use anyhow::Result;
use log::{error, info};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<()> {
    // 1. 环境初始化
    if let Some(path) = std::env::args().nth(1) {
        nix::unistd::chdir(path.as_str())?;
    }

    let root = common::get_module_root();
    let log_dir = root.join("logs");
    std::fs::create_dir_all(&log_dir)?;

    // 2. 提前读取配置
    let config_path: std::path::PathBuf = root.join("config/config.yaml");
    let config = Config::from_file(config_path.to_str().unwrap()).unwrap_or_default();

    // 3. 立即加载语言（默认中文）
    let lang = if config.meta.language.is_empty() {
        "zh"
    } else {
        &config.meta.language
    };
    load_language(lang);

    // 4. 初始化日志（默认 INFO 等级）
    let log_level = if config.meta.loglevel.is_empty() {
        "INFO"
    } else {
        &config.meta.loglevel
    };
    logger::init(log_level)?;

    info!("{}", t("yumi-module-starting"));

    // 5. 创建通信通道与共享配置
    let (tx, rx) = mpsc::channel::<common::DaemonEvent>();

    let rules_path = monitor::config::get_rules_path();
    let initial_rules = crate::utils::read_config(&rules_path)
        .unwrap_or_else(|_| monitor::app_detect::get_default_rules());

    let config_arc = Arc::new(Mutex::new(initial_rules));
    let force_refresh_arc = Arc::new(AtomicBool::new(false));

    // 6. 启动 Scheduler
    if let Err(e) = scheduler::start_scheduler_thread(rx) {
        error!(
            "{}",
            t_with_args(
                "scheduler-module-start-failed",
                &fluent_args!("error" => e.to_string())
            )
        );
        return Err(e);
    }
    info!("{}", t("scheduler-module-started"));

    // 7. 启动 Monitor
    let monitor_thread = thread::Builder::new()
        .name("monitor_core".to_string())
        .spawn(move || {
            if let Err(e) = monitor::start_monitor_with_shared(tx, config_arc, force_refresh_arc) {
                error!(
                    "{}",
                    t_with_args(
                        "monitor-module-crashed",
                        &fluent_args!("error" => e.to_string())
                    )
                );
            }
        })?;

    info!("{}", t("monitor-module-started"));

    // 8. 挂起
    monitor_thread.join().unwrap();

    Ok(())
}
