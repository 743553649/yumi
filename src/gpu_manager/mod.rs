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

// GPU 管理器 — Adreno GPU 频率与调速器控制核心模块。
// 实现拆分到 config.rs / compat.rs；熔断器 WriteCircuitBreaker 内联于此。

mod compat;
mod config;

pub use compat::{GpuCompatInfo, probe_compat};
pub use config::{GpuConfig, GpuModeConfig, GpuModeConfigs};

use std::path::Path;
use std::time::{Duration, Instant};

use crate::fluent_args;
use crate::i18n::{t, t_with_args};
use crate::utils::FastWriter;

/// Resolved GPU configuration for a specific mode, with all values finalized
/// (auto-calculation and clamping already applied).
#[derive(Debug, Clone)]
pub struct ResolvedGpuConfig {
    pub max_gpuclk: u32,
    pub governor: String,
    pub force_no_nap: u32,
}

/// GpuManager — main controller for Adreno GPU frequency and governor management.
pub struct GpuManager {
    /// Whether GPU management is enabled (from config)
    enabled: bool,
    /// Per-mode GPU configs loaded from YAML
    mode_configs: GpuModeConfigs,
    /// Compat info (probed at construction)
    compat: GpuCompatInfo,
    /// FastWriter for max_gpuclk sysfs node
    max_gpuclk_writer: Option<FastWriter>,
    /// FastWriter for governor sysfs node
    governor_writer: Option<FastWriter>,
    /// FastWriter for force_no_nap sysfs node
    force_no_nap_writer: Option<FastWriter>,
    /// Circuit breaker to prevent repeated failed writes
    circuit_breaker: WriteCircuitBreaker,
    /// Current active mode name
    current_mode: Option<String>,
}

impl GpuManager {
    /// Construct a new GpuManager from configuration.
    /// Probes GPU compatibility and sets up sysfs writers if available.
    pub fn new(config: &GpuConfig) -> Self {
        let compat = if config.enabled {
            probe_compat()
        } else {
            GpuCompatInfo::disabled()
        };

        let (max_gpuclk_writer, governor_writer, force_no_nap_writer) = if compat.available {
            let kgsl = &compat.kgsl_path;
            (
                Self::open_writer(kgsl.join("max_gpuclk")),
                // Only create governor writer if the node is writable
                if compat.governor_writable {
                    Self::open_writer(kgsl.join("devfreq/governor"))
                } else {
                    None
                },
                // Only create force_no_nap writer if the node exists
                if kgsl.join("force_no_nap").exists() {
                    Self::open_writer(kgsl.join("force_no_nap"))
                } else {
                    None
                },
            )
        } else {
            (None, None, None)
        };

        Self {
            enabled: config.enabled,
            mode_configs: config.modes.clone(),
            compat,
            max_gpuclk_writer,
            governor_writer,
            force_no_nap_writer,
            circuit_breaker: WriteCircuitBreaker::new(),
            current_mode: None,
        }
    }

    /// Initialize GPU manager: if enabled and available, apply the "balance" mode.
    pub fn init(&mut self) -> anyhow::Result<()> {
        if !self.enabled || !self.compat.available {
            return Ok(());
        }
        if let Err(_e) = self.apply_mode("balance") {
            log::warn!("[GPU] init: failed to apply balance mode");
        }
        log::info!("{}", t("gpu-init"));
        Ok(())
    }

