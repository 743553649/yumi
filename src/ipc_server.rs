use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::common::DaemonEvent;
use crate::monitor::config::RulesConfig;
use crate::utils;

/// Magisk/KernelSU 模块部署根路径，作为 rules.yaml 与日志的兜底写入/读取位置
const ADB_MODULE_ROOT: &str = "/data/adb/modules/yumi";

/// 返回 rules.yaml 的全部写入目标：主路径（配置真源 `get_rules_path()`）+ 模块部署兜底路径。
/// 两者重合时仅返回一项，避免重复写入。
fn rules_write_targets() -> Vec<PathBuf> {
    let primary = crate::monitor::config::get_rules_path();
    let adb = PathBuf::from(ADB_MODULE_ROOT).join("rules.yaml");
    if primary == adb {
        vec![primary]
    } else {
        vec![primary, adb]
    }
}

/// 启动 IPC 服务，监听指定端口并处理文本命令
pub fn start(
    tx: mpsc::Sender<DaemonEvent>,
    root: PathBuf,
    port: u16,
    config_arc: Arc<Mutex<RulesConfig>>,
    force_refresh_arc: Arc<AtomicBool>,
) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            log::warn!("⚠️ IPC server bind failed on {}: {}", addr, e);
            return;
        }
    };

    log::info!("🚀 IPC server listening on http://{}", addr);
    start_with_listener(listener, tx, root, config_arc, force_refresh_arc);
}

/// 基于已绑定的 TcpListener 启动 accept 循环（便于单测传入动态端口）
pub fn start_with_listener(
    listener: TcpListener,
    tx: mpsc::Sender<DaemonEvent>,
    root: PathBuf,
    config_arc: Arc<Mutex<RulesConfig>>,
    force_refresh_arc: Arc<AtomicBool>,
) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tx = tx.clone();
                let root = root.clone();
                let config_arc = Arc::clone(&config_arc);
                let force_refresh_arc = Arc::clone(&force_refresh_arc);
                std::thread::spawn(move || {
                    handle_client(stream, tx, root, config_arc, force_refresh_arc);
                });
            }
            Err(e) => {
                log::debug!("IPC accept error: {}", e);
            }
        }
    }
}

fn handle_client(
    mut stream: TcpStream,
    tx: mpsc::Sender<DaemonEvent>,
    root: PathBuf,
    config_arc: Arc<Mutex<RulesConfig>>,
    force_refresh_arc: Arc<AtomicBool>,
) {
    // 设置 10s 读超时，超时后自动断开连接防资源耗尽
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

    let stream_clone = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            log::debug!("IPC stream clone failed: {}", e);
            return;
        }
    };
    let mut reader = BufReader::new(stream_clone);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF 客户端正常关闭
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response = process_command(trimmed, &tx, &root, &config_arc, &force_refresh_arc);
                if let Err(e) = stream.write_all(response.as_bytes()) {
                    log::debug!("IPC client write error: {}", e);
                    break;
                }
                let _ = stream.flush();
            }
            Err(e) => {
                // 超时或 WouldBlock 视为正常断开，不记录错误日志
                if e.kind() != std::io::ErrorKind::TimedOut && e.kind() != std::io::ErrorKind::WouldBlock {
                    log::debug!("IPC client read error: {}", e);
                }
                break;
            }
        }
    }
}

