use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crate::common::DaemonEvent;
use crate::utils;

/// 启动 IPC 服务，监听指定端口并处理文本命令
pub fn start(tx: mpsc::Sender<DaemonEvent>, root: PathBuf, port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            log::warn!("IPC server bind failed on {}: {}", addr, e);
            return;
        }
    };

    log::info!("IPC server listening on http://{}", addr);
    start_with_listener(listener, tx, root);
}

/// 基于已绑定的 TcpListener 启动 accept 循环（便于单测传入动态端口）
pub fn start_with_listener(listener: TcpListener, tx: mpsc::Sender<DaemonEvent>, root: PathBuf) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tx = tx.clone();
                let root = root.clone();
                std::thread::spawn(move || {
                    handle_client(stream, tx, root);
                });
            }
            Err(e) => {
                log::debug!("IPC accept error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream, tx: mpsc::Sender<DaemonEvent>, root: PathBuf) {
    // 设置 10s 读超时，超时后自动断开连接防资源耗尽
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

    let mut reader = BufReader::new(stream.try_clone().unwrap_or_else(|_| stream.try_clone().unwrap()));

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF 客户端正常关闭
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response = process_command(trimmed, &tx, &root);
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

fn process_command(cmd: &str, tx: &mpsc::Sender<DaemonEvent>, root: &PathBuf) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return "err:unknown_command\n".to_string();
    }

    match parts[0] {
        "ping" => "pong\n".to_string(),
        "get_mode" => {
            let mode_file = root.join("current_mode.txt");
            if let Ok(content) = utils::read_file_content(mode_file.to_str().unwrap_or("")) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return format!("{}\n", trimmed);
                }
            }
            // 回退读取 rules.yaml 的 global_mode
            let rules_path = crate::monitor::config::get_rules_path();
            if let Ok(rules) = utils::read_config::<crate::monitor::config::RulesConfig, _>(&rules_path) {
                format!("{}\n", rules.global_mode.trim())
            } else {
                "err:read_mode\n".to_string()
            }
        }
        "set_mode" => {
            if parts.len() == 2 && is_valid_mode(parts[1]) {
                let target_mode = parts[1];

                // 1. 同步更新 rules.yaml 中的 global_mode，防止 app_detect 被覆盖重置
                let rules_path = crate::monitor::config::get_rules_path();
                let mut rules = utils::read_config::<crate::monitor::config::RulesConfig, _>(&rules_path)
                    .unwrap_or_else(|_| crate::monitor::app_detect::get_default_rules());
                rules.global_mode = target_mode.to_string();
                if let Ok(yaml_str) = serde_yaml::to_string(&rules) {
                    let _ = utils::try_write_file(&rules_path, &yaml_str);
                    let _ = utils::try_write_file("/storage/emulated/0/yumi/rules.yaml", &yaml_str);
                    let _ = utils::try_write_file("/storage/emulated/0/yumi/module/rules.yaml", &yaml_str);
                }

                // 2. 发送 ModeChange 事件触发即时调度改写与 current_mode.txt 落盘
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
        "set_app_mode" => {
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
                    let _ = utils::try_write_file(&rules_path, &yaml_str);
                    let _ = utils::try_write_file("/storage/emulated/0/yumi/rules.yaml", &yaml_str);
                    let _ = utils::try_write_file("/storage/emulated/0/yumi/module/rules.yaml", &yaml_str);
                    let _ = utils::try_write_file("/data/adb/modules/yumi/rules.yaml", &yaml_str);
                }
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
                let event = DaemonEvent::ConfigReload(rules);
                let _ = tx.send(event);
                "ok\n".to_string()
            } else {
                "err:reload_failed\n".to_string()
            }
        }
        "get_log" => {
            let max_lines = if parts.len() > 1 {
                parts[1].parse::<usize>().unwrap_or(100)
            } else {
                100
            };
            let candidate_logs = [
                PathBuf::from("/data/adb/modules/yumi/logs/daemon.log"),
                root.join("module/logs/daemon.log"),
                root.join("logs/daemon.log"),
                root.join("module/daemon.log"),
                root.join("daemon.log"),
                PathBuf::from("/storage/emulated/0/yumi/module/logs/daemon.log"),
                PathBuf::from("/storage/emulated/0/yumi/logs/daemon.log"),
                PathBuf::from("/storage/emulated/0/yumi/module/daemon.log"),
                PathBuf::from("/storage/emulated/0/yumi/daemon.log"),
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

        // 1. ping
        assert_eq!(process_command("ping", &tx, &temp_dir), "pong\n");

        // 2. get_mode
        assert_eq!(process_command("get_mode", &tx, &temp_dir), "balance\n");

        // 3. set_mode valid
        assert_eq!(process_command("set_mode performance", &tx, &temp_dir), "ok\n");
        let event = rx.try_recv().unwrap();
        match event {
            DaemonEvent::ModeChange { mode, package_name, .. } => {
                assert_eq!(mode, "performance");
                assert_eq!(package_name, "ipc");
            }
            _ => panic!("Unexpected event"),
        }

        // 4. set_mode invalid
        assert_eq!(process_command("set_mode fas", &tx, &temp_dir), "err:invalid_mode\n");

        // 5. get_log test (includes ---END_LOG--- terminator)
        let logs_dir = temp_dir.join("logs");
        let _ = fs::create_dir_all(&logs_dir);
        let daemon_log = logs_dir.join("daemon.log");
        let _ = fs::write(&daemon_log, "[2026-08-01 12:00:00] [INFO] [main] daemon started\n");
        assert_eq!(process_command("get_log", &tx, &temp_dir), "[2026-08-01 12:00:00] [INFO] [main] daemon started\n---END_LOG---\n");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
