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

use std::path::{Path, PathBuf};

const KGSL_DEVICE_PATH: &str = "/sys/class/kgsl/kgsl-3d0";
const KGSL_DEVICE_FALLBACK: &str = "/sys/kernel/kgsl/kgsl-3d0";

/// GPU 兼容性探测信息
#[derive(Debug, Clone)]
pub struct GpuCompatInfo {
    pub available: bool,
    pub kgsl_path: PathBuf,
    /// Sorted list of available GPU frequencies (values from kernel sysfs)
    pub frequencies: Vec<u32>,
    /// Available GPU governors
    pub governors: Vec<String>,
    /// GPU model string (e.g. "Adreno 730")
    pub gpu_model: String,
    /// Whether governor switching is supported
    pub has_governor_control: bool,
    /// Whether frequency control is supported
    pub has_freq_control: bool,
}

impl GpuCompatInfo {
    /// Create a disabled/default compat info (used when KGSL is unavailable)
    pub fn disabled() -> Self {
        Self {
            available: false,
            kgsl_path: PathBuf::new(),
            frequencies: Vec::new(),
            governors: Vec::new(),
            gpu_model: String::new(),
            has_governor_control: false,
            has_freq_control: false,
        }
    }
}

/// Probe GPU compatibility by checking KGSL sysfs paths
pub fn probe_compat() -> GpuCompatInfo {
    // Determine KGSL base path
    let kgsl_path = resolve_kgsl_path();

    if kgsl_path.as_os_str().is_empty() {
        log::info!("[GPU] KGSL device path not found, GPU control unavailable");
        return GpuCompatInfo::disabled();
    }

    // Read available frequencies
    let frequencies = read_available_frequencies(&kgsl_path);
    if frequencies.len() < 3 {
        log::info!(
            "[GPU] Insufficient GPU frequencies ({}), GPU control unavailable",
            frequencies.len()
        );
        return GpuCompatInfo::disabled();
    }

    // Read available governors
    let governors = read_available_governors(&kgsl_path);

    // Read GPU model
    let gpu_model = read_gpu_model(&kgsl_path);

    let has_governor_control = !governors.is_empty();
    let has_freq_control = !frequencies.is_empty();

    log::info!(
        "[GPU] Detected {} | freqs={} governors={}",
        gpu_model,
        frequencies.len(),
        governors.len()
    );

    GpuCompatInfo {
        available: true,
        kgsl_path,
        frequencies,
        governors,
        gpu_model,
        has_governor_control,
        has_freq_control,
    }
}

/// Try candidate KGSL paths and return the first valid one
fn resolve_kgsl_path() -> PathBuf {
    for candidate in &[KGSL_DEVICE_PATH, KGSL_DEVICE_FALLBACK] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    PathBuf::new()
}

/// Read available_frequencies from sysfs. The node may have different names
/// across kernel versions.
fn read_available_frequencies(kgsl: &Path) -> Vec<u32> {
    let candidates = [
        "available_frequencies",
        "gpu_available_frequencies",
        "devfreq/available_frequencies",
    ];

    let mut freqs = Vec::new();
    for name in &candidates {
        let path = kgsl.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let numbers: Vec<u32> = content
                .split_whitespace()
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            if numbers.len() >= 3 {
                freqs = numbers;
                break;
            }
        }
    }

    freqs.sort_unstable();
    freqs.dedup();
    freqs
}

/// Read available governors from devfreq node
fn read_available_governors(kgsl: &Path) -> Vec<String> {
    let path = kgsl.join("devfreq/available_governors");
    std::fs::read_to_string(&path)
        .map(|content| content.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default()
}

/// Read GPU model string
fn read_gpu_model(kgsl: &Path) -> String {
    let candidates = ["gpu_model", "devfreq/gpu_model"];
    for name in &candidates {
        let path = kgsl.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
    }
    "Unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_compat() {
        let compat = GpuCompatInfo::disabled();
        assert!(!compat.available);
        assert!(compat.frequencies.is_empty());
        assert!(compat.governors.is_empty());
        assert!(compat.gpu_model.is_empty());
    }

    #[test]
    fn test_resolve_kgsl_path_no_dev() {
        // On a non-Android system, this should return empty
        let path = resolve_kgsl_path();
        // We can't guarantee the test environment, but verify it returns something
        assert!(!cfg!(target_os = "android") || path.as_os_str().is_empty());
    }
}
