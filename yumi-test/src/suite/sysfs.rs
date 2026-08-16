use anyhow::Result;
use std::time::Instant;

use crate::{TestResult, TestStatus};

pub fn run() -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    results.push(test_check_cpu_freq_nodes()?);
    results.push(test_check_governor_node()?);
    results.push(test_check_cpuidle_node()?);

    Ok(results)
}

fn test_check_cpu_freq_nodes() -> Result<TestResult> {
    let start = Instant::now();
    let mut missing = Vec::new();

    for policy in 0..8 {
        let path = format!("/sys/devices/system/cpu/cpufreq/policy{}/scaling_cur_freq");
        if !std::path::Path::new(&path).exists() {
            // Try to find at least one policy
            if policy == 0 {
                missing.push(path);
            }
        }
    }

    // Check if at least one CPU freq node exists
    let has_any = (0..8).any(|i| {
        std::path::Path::new(&format!(
            "/sys/devices/system/cpu/cpufreq/policy{}/scaling_cur_freq",
            i
        ))
        .exists()
    });

    let status = if has_any {
        TestStatus::Pass
    } else {
        TestStatus::Fail("No CPU frequency nodes found".to_string())
    };

    Ok(TestResult {
        module: "sysfs".to_string(),
        name: "check_cpu_freq_nodes".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_check_governor_node() -> Result<TestResult> {
    let start = Instant::now();
    let path = "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor";
    let status = if std::path::Path::new(path).exists() {
        TestStatus::Pass
    } else {
        TestStatus::Fail("Governor node not found".to_string())
    };
    Ok(TestResult {
        module: "sysfs".to_string(),
        name: "check_governor_node".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn test_check_cpuidle_node() -> Result<TestResult> {
    let start = Instant::now();
    let path = "/sys/devices/system/cpu/cpuidle/current_governor";
    let status = if std::path::Path::new(path).exists() {
        TestStatus::Pass
    } else {
        TestStatus::Skip("cpuidle governor node not found (may not be available)".to_string())
    };
    Ok(TestResult {
        module: "sysfs".to_string(),
        name: "check_cpuidle_node".to_string(),
        status,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
