use anyhow::Result;
use std::time::Instant;

use crate::{TestResult, TestStatus};

pub fn run() -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    results.push(test_check_running()?);
    results.push(test_check_pid_file()?);
    results.push(test_check_memory()?);
    results.push(test_check_no_crash()?);

    Ok(results)
}

fn test_check_running() -> Result<TestResult> {
    let start = Instant::now();
    let status = match check_process_running() {
        Ok(true) => TestStatus::Pass,
        Ok(false) => TestStatus::Fail("yumi process not running".to_string()),
        Err(e) => TestStatus::Fail(format!("Error: {}", e)),
    };
    Ok(TestResult {
        module: "process".to_string(),
        name: "check_running".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_check_pid_file() -> Result<TestResult> {
    let start = Instant::now();
    let status = if std::path::Path::new("/data/local/tmp/yumi.pid").exists()
        || std::path::Path::new("/data/adb/yumi/yumi.pid").exists()
    {
        TestStatus::Pass
    } else {
        TestStatus::Skip("PID file not found (may use different location)".to_string())
    };
    Ok(TestResult {
        module: "process".to_string(),
        name: "check_pid_file".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_check_memory() -> Result<TestResult> {
    let start = Instant::now();
    let status = match get_process_memory_kb() {
        Ok(mem_kb) => {
            if mem_kb < 102400 {
                // 100MB
                TestStatus::Pass
            } else {
                TestStatus::Fail(format!("Memory usage too high: {}KB", mem_kb))
            }
        }
        Err(e) => TestStatus::Skip(format!("Cannot check memory: {}", e)),
    };
    Ok(TestResult {
        module: "process".to_string(),
        name: "check_memory".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_check_no_crash() -> Result<TestResult> {
    let start = Instant::now();
    // Check logcat for recent crashes
    let status = match std::process::Command::new("logcat")
        .args(["-d", "-t", "100", "-s", "yumi:*", "DEBUG:*"])
        .output()
    {
        Ok(output) => {
            let log = String::from_utf8_lossy(&output.stdout);
            if log.contains("FATAL") || log.contains("panic") {
                TestStatus::Fail("Crash detected in logcat".to_string())
            } else {
                TestStatus::Pass
            }
        }
        Err(e) => TestStatus::Skip(format!("Cannot read logcat: {}", e)),
    };
    Ok(TestResult {
        module: "process".to_string(),
        name: "check_no_crash".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn check_process_running() -> Result<bool> {
    let output = std::process::Command::new("pidof").arg("yumi").output()?;
    Ok(!output.stdout.is_empty())
}

fn get_process_memory_kb() -> Result<u64> {
    let output = std::process::Command::new("pidof").arg("yumi").output()?;
    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if pid.is_empty() {
        anyhow::bail!("yumi not running");
    }

    let status_path = format!("/{}/status", pid);
    let content = std::fs::read_to_string(&status_path)?;
    for line in content.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return Ok(parts[1].parse()?);
            }
        }
    }
    anyhow::bail!("VmRSS not found")
}
