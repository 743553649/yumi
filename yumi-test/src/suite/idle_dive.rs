use anyhow::Result;
use std::time::Instant;

use crate::{TestResult, TestStatus};

pub fn run() -> Result<Vec<TestResult>> {
    let mut results = Vec::new();
    results.push(test_check_idle_dive_config()?);
    Ok(results)
}

fn test_check_idle_dive_config() -> Result<TestResult> {
    let start = Instant::now();
    let config_path = "/data/adb/yumi/config/rules.yaml";
    let alt_path = "/data/local/tmp/yumi/rules.yaml";

    let path = if std::path::Path::new(config_path).exists() {
        config_path
    } else if std::path::Path::new(alt_path).exists() {
        alt_path
    } else {
        return Ok(TestResult {
            module: "idle_dive".to_string(),
            name: "check_idle_dive_config".to_string(),
            status: TestStatus::Skip("Config file not found".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
        });
    };

    let content = std::fs::read_to_string(path)?;
    let status = if content.contains("idle_dive") || content.contains("still_dive") {
        TestStatus::Pass
    } else {
        TestStatus::Skip("Idle dive config not found (may be disabled)".to_string())
    };

    Ok(TestResult {
        module: "idle_dive".to_string(),
        name: "check_idle_dive_config".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