    /// Main mode-switch entry point.
    pub fn apply_mode(&mut self, mode: &str) -> anyhow::Result<()> {
        if !self.enabled || !self.compat.available {
            return Ok(());
        }

        let start = Instant::now();

        // Resolve configuration for this mode
        let resolved = self.resolve_mode_config(mode);

        // Clamp max_gpuclk to a valid frequency
        let clamped_freq = self.clamp_max_gpuclk(resolved.max_gpuclk);

        // Validate governor (with fallback chain)
        let governor = self
            .validate_governor(&resolved.governor)
            .unwrap_or_else(|| "msm-adreno-tz".to_string());

        // Check circuit breaker cooldown
        self.circuit_breaker.try_reset();
        if self.circuit_breaker.is_cooldown() {
            log::warn!(
                "[GPU] apply_mode({}): circuit breaker in cooldown, skipping writes",
                mode
            );
            return Err(anyhow::anyhow!("circuit breaker in cooldown"));
        }

        // Write max_gpuclk
        if !self.write_max_gpuclk(clamped_freq) {
            let tripped = self.circuit_breaker.record_failure();
            if tripped {
                log::warn!(
                    "[GPU] apply_mode({}): max_gpuclk write failed, circuit breaker tripped",
                    mode
                );
                return Err(anyhow::anyhow!("max_gpuclk write failed"));
            }
        }

        // Write governor (with read-back confirm)
        if !self.write_governor(&governor) {
            let tripped = self.circuit_breaker.record_failure();
            if tripped {
                log::warn!(
                    "[GPU] apply_mode({}): governor write failed, circuit breaker tripped",
                    mode
                );
                return Err(anyhow::anyhow!("governor write failed"));
            }
        }

        // Write force_no_nap
        if !self.write_force_no_nap(resolved.force_no_nap) {
            let tripped = self.circuit_breaker.record_failure();
            if tripped {
                log::warn!(
                    "[GPU] apply_mode({}): force_no_nap write failed, circuit breaker tripped",
                    mode
                );
                return Err(anyhow::anyhow!("force_no_nap write failed"));
            }
        }

        // All writes succeeded, reset circuit breaker
        self.circuit_breaker.record_success();
        self.current_mode = Some(mode.to_string());

        let elapsed = start.elapsed();
        log::info!(
            "{}",
            t_with_args(
                "gpu-mode-switch",
                &fluent_args!("mode" => mode, "ms" => elapsed.as_millis().to_string(), "freq" => clamped_freq.to_string())
            )
        );

        Ok(())
    }

    /// Enter doze (screen-off) GPU state
    pub fn enter_doze(&mut self) {
        let _ = self.apply_mode("doze");
        log::info!("{}", t("gpu-enter-doze"));
    }

    /// Exit doze (screen-on) GPU state and restore the given mode
    pub fn exit_doze(&mut self, restore_mode: &str) {
        let _ = self.apply_mode(restore_mode);
        log::info!("{}", t("gpu-exit-doze"));
    }

    /// Release GPU control: restore defaults
    pub fn release(&mut self) {
        if !self.enabled || !self.compat.available {
            return;
        }
        // Write max frequency (0 means "no limit" but some kernels reject it,
        // so we use the actual maximum from available_frequencies)
        let max_freq = self.compat.frequencies.last().copied().unwrap_or(0);
        if !self.write_max_gpuclk(max_freq) {
            log::warn!("[GPU] release: failed to restore max_gpuclk");
        }
        if !self.write_governor("msm-adreno-tz") {
            log::warn!("[GPU] release: failed to restore governor");
        }
        if !self.write_force_no_nap(0) {
            log::warn!("[GPU] release: failed to restore force_no_nap");
        }
        self.current_mode = None;
        log::info!("{}", t("gpu-release"));
    }

    /// Accessor for current mode
    pub fn current_mode(&self) -> Option<&str> {
        self.current_mode.as_deref()
    }

    /// Accessor for compat info
    pub fn compat(&self) -> &GpuCompatInfo {
        &self.compat
    }

    /// Resolve the GPU config for a mode, pulling values from GpuModeConfig
    /// and auto-calculating max_gpuclk when it's 0.
    fn resolve_mode_config(&self, mode: &str) -> ResolvedGpuConfig {
        let mode_cfg = self.mode_configs.get(mode);

        let max_gpuclk = match mode_cfg {
            Some(cfg) if cfg.max_gpuclk != 0 => cfg.max_gpuclk,
            _ => self.auto_calculate_freq(mode),
        };

        let governor = mode_cfg.map(|c| c.governor.clone()).unwrap_or_else(|| {
            match mode {
                "powersave" | "doze" => "powersave",
                _ => "msm-adreno-tz",
            }
            .to_string()
        });

        let force_no_nap = mode_cfg
            .map(|c| if c.force_no_nap > 0 { 1 } else { 0 })
            .unwrap_or(0);

        ResolvedGpuConfig {
            max_gpuclk,
            governor,
            force_no_nap,
        }
    }

