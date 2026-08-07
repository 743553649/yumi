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
//  Scheduler 主线程 — 负责所有的状态机流转与调度干预
//  编排 FAS / CLG / CPUSet / IdleDive / TouchBoost 子模块，处理
//  IPC 通道下发的 DaemonEvent。
// ════════════════════════════════════════════════════════════════

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::common::{self, DaemonEvent};
use crate::fluent_args;
use crate::i18n::{load_language, t, t_with_args};
use crate::logger;
use crate::utils;

use super::config::Config;
use super::policy::get_cpu_policies;
use super::scheduler::CpuScheduler;

pub fn start_scheduler_thread(rx: mpsc::Receiver<DaemonEvent>) -> Result<()> {
    let root = common::get_module_root();
    let config_path = root.join("config/config.yaml");
    let config_dir = root.join("config");

    let config = Config::from_file(config_path.to_str().unwrap_or("")).unwrap_or_default();

    let shared_config = Arc::new(RwLock::new(config));
    let shared_mode_name = Arc::new(Mutex::new("balance".to_string()));
    let sys_path_exist = Arc::new(utils::SysPathExist::new());

    // ==========================================
    // Config Watcher 线程
    // ==========================================
    let config_clone = shared_config.clone();
    let sys_path_clone = sys_path_exist.clone();

    // CPUSet 共享配置（支持热重载）
    let cpuset_path = config_dir.join("cpuset.yaml");
    let shared_cpuset_config = Arc::new(std::sync::RwLock::new(
        crate::utils::read_config::<crate::cpuset_manager::CpuSetConfig, _>(&cpuset_path)
            .unwrap_or_default(),
    ));
    let cpuset_config_watcher = shared_cpuset_config.clone();

    // CPU 静止下潜共享配置（支持热重载）
    let idle_dive_path = config_dir.join("idle_dive.yaml");
    let shared_idle_dive_config = Arc::new(std::sync::RwLock::new(
        crate::utils::read_config::<crate::idle_dive::IdleDiveConfig, _>(&idle_dive_path)
            .unwrap_or_default(),
    ));
    let idle_dive_config_watcher = shared_idle_dive_config.clone();

    // TouchBoost 共享配置（支持热重载）
    let touch_boost_path = config_dir.join("touch_boost.yaml");
    let shared_touch_boost_config = Arc::new(std::sync::RwLock::new(
        crate::utils::read_config::<crate::touch_boost::TouchBoostConfig, _>(&touch_boost_path)
            .unwrap_or_default(),
    ));
    let touch_boost_config_watcher = shared_touch_boost_config.clone();

    // GPU 共享配置（支持热重载）
    let gpu_path = config_dir.join("gpu.yaml");
    let shared_gpu_config = Arc::new(std::sync::RwLock::new(
        crate::utils::read_config::<crate::gpu_manager::GpuConfig, _>(&gpu_path)
            .unwrap_or_default(),
    ));
    let gpu_config_watcher = shared_gpu_config.clone();

    thread::Builder::new()
        .name("config_watcher".to_string())
        .spawn(move || {
            loop {
                let changed_file = match utils::watch_path_for_file(&config_dir) {
                    Ok(file) => file,
                    Err(e) => {
                        log::error!(
                            "{}",
                            t_with_args(
                                "config-watch-error",
                                &fluent_args!("error" => e.to_string())
                            )
                        );
                        continue;
                    }
                };

                log::info!(
                    "{}",
                    t_with_args(
                        "config-file-changed",
                        &fluent_args!("file" => changed_file.clone())
                    )
                );

                // 主配置文件变更
                if changed_file == "config.yaml" || changed_file.is_empty() {
                    let old_lang = config_clone
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .meta
                        .language
                        .clone();

                    match Config::from_file(config_path.to_str().unwrap_or("")) {
                        Ok(new_config) => {
                            logger::update_level(&new_config.meta.loglevel);
                            *config_clone.write().unwrap_or_else(|e| e.into_inner()) = new_config;

                            let new_lang = config_clone
                                .read()
                                .unwrap_or_else(|e| e.into_inner())
                                .meta
                                .language
                                .clone();
                            if old_lang != new_lang {
                                load_language(&new_lang);
                            }

                            log::info!("{}", t("config-reloaded-success"));

                            let scheduler =
                                CpuScheduler::new(config_clone.clone(), sys_path_clone.clone());
                            if let Err(e) = scheduler.apply_system_tweaks() {
                                log::error!(
                                    "{}",
                                    t_with_args(
                                        "config-apply-tweaks-failed",
                                        &fluent_args!("error" => e.to_string())
                                    )
                                );
                            }
                        }
                        Err(load_err) => log::error!(
                            "{}",
                            t_with_args(
                                "config-reload-fail",
                                &fluent_args!("error" => load_err.to_string())
                            )
                        ),
                    }
                }

                // CPUSet 配置变更
                if changed_file == "cpuset.yaml" || changed_file.is_empty() {
                    let new_cpuset = crate::utils::read_config::<
                        crate::cpuset_manager::CpuSetConfig,
                        _,
                    >(&cpuset_path)
                    .unwrap_or_default();
                    *cpuset_config_watcher
                        .write()
                        .unwrap_or_else(|e| e.into_inner()) = new_cpuset;
                    log::info!("{}", t("cpuset-config-reloaded"));
                }

                // IdleDive 配置变更
                if changed_file == "idle_dive.yaml" || changed_file.is_empty() {
                    let new_idle_dive = crate::utils::read_config::<
                        crate::idle_dive::IdleDiveConfig,
                        _,
                    >(&idle_dive_path)
                    .unwrap_or_default();
                    *idle_dive_config_watcher
                        .write()
                        .unwrap_or_else(|e| e.into_inner()) = new_idle_dive;
                    log::info!("{}", t("idle-dive-config-reloaded"));
                }

                // TouchBoost 配置变更
                if changed_file == "touch_boost.yaml" || changed_file.is_empty() {
                    let new_touch_boost = crate::utils::read_config::<
                        crate::touch_boost::TouchBoostConfig,
                        _,
                    >(&touch_boost_path)
                    .unwrap_or_default();
                    *touch_boost_config_watcher
                        .write()
                        .unwrap_or_else(|e| e.into_inner()) = new_touch_boost;
                    log::info!("{}", t("touch-boost-config-reloaded"));
                }

                // GPU 配置变更
                if changed_file == "gpu.yaml" || changed_file.is_empty() {
                    let new_gpu =
                        crate::utils::read_config::<crate::gpu_manager::GpuConfig, _>(&gpu_path)
                            .unwrap_or_default();
                    *gpu_config_watcher
                        .write()
                        .unwrap_or_else(|e| e.into_inner()) = new_gpu;
                    log::info!("{}", t("gpu-config-reloaded"));
                }
            }
        })?;

    // GPU 保活线程：定期重新写入当前模式的 GPU 配置，防止第三方覆盖
    let gpu_keepalive_config = shared_gpu_config.clone();
    let gpu_keepalive_mode = shared_mode_name.clone();
    thread::Builder::new()
        .name("gpu_keepalive".to_string())
        .spawn(move || {
            let interval = Duration::from_secs(
                gpu_keepalive_config
                    .read()
                    .unwrap_or_else(|e| e.into_inner())
                    .keepalive_interval_s,
            );
            log::info!(
                "{}",
                t_with_args(
                    "gpu-keepalive-started",
                    &fluent_args!("secs" => interval.as_secs().to_string())
                )
            );
            loop {
                thread::sleep(interval);
                let mode = gpu_keepalive_mode
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                let cfg = gpu_keepalive_config
                    .read()
                    .unwrap_or_else(|e| e.into_inner());

                if !cfg.enabled {
                    continue;
                }

                let mode_cfg = cfg.modes.get(&mode);
                let gov = mode_cfg
                    .map(|c| c.governor.as_str())
                    .unwrap_or("msm-adreno-tz");

                // Re-write critical GPU sysfs nodes to prevent third-party override
                let kgsl_path = std::path::Path::new("/sys/class/kgsl/kgsl-3d0");
                if kgsl_path.exists() {
                    // Only keep alive the governor (most common override target).
                    // max_gpuclk is already set by GpuManager on every mode switch
                    // and doesn't need keepalive protection.
                    let _ = crate::utils::try_write_file(
                        kgsl_path.join("devfreq/governor"),
                        gov.as_bytes(),
                    );
                    // force_no_nap: for fast mode, re-apply if overridden
                    let nap_val = mode_cfg
                        .map(|c| if c.force_no_nap > 0 { b"1" as &[u8] } else { b"0" as &[u8] })
                        .unwrap_or(b"0");
                    let _ = crate::utils::try_write_file(
                        kgsl_path.join("force_no_nap"),
                        nap_val,
                    );
                }
            }
        })?;

    log::info!("{}", t("main-config-watch-thread-create"));

    // ==========================================
    // IPC 监听主线程 (负责所有的状态机流转与调度干预)
    // ==========================================
    let config_clone = shared_config.clone();
    let mode_clone = shared_mode_name.clone();
    let shared_gpu_config_for_ipc = shared_gpu_config.clone();

    thread::Builder::new()
        .name("scheduler_ipc".to_string())
        .spawn(move || {
            log::info!("{}", t("scheduler-ipc-started"));

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {

            let root = common::get_module_root();
            let mode_file_path = root.join("current_mode.txt");

            let mut fas_controller = crate::scheduler::fas::FasController::new();
            let mut cpu_governor = crate::scheduler::cpu_load_governor::CpuLoadGovernor::new();

            // CPUSet 管理器：动态调整进程 CPU 核心绑定
            let cpuset_mgr_config = shared_cpuset_config.clone();
            let cpuset_manager = Arc::new(RwLock::new(crate::cpuset_manager::CpuSetManager::new(cpuset_mgr_config)));
            if let Err(e) = cpuset_manager.write().unwrap_or_else(|e| e.into_inner()).init() {
                log::error!("{}", t_with_args("cpuset-init-failed", &fluent_args!("error" => e.to_string())));
            }

            // CPU 静止下潜：低负载时主动让 CPU 进入更深的 C-state
            let idle_dive_config = shared_idle_dive_config.clone();
            let mut idle_dive = crate::idle_dive::IdleDiveController::new(idle_dive_config);
            if let Err(e) = idle_dive.init() {
                log::error!("{}", t_with_args("idle-dive-init-failed", &fluent_args!("error" => e.to_string())));
            }

            // TouchBoost：触摸提频
            let touch_boost_config = shared_touch_boost_config.clone();
            let policies_for_touch = get_cpu_policies();
            let (touch_tx, touch_rx) = mpsc::channel::<bool>();
            let fas_silenced_flag = Arc::new(AtomicBool::new(false));
            if !policies_for_touch.is_empty() {
                crate::touch_boost::start_touch_listener_thread(
                    touch_boost_config,
                    policies_for_touch,
                    touch_tx,
                    fas_silenced_flag.clone(),
                );
            }

            // GPU 控制器：Adreno GPU 频率与调速器管理
            let gpu_config = shared_gpu_config_for_ipc.clone();
            let mut gpu_manager = crate::gpu_manager::GpuManager::new(
                &gpu_config.read().unwrap_or_else(|e| e.into_inner())
            );
            if let Err(e) = gpu_manager.init() {
                log::error!("{}", t_with_args("gpu-init-failed", &fluent_args!("error" => e.to_string())));
            }

            let rules_path = crate::monitor::config::get_rules_path();
            let mut current_rules = crate::utils::read_config::<crate::monitor::config::RulesConfig, _>(&rules_path).unwrap_or_default();

            // 状态机变量
            let mut fas_suspended_at: Option<Instant> = None;
            let mut fas_suspended_package = String::new();
            const FAS_SUSPEND_GRACE_SECS: u64 = 5;

            let mut is_screen_on = true; // 屏幕状态标记
            let mut touch_channel_disconnected_warned = false; // 防刷屏：记录 channel 断开警告标志

            let temp_sensor_path = crate::utils::find_cpu_temp_path().unwrap_or_default();
            let mut last_temp_update = Instant::now();

            let get_clg_cfg = |config: &Config, mode: &str| -> crate::scheduler::config::CpuLoadGovernorConfig {
                config.get_mode(mode)
                    .map(|m| m.cpu_load_governor.clone())
                    .unwrap_or(crate::scheduler::config::CpuLoadGovernorConfig {
                        enabled: false,
                        ..Default::default()
                    })
            };

            // 启动时初始化
            {
                let current_mode = mode_clone.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if current_mode != "fas" {
                    let config_lock = config_clone.read().unwrap_or_else(|e| e.into_inner());
                    let clg_cfg = get_clg_cfg(&config_lock, &current_mode);
                    if clg_cfg.enabled {
                        cpu_governor.init_policies(&clg_cfg);
                        log::info!("{}", t_with_args("scheduler-clg-init", &fluent_args!("mode" => current_mode.clone())));
                    }
                }
                // 启动时应用当前模式的 CPUSet 分配
                let _ = cpuset_manager.write().unwrap_or_else(|e| e.into_inner()).apply_mode(&current_mode);

                // 启动时 GPU 跟随当前模式
                let _ = gpu_manager.apply_mode(&current_mode);
            }

            for msg in rx {
                match msg {
                    // --- 1. 屏幕状态事件 (息屏深度睡眠) ---
                    DaemonEvent::ScreenStateChange(screen_on) => {
                        is_screen_on = screen_on;
                        let current_mode = mode_clone.lock().unwrap_or_else(|e| e.into_inner()).clone();

                        if !is_screen_on {
                            log::info!("{}", t("scheduler-doze-enable"));

                            // CPU 静止下潜：息屏进入更深的下潜状态
                            idle_dive.enter_doze();

                            // 息屏立刻剥夺 FAS 的频率控制权
                            if current_mode == "fas" {
                                fas_controller.reset_all_freqs();
                                fas_controller.clear_game();
                                fas_controller.policies.clear();
                                fas_suspended_at = None;
                                fas_suspended_package.clear();
                            }

                            // 强行让 CLG 接管，并动态生成一个极致省电配置
                            let config_lock = config_clone.read().unwrap_or_else(|e| e.into_inner());
                            let mut doze_cfg = get_clg_cfg(&config_lock, "powersave");
                            doze_cfg.enabled = true;
                            doze_cfg.perf_floor = 0.0;
                            doze_cfg.perf_ceil = doze_cfg.perf_ceil.min(0.30); // 锁死天花板最高只给 30% 性能
                            doze_cfg.smoothing_up = 0.05;           // 升频极其迟钝
                            doze_cfg.smoothing_down = 1.0;          // 瞬间降频
                            doze_cfg.up_rate_limit_ticks = 5;       // 升频速率限制从 3 提高到 5

                            cpu_governor.init_policies(&doze_cfg);
                            cpuset_manager.write().unwrap_or_else(|e| e.into_inner()).on_screen_off();
                            gpu_manager.enter_doze();
                        } else {
                            log::info!("{}", t("scheduler-doze-restore"));

                            // CPU 静止下潜：亮屏退出息屏下潜
                            idle_dive.exit_doze();

                            let config_lock = config_clone.read().unwrap_or_else(|e| e.into_inner());
                            let clg_cfg = get_clg_cfg(&config_lock, &current_mode);

                            if current_mode != "fas" {
                                if clg_cfg.enabled {
                                    if cpu_governor.is_active() { cpu_governor.reload_config(&clg_cfg); }
                                    else { cpu_governor.init_policies(&clg_cfg); }
                                } else { cpu_governor.release(); }
                            } else {
                                cpu_governor.release();
                                *mode_clone.lock().unwrap_or_else(|e| e.into_inner()) = String::new();
                            }

                            // 亮屏恢复 CPUSet 分配（游戏模式使用 performance 策略）
                            let restore_mode = crate::cpuset_manager::CpuSetManager::mode_to_cpuset_mode(&current_mode);
                            cpuset_manager.write().unwrap_or_else(|e| e.into_inner()).on_screen_on(restore_mode);
                            gpu_manager.exit_doze(&current_mode);
                        }
                    },

                    // --- 2. 前台模式切换事件 ---
                    DaemonEvent::ModeChange { package_name, pid, mode, temperature } => {
                        let mut current_mode_lock = mode_clone.lock().unwrap_or_else(|e| e.into_inner());
                        let old_mode = current_mode_lock.clone();

                        if old_mode != mode {
                            log::info!("{}", t_with_args("scheduler-mode-change-request", &fluent_args!(
                                "old" => old_mode.clone(), "new" => mode.as_str(), "pkg" => package_name.as_str(), "temp" => temperature
                            )));

                            *current_mode_lock = mode.clone();
                            drop(current_mode_lock);

                            let _ = utils::try_write_file(&mode_file_path, mode.as_bytes());

                            // CPUSet 跟随模式切换（游戏模式使用 performance 策略）
                            // 息屏时跳过：不能覆盖 doze 的核心约束（与 CLG 的 is_screen_on 保护对齐）
                            if is_screen_on && cpuset_manager.read().unwrap_or_else(|e| e.into_inner()).current_mode() != mode {
                                let cpuset_mode = crate::cpuset_manager::CpuSetManager::mode_to_cpuset_mode(&mode);
                                cpuset_manager.write().unwrap_or_else(|e| e.into_inner()).on_mode_change(cpuset_mode);
                            }

                            // GPU 跟随模式切换（息屏时 GPU 保持在 doze 模式）
                            if is_screen_on {
                                let _ = gpu_manager.apply_mode(&mode);
                            }

                            if mode != "fas" {
                                let cpuset_mgr = cpuset_manager.clone();
                                std::thread::spawn(move || {
                                    cpuset_mgr.read().unwrap_or_else(|e| e.into_inner()).apply_ui_qos(pid);
                                });
                            }

                            if mode == "fas" {
                                // FAS 模式：静默 TouchBoost，避免 min_freq 提频与 FAS 冲突
                                fas_silenced_flag.store(true, Ordering::Relaxed);

                                // 进游戏：释放 CLG 控制权，激活 FAS
                                cpu_governor.release();

                                let can_resume = fas_suspended_at.map_or(false, |at| {
                                    at.elapsed().as_secs() < FAS_SUSPEND_GRACE_SECS && fas_suspended_package == package_name && !fas_controller.policies.is_empty()
                                });

                                if can_resume {
                                    fas_suspended_at = None;
                                    fas_suspended_package.clear();
                                    for policy in &mut fas_controller.policies { policy.force_reapply(); }
                                } else {
                                    fas_suspended_at = None;
                                    fas_suspended_package.clear();
                                    fas_controller.load_policies(&current_rules.fas_rules);
                                }
                                fas_controller.set_game(pid, &package_name);
                                fas_controller.set_temperature(temperature);
                                fas_controller.set_temp_threshold(current_rules.fas_rules.core_temp_threshold);
                            } else {
                                // 退出 FAS 模式：恢复 TouchBoost 提频能力
                                fas_silenced_flag.store(false, Ordering::Relaxed);

                                // 退游戏：尝试挂起 FAS，并激活普通模式
                                if fas_suspended_at.is_some() {
                                    fas_controller.reset_all_freqs();
                                    fas_controller.clear_game();
                                    fas_controller.policies.clear();
                                    fas_suspended_at = None;
                                    fas_suspended_package.clear();
                                }

                                if old_mode == "fas" && !fas_controller.policies.is_empty() {
                                    fas_suspended_at = Some(Instant::now());
                                    fas_suspended_package = package_name.clone();
                                } else if old_mode == "fas" {
                                    fas_controller.clear_game();
                                    fas_controller.policies.clear();
                                    fas_suspended_at = None;
                                    fas_suspended_package.clear();
                                }

                                // 仅在亮屏时处理 CLG。如果息屏，Doze 配置仍在生效，这里不能覆盖它
                                if is_screen_on {
                                    let config_lock = config_clone.read().unwrap_or_else(|e| e.into_inner());
                                    let clg_cfg = get_clg_cfg(&config_lock, &mode);
                                    if clg_cfg.enabled {
                                        if cpu_governor.is_active() { cpu_governor.reload_config(&clg_cfg); }
                                        else { cpu_governor.init_policies(&clg_cfg); }
                                    } else {
                                        cpu_governor.release();
                                    }
                                }
                            }
                        } else if mode == "fas" {
                            fas_controller.set_temperature(temperature);
                        }
                    },

                    // --- 3. CPU 负载事件 (eBPF 驱动) ---
                    DaemonEvent::SystemLoadUpdate { core_utils, foreground_max_util } => {
                        let current_mode = mode_clone.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        // 仅当亮屏且在 FAS 模式且未挂起时，投喂 FAS
                        if is_screen_on && current_mode == "fas" && fas_suspended_at.is_none() {
                            fas_controller.update_cpu_util(foreground_max_util);
                            fas_controller.update_core_utils(&core_utils);
                        }
                        // 如果 CLG 处于活动状态（包含日常模式或息屏 Doze 模式），全权投喂
                        if cpu_governor.is_active() {
                            cpu_governor.on_load_update(&core_utils);
                        }

                        // CPU 静止下潜：投喂系统平均负载（息屏 DozeDiving 状态下由状态机忽略）。
                        // 数据缺失时跳过而非合成 0.0，避免把 eBPF 故障误判为"零负载"触发下潜
                        if !core_utils.is_empty() {
                            let avg_util =
                                core_utils.iter().sum::<f32>() / core_utils.len() as f32;
                            idle_dive.update(avg_util);
                        }
                    },

                    // --- 4. 帧率事件 (eBPF 驱动) ---
                    DaemonEvent::FrameUpdate { frame_delta_ns } => {
                        if !is_screen_on { continue; } // 息屏不处理渲染帧

                        let current_mode = mode_clone.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if current_mode == "fas" {
                            if !temp_sensor_path.is_empty() && last_temp_update.elapsed().as_secs() >= 3 {
                                if let Ok(raw_temp) = crate::utils::read_f64_from_file(&temp_sensor_path) {
                                    fas_controller.set_temperature(raw_temp / 1000.0);
                                }
                                last_temp_update = Instant::now();
                            }
                            fas_controller.update_frame(frame_delta_ns);
                        }
                    }

                    // --- 5. 热重载配置事件 ---
                    DaemonEvent::ConfigReload(new_rules) => {
                        current_rules = new_rules;
                        let current_mode = mode_clone.lock().unwrap_or_else(|e| e.into_inner()).clone();

                        if current_mode == "fas" {
                            if fas_controller.policies.is_empty() {
                                fas_controller.load_policies(&current_rules.fas_rules);
                            } else {
                                fas_controller.reload_rules(&current_rules.fas_rules);
                            }
                        } else if is_screen_on { // 息屏时不要用新配置覆盖 Doze
                            let config_lock = config_clone.read().unwrap_or_else(|e| e.into_inner());
                            let clg_cfg = get_clg_cfg(&config_lock, &current_mode);
                            if clg_cfg.enabled {
                                if cpu_governor.is_active() { cpu_governor.reload_config(&clg_cfg); }
                                else { cpu_governor.init_policies(&clg_cfg); }
                            } else if cpu_governor.is_active() {
                                cpu_governor.release();
                            }
                        }
                    }
                }

                // 定期检查 FAS 挂起状态是否超时
                if let Some(suspended_at) = fas_suspended_at {
                    if suspended_at.elapsed().as_secs() >= FAS_SUSPEND_GRACE_SECS {
                        fas_controller.reset_all_freqs();
                        fas_controller.clear_game();
                        fas_controller.policies.clear();
                        fas_suspended_at = None;
                        fas_suspended_package.clear();
                    }
                }

                // 非阻塞处理 TouchBoost 事件（监听线程通过 channel 推送）
                loop {
                    match touch_rx.try_recv() {
                        Ok(_touching) => {
                            // FAS 联动：fas_silenced_flag 由调度器在模式切换时设置，
                            // TouchBoost 监听线程读取标志并自动静默提频，避免与 FAS 冲突
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            if !touch_channel_disconnected_warned {
                                log::warn!("{}", t("touch-boost-channel-disconnected"));
                                touch_channel_disconnected_warned = true;
                            }
                            break;
                        }
                    }
                }
            }
            log::warn!("{}", t("scheduler-channel-closed"));

            })); // end catch_unwind

            if let Err(e) = result {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                log::error!("{}", t_with_args("scheduler-ipc-panicked",
                    &fluent_args!("error" => msg)));
            }
        })?;

    Ok(())
}
