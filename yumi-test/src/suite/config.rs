use anyhow::Result;
use std::time::Instant;

use crate::{TestResult, TestStatus};

pub fn run() -> Result<Vec<TestResult>> {
    let mut results = Vec::new();
    results.push(test_read_config()?);
    results.push(test_validate_yaml()?);
    Ok(results)
}

fn test_read_config() -> Result<TestResult> {
    let start = Instant::now();
    let config_path = "/data/adb/yumi/config/rules.yaml";
    let alt_path = "/data/local/tmp/yumi/rules.yaml";

    let path = if std::path::Path::new(config_path).exists() {
        Some(config_path)
    } else if std::path::Path::new(alt_path).exists() {
        Some(alt_path)
    } else {
        None
    };

    let status = match path {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(_) => TestStatus::Pass,
            Err(e) => TestStatus::Fail(format!("Cannot read config: {}", e)),
        },
        None => TestStatus::Skip("Config file not found".to_string()),
    };

    Ok(TestResult {
        module: "config".to_string(),
        name: "read_current_config".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_validate_yaml() -> Result<TestResult> {
    let start = Instant::now();
    let config_path = "/data/adb/yumi/config/rules.yaml";
    let alt_path = "/data/local/tmp/yumi/rules.yaml";

    let path = if std::path::Path::new(config_path).exists() {
        Some(config_path)
    } else if std::path::Path::new(alt_path).exists() {
        Some(alt_path)
    } else {
        None
    };

    let status = match path {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(content) => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                Ok(_) => TestStatus::Pass,
                Err(e) => TestStatus::Fail(format!("Invalid YAML: {}", e)),
            },
            Err(e) => TestStatus::Fail(format!("Cannot read config: {}", e)),
        },
        None => TestStatus::Skip("Config file not found".to_string()),
    };

    Ok(TestResult {
        module: "config".to_string(),
        name: "validate_config_format".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