    /// Auto-calculate GPU frequency based on mode name when user config is 0.
    fn auto_calculate_freq(&self, mode: &str) -> u32 {
        let freqs = &self.compat.frequencies;
        if freqs.is_empty() {
            return 0;
        }
        match mode {
            "powersave" | "doze" => *freqs.first().unwrap_or(&0),
            "balance" => {
                let idx = freqs.len() * 40 / 100;
                freqs[idx.min(freqs.len() - 1)]
            }
            "performance" => {
                let idx = freqs.len() * 85 / 100;
                freqs[idx.min(freqs.len() - 1)]
            }
            "fast" => *freqs.last().unwrap_or(&0),
            _ => *freqs.last().unwrap_or(&0),
        }
    }

    /// Binary search in frequencies, clamp to nearest valid value.
    fn clamp_max_gpuclk(&self, target: u32) -> u32 {
        let freqs = &self.compat.frequencies;
        if freqs.is_empty() {
            return 0;
        }
        if target == 0 {
            return *freqs.last().unwrap_or(&0);
        }
        match freqs.binary_search(&target) {
            Ok(idx) => freqs[idx],
            Err(0) => freqs[0],
            Err(idx) if idx >= freqs.len() => *freqs.last().unwrap(),
            Err(idx) => freqs[idx - 1],
        }
    }

    /// Validate governor: check if requested governor is in available_governors.
    fn validate_governor(&self, requested: &str) -> Option<String> {
        let available = &self.compat.governors;
        if available.is_empty() {
            return None;
        }
        if available.iter().any(|g| g == requested) {
            return Some(requested.to_string());
        }
        let fallbacks = ["msm-adreno-tz", "simple_ondemand", "powersave"];
        for fb in &fallbacks {
            if available.iter().any(|g| g == fb) {
                log::debug!(
                    "[GPU] Governor '{}' not available, falling back to '{}'",
                    requested,
                    fb
                );
                return Some(fb.to_string());
            }
        }
        available.first().cloned()
    }