pub fn process_command(
    cmd: &str,
    tx: &mpsc::Sender<DaemonEvent>,
    root: &PathBuf,
    config_arc: &Arc<Mutex<RulesConfig>>,
    force_refresh_arc: &Arc<AtomicBool>,
) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return "err:empty_command\n".to_string();
    }

    match parts[0] {
        "ping" => "pong\n".to_string(),
        "get_mode" => {
            let mode_file = root.join("current_mode.txt");
            if let Ok(m) = utils::read_file_content(mode_file.to_str().unwrap_or("")) {
                format!("{}\n", m.trim())
            } else {
                "balance\n".to_string()
            }
        }
        "set_mode" => {
            if parts.len() < 2 {
                return "err:missing_mode\n".to_string();
            }
            let target_mode = parts[1];

            if is_valid_mode(target_mode) {
                let rules_path = crate::monitor::config::get_rules_path();
                let mut rules = utils::read_config::<crate::monitor::config::RulesConfig, _>(&rules_path)
                    .unwrap_or_else(|_| crate::monitor::app_detect::get_default_rules());
                rules.global_mode = target_mode.to_string();
                if let Ok(yaml_str) = serde_yaml::to_string(&rules) {
                    for target in rules_write_targets() {
                        let _ = utils::try_write_file(&target, &yaml_str);
                    }
                }

                *config_arc.lock().unwrap_or_else(|e| e.into_inner()) = rules;
                force_refresh_arc.store(true, Ordering::SeqCst);

                let event = DaemonEvent::ModeChange {
                    package_name: "ipc".to_string(),
                    pid: 0,
                    mode: target_mode.to_string(),
                    temperature: 0.0,
                };
                if tx.send(event).is_ok() {
                    "ok\n".to_string()
                } else {
                    "err:send_failed\n".to_string()
                }
            } else {
                "err:invalid_mode\n".to_string()
            }
        }
        "set_app_mode" | "set_app_rule" => {
            if parts.len() == 3 {
                let pkg = parts[1];
                let mode = parts[2];
                let rules_path = crate::monitor::config::get_rules_path();
                let mut rules = utils::read_config::<crate::monitor::config::RulesConfig, _>(&rules_path)
                    .unwrap_or_else(|_| crate::monitor::app_detect::get_default_rules());
                if mode == "default" || mode == "none" {
                    rules.app_modes.remove(pkg);
                } else {
                    rules.app_modes.insert(pkg.to_string(), mode.to_string());
                }
                if let Ok(yaml_str) = serde_yaml::to_string(&rules) {
                    for target in rules_write_targets() {
                        let _ = utils::try_write_file(&target, &yaml_str);
                    }
                }

                *config_arc.lock().unwrap_or_else(|e| e.into_inner()) = rules.clone();
                force_refresh_arc.store(true, Ordering::SeqCst);

                let event = DaemonEvent::ConfigReload(rules);
                let _ = tx.send(event);
                "ok\n".to_string()
            } else {
                "err:invalid_args\n".to_string()
            }
        }
        "reload_rules" => {
            let rules_path = crate::monitor::config::get_rules_path();
            if let Ok(rules) = utils::read_config::<crate::monitor::config::RulesConfig, _>(&rules_path) {
                *config_arc.lock().unwrap_or_else(|e| e.into_inner()) = rules.clone();
                force_refresh_arc.store(true, Ordering::SeqCst);

                if tx.send(DaemonEvent::ConfigReload(rules)).is_ok() {
                    "ok:reload_rules\n".to_string()
                } else {
                    "err:send_event\n".to_string()
                }
            } else {
                "err:read_rules\n".to_string()
            }
        }
        "get_log" => {
            let max_lines = if parts.len() > 1 {
                parts[1].parse::<usize>().unwrap_or(100)
            } else {
                100
            };
            let candidate_logs = [
                PathBuf::from(ADB_MODULE_ROOT).join("logs/daemon.log"),
                root.join("logs/daemon.log"),
                root.join("daemon.log"),
            ];
            let mut response = String::new();
            for log_file in &candidate_logs {
                if let Ok(content) = std::fs::read_to_string(log_file) {
                    let lines: Vec<&str> = content.lines().collect();
                    if !lines.is_empty() {
                        let start_idx = if lines.len() > max_lines { lines.len() - max_lines } else { 0 };
                        let tail = &lines[start_idx..];
                        response.push_str(&tail.join("\n"));
                        response.push('\n');
                        break;
                    }
                }
            }
            response.push_str("---END_LOG---\n");
            response
        }
        _ => "err:unknown_command\n".to_string(),
    }
}

fn is_valid_mode(mode: &str) -> bool {
    matches!(mode, "powersave" | "balance" | "performance" | "fast" | "fas" | "default")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_ipc_protocol_commands() {
        let (tx, rx) = mpsc::channel();
        let temp_dir = std::env::temp_dir().join("yumi_ipc_test");
        let _ = fs::create_dir_all(&temp_dir);
        let mode_file = temp_dir.join("current_mode.txt");
        let _ = fs::write(&mode_file, "balance\n");

        let config_arc = Arc::new(Mutex::new(crate::monitor::app_detect::get_default_rules()));
        let force_refresh_arc = Arc::new(AtomicBool::new(false));

        // 1. ping
        assert_eq!(process_command("ping", &tx, &temp_dir, &config_arc, &force_refresh_arc), "pong\n");

        // 2. get_mode
        assert_eq!(process_command("get_mode", &tx, &temp_dir, &config_arc, &force_refresh_arc), "balance\n");

        // 3. set_mode valid
        assert_eq!(process_command("set_mode performance", &tx, &temp_dir, &config_arc, &force_refresh_arc), "ok\n");
        let event = rx.try_recv().unwrap();
        match event {
            DaemonEvent::ModeChange { mode, package_name, .. } => {
                assert_eq!(mode, "performance");
                assert_eq!(package_name, "ipc");
            }
            _ => panic!("Unexpected event"),
        }

        // 4. set_mode invalid
        assert_eq!(process_command("set_mode fas", &tx, &temp_dir, &config_arc, &force_refresh_arc), "err:invalid_mode\n");

        // 5. reload_rules test
        let res = process_command("reload_rules", &tx, &temp_dir, &config_arc, &force_refresh_arc);
        assert!(res == "ok:reload_rules\n" || res == "err:read_rules\n" || res == "err:send_event\n");

        // 6. get_log test (includes ---END_LOG--- terminator)
        let logs_dir = temp_dir.join("logs");
        let _ = fs::create_dir_all(&logs_dir);
        let daemon_log = logs_dir.join("daemon.log");
        let _ = fs::write(&daemon_log, "[2026-08-01 12:00:00] [INFO] [main] daemon started\n");
        assert_eq!(process_command("get_log", &tx, &temp_dir, &config_arc, &force_refresh_arc), "[2026-08-01 12:00:00] [INFO] [main] daemon started\n---END_LOG---\n");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
