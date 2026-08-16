use anyhow::Result;
use std::time::Instant;

use crate::{TestResult, TestStatus};

pub fn run() -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    results.push(test_check_fas_config()?);
    results.push(test_check_fps_gears()?);

    Ok(results)
}

fn test_check_fas_config() -> Result<TestResult> {
    let start = Instant::now();
    let config_path = "/data/adb/yumi/config/rules.yaml";
    let alt_path = "/data/local/tmp/yumi/rules.yaml";

    let path = if std::path::Path::new(config_path).exists() {
        config_path
    } else if std::path::Path::new(alt_path).exists() {
        alt_path
    } else {
        return Ok(TestResult {
            module: "fas".to_string(),
            name: "check_fas_config".to_string(),
            status: TestStatus::Skip("Config file not found".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    };

    let content = std::fs::read_to_string(path)?;
    let status = if content.contains("fps_gears") || content.contains("pid") {
        TestStatus::Pass
    } else {
        TestStatus::Fail("FAS config fields not found".to_string())
    };

    Ok(TestResult {
        module: "fas".to_string(),
        name: "check_fas_config".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_check_fps_gears() -> Result<TestResult> {
    let start = Instant::now();
    // Read logcat for FAS gear switch events
    let status = match std::process::Command::new("logcat")
        .args(["-d", "-t", "500", "-s", "yumi:*"])
        .output()
    {
        Ok(output) => {
            let log = String::from_utf8_lossy(&output.stdout);
            if log.contains("gear") || log.contains("FAS") {
                TestStatus::Pass
            } else {
                TestStatus::Skip("No FAS events in logcat".to_string())
            }
        }
        Err(e) => TestStatus::Skip(format!("Cannot read logcat: {}", e)),
    };

    Ok(TestResult {
        module: "fas".to_string(),
        name: "check_fps_gears".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