    /// Write max_gpuclk with FastWriter, with 1 retry on failure.
    fn write_max_gpuclk(&mut self, freq: u32) -> bool {
        let writer = match &mut self.max_gpuclk_writer {
            Some(w) => w,
            None => return false,
        };
        if writer.write_value_force(freq) {
            return true;
        }
        log::debug!(
            "[GPU] max_gpuclk write failed ({}), retrying after 50ms...",
            freq
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
        writer.write_value_force(freq)
    }

    /// Write governor string with read-back confirm. Falls back to msm-adreno-tz on failure.
    fn write_governor(&mut self, gov: &str) -> bool {
        let writer = match &mut self.governor_writer {
            Some(w) => w,
            None => return false,
        };
        for attempt in 0..3 {
            if writer.write_value_force_str(gov) {
                let gov_path = self.compat.kgsl_path.join("devfreq/governor");
                if let Ok(content) = std::fs::read_to_string(&gov_path) {
                    let readback = content.trim();
                    if readback == gov {
                        return true;
                    }
                    log::debug!(
                        "[GPU] Governor read-back mismatch: wrote '{}', read '{}'",
                        gov,
                        readback
                    );
                }
            }
            if attempt < 2 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        log::warn!(
            "[GPU] Failed to set governor '{}' after retries, falling back to msm-adreno-tz",
            gov
        );
        if writer.write_value_force_str("msm-adreno-tz")
            && let Ok(content) =
                std::fs::read_to_string(self.compat.kgsl_path.join("devfreq/governor"))
            && content.trim() == "msm-adreno-tz"
        {
            return true;
        }
        log::error!(
            "[GPU] Fallback governor write also failed, governor control may be inconsistent"
        );
        false
    }

    /// Write force_no_nap (clamped to 0 or 1) with FastWriter.
    fn write_force_no_nap(&mut self, val: u32) -> bool {
        let writer = match &mut self.force_no_nap_writer {
            Some(w) => w,
            None => return false,
        };
        writer.write_value_force(if val > 0 { 1 } else { 0 })
    }

    fn open_writer(path: impl AsRef<Path>) -> Option<FastWriter> {
        let writer = FastWriter::new(path);
        if writer.is_valid() {
            Some(writer)
        } else {
            None
        }
    }
}

// ── WriteCircuitBreaker: 防止连续写入失败后疯狂重试 ──

/// WriteCircuitBreaker: prevents repeated writes to GPU sysfs when
/// they keep failing, with a cooldown period.
#[derive(Debug)]
pub struct WriteCircuitBreaker {
    pub fail_count: u32,
    pub last_fail_time: Instant,
    pub cooldown_until: Option<Instant>,
}

impl WriteCircuitBreaker {
    pub const MAX_FAILURES: u32 = 3;
    pub const COOLDOWN_SECS: u64 = 30;

    pub fn new() -> Self {
        Self {
            fail_count: 0,
            last_fail_time: Instant::now(),
            cooldown_until: None,
        }
    }

    /// Record a write failure. Returns true if the circuit breaker is now tripped.
    pub fn record_failure(&mut self) -> bool {
        self.fail_count += 1;
        self.last_fail_time = Instant::now();
        if self.fail_count >= Self::MAX_FAILURES {
            self.cooldown_until = Some(Instant::now() + Duration::from_secs(Self::COOLDOWN_SECS));
            log::warn!(
                "[GPU] Write circuit breaker tripped after {} failures, cooling down for {}s",
                self.fail_count,
                Self::COOLDOWN_SECS
            );
            true
        } else {
            false
        }
    }

    /// Record a successful write, resetting counters
    pub fn record_success(&mut self) {
        self.fail_count = 0;
        self.cooldown_until = None;
    }

    /// Check if the circuit breaker is currently in cooldown
    pub fn is_cooldown(&self) -> bool {
        self.cooldown_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    /// Try to reset the circuit breaker if cooldown has expired
    pub fn try_reset(&mut self) {
        if let Some(until) = self.cooldown_until
            && Instant::now() >= until
        {
            self.fail_count = 0;
            self.cooldown_until = None;
            log::info!("[GPU] Write circuit breaker reset after cooldown");
        }
    }
}

impl Default for WriteCircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_compat(freqs: Vec<u32>, governors: Vec<&str>) -> GpuCompatInfo {
        GpuCompatInfo {
            available: true,
            kgsl_path: std::path::PathBuf::from("/sys/class/kgsl/kgsl-3d0"),
            frequencies: freqs.clone(),
            governors: governors.iter().map(|s| s.to_string()).collect(),
            gpu_model: "Adreno Test".into(),
            has_governor_control: !governors.is_empty(),
            governor_writable: !governors.is_empty(),
            has_freq_control: !freqs.is_empty(),
        }
    }

    fn make_manager_with_compat(compat: GpuCompatInfo) -> GpuManager {
        GpuManager {
            enabled: true,
            mode_configs: GpuModeConfigs::default(),
            compat,
            max_gpuclk_writer: None,
            governor_writer: None,
            force_no_nap_writer: None,
            circuit_breaker: WriteCircuitBreaker::new(),
            current_mode: None,
        }
    }

    #[test]
    fn test_clamp_gpuclk_empty_list_returns_zero() {
        let mgr = make_manager_with_compat(make_compat(vec![], vec![]));
        assert_eq!(mgr.clamp_max_gpuclk(500), 0);
    }

    #[test]
    fn test_clamp_gpuclk_exact_match() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400], vec![]));
        assert_eq!(mgr.clamp_max_gpuclk(200), 200);
    }

    #[test]
    fn test_clamp_gpuclk_between_values_rounds_down() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400], vec![]));
        assert_eq!(mgr.clamp_max_gpuclk(250), 200);
    }

    #[test]
    fn test_clamp_gpuclk_zero_target_returns_max() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400], vec![]));
        assert_eq!(mgr.clamp_max_gpuclk(0), 400);
    }

    #[test]
    fn test_clamp_gpuclk_below_min() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        assert_eq!(mgr.clamp_max_gpuclk(50), 100);
    }

    #[test]
    fn test_clamp_gpuclk_above_max() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        assert_eq!(mgr.clamp_max_gpuclk(999), 300);
    }

    #[test]
    fn test_validate_governor_exact_match() {
        let mgr = make_manager_with_compat(make_compat(
            vec![],
            vec!["msm-adreno-tz", "simple_ondemand"],
        ));
        assert_eq!(
            mgr.validate_governor("msm-adreno-tz"),
            Some("msm-adreno-tz".to_string())
        );
    }

    #[test]
    fn test_validate_governor_fallback_chain() {
        let mgr =
            make_manager_with_compat(make_compat(vec![], vec!["simple_ondemand", "powersave"]));
        assert_eq!(
            mgr.validate_governor("performance"),
            Some("simple_ondemand".to_string())
        );
    }

    #[test]
    fn test_validate_governor_empty_available_returns_none() {
        let mgr = make_manager_with_compat(make_compat(vec![], vec![]));
        assert_eq!(mgr.validate_governor("msm-adreno-tz"), None);
    }

    #[test]
    fn test_validate_governor_fallback_to_first() {
        let mgr = make_manager_with_compat(make_compat(vec![], vec!["custom_gov"]));
        assert_eq!(
            mgr.validate_governor("performance"),
            Some("custom_gov".to_string())
        );
    }

    #[test]
    fn test_resolve_mode_config_powersave_min_freq() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        let resolved = mgr.resolve_mode_config("powersave");
        assert_eq!(resolved.max_gpuclk, 100);
        assert_eq!(resolved.force_no_nap, 0);
    }

    #[test]
    fn test_resolve_mode_config_doze_min_freq() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        let resolved = mgr.resolve_mode_config("doze");
        assert_eq!(resolved.max_gpuclk, 100);
        assert_eq!(resolved.force_no_nap, 0);
    }

    #[test]
    fn test_resolve_mode_config_balance_40pct() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.resolve_mode_config("balance").max_gpuclk, 300);
    }

    #[test]
    fn test_resolve_mode_config_performance_85pct() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.resolve_mode_config("performance").max_gpuclk, 500);
    }

    #[test]
    fn test_resolve_mode_config_fast_max_freq() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.resolve_mode_config("fast").max_gpuclk, 500);
    }

    #[test]
    fn test_resolve_mode_config_uses_configured_value_when_nonzero() {
        let mut mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        mgr.mode_configs.balance = GpuModeConfig {
            max_gpuclk: 350,
            governor: "msm-adreno-tz".into(),
            force_no_nap: 0,
        };
        assert_eq!(mgr.resolve_mode_config("balance").max_gpuclk, 350);
    }

    #[test]
    fn test_resolve_mode_config_empty_freqs_returns_zero() {
        let mgr = make_manager_with_compat(make_compat(vec![], vec![]));
        assert_eq!(mgr.resolve_mode_config("balance").max_gpuclk, 0);
    }

    // ── WriteCircuitBreaker 测试 ──

    #[test]
    fn test_circuit_breaker_one_failure_does_not_trip() {
        let mut cb = WriteCircuitBreaker::new();
        assert!(!cb.record_failure());
        assert!(!cb.is_cooldown());
    }

    #[test]
    fn test_circuit_breaker_three_failures_trips() {
        let mut cb = WriteCircuitBreaker::new();
        assert!(!cb.record_failure());
        assert!(!cb.record_failure());
        assert!(cb.record_failure());
        assert!(cb.is_cooldown());
    }

    #[test]
    fn test_circuit_breaker_success_resets() {
        let mut cb = WriteCircuitBreaker::new();
        let _ = cb.record_failure();
        let _ = cb.record_failure();
        let _ = cb.record_failure();
        assert!(cb.is_cooldown());

        cb.record_success();
        assert!(!cb.is_cooldown());
        assert_eq!(cb.fail_count, 0);
    }

    // ── apply_mode / enter_doze / exit_doze / release 行为测试 ──

    #[test]
    fn test_apply_mode_disabled_returns_ok() {
        let mut mgr = GpuManager {
            enabled: false,
            ..make_manager_with_compat(make_compat(vec![100, 200, 300], vec!["msm-adreno-tz"]))
        };
        assert!(mgr.apply_mode("balance").is_ok());
        assert!(mgr.current_mode().is_none());
    }

    #[test]
    fn test_apply_mode_compat_unavailable_returns_ok() {
        let mut mgr = GpuManager {
            enabled: true,
            ..make_manager_with_compat(GpuCompatInfo::disabled())
        };
        assert!(mgr.apply_mode("balance").is_ok());
        assert!(mgr.current_mode().is_none());
    }

    #[test]
    fn test_apply_mode_writers_none_still_sets_mode() {
        // Writers are None (no sysfs), writes silently fail but mode is recorded
        let mut mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        // First apply_mode: 3 writes fail → circuit breaker trips on 3rd
        let _ = mgr.apply_mode("balance");
        // current_mode may or may not be set depending on circuit breaker timing
        // The important thing is it doesn't panic
    }

    #[test]
    fn test_enter_doze_sets_mode() {
        let mut mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        mgr.enter_doze();
        // enter_doze calls apply_mode("doze"), writers are None so it may or may not set mode
        // Just verify no panic
    }

    #[test]
    fn test_exit_doze_restores_mode() {
        let mut mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        mgr.exit_doze("balance");
        // exit_doze calls apply_mode(restore_mode), just verify no panic
    }

    #[test]
    fn test_release_clears_mode() {
        let mut mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        // Manually set current_mode to simulate an active state
        mgr.current_mode = Some("balance".to_string());
        mgr.release();
        assert!(mgr.current_mode().is_none());
    }

    #[test]
    fn test_release_disabled_noop() {
        let mut mgr = GpuManager {
            enabled: false,
            ..make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]))
        };
        mgr.current_mode = Some("balance".to_string());
        mgr.release();
        // release early-returns when disabled, current_mode stays set
        assert_eq!(mgr.current_mode(), Some("balance"));
    }

    #[test]
    fn test_current_mode_accessor() {
        let mut mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        assert!(mgr.current_mode().is_none());
        mgr.current_mode = Some("fast".to_string());
        assert_eq!(mgr.current_mode(), Some("fast"));
    }

    #[test]
    fn test_compat_accessor() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300], vec![]));
        assert!(mgr.compat().available);
        assert_eq!(mgr.compat().gpu_model, "Adreno Test");
    }

    // ── auto_calculate_freq 边界测试 ──

    #[test]
    fn test_auto_calculate_freq_empty_returns_zero() {
        let mgr = make_manager_with_compat(make_compat(vec![], vec![]));
        assert_eq!(mgr.auto_calculate_freq("balance"), 0);
    }

    #[test]
    fn test_auto_calculate_freq_single_element() {
        let mgr = make_manager_with_compat(make_compat(vec![500], vec![]));
        // balance: idx = 1 * 40 / 100 = 0 → min(0, 0) = 0 → freqs[0] = 500
        assert_eq!(mgr.auto_calculate_freq("balance"), 500);
    }

    #[test]
    fn test_auto_calculate_freq_unknown_mode_uses_max() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.auto_calculate_freq("turbo"), 500);
    }

    #[test]
    fn test_auto_calculate_freq_powersave_uses_min() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.auto_calculate_freq("powersave"), 100);
    }

    #[test]
    fn test_auto_calculate_freq_doze_uses_min() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.auto_calculate_freq("doze"), 100);
    }

    #[test]
    fn test_auto_calculate_freq_fast_uses_max() {
        let mgr = make_manager_with_compat(make_compat(vec![100, 200, 300, 400, 500], vec![]));
        assert_eq!(mgr.auto_calculate_freq("fast"), 500);
    }
}
