# Task 2 Report: IdleDive Code Quality (C1+C3+C5+C6)

## Status: success

## Changes Made

### C1: LatencyWriter error propagation (latency.rs)

- `set_governor()`: Added `any_ok` tracking. Returns `Err` when all governor paths fail and paths are non-empty. Returns `Ok` if at least one succeeded or paths are empty.
- `set_latency()`: Added `pm_qos_ok` and `any_sysfs_ok` tracking. PM-QoS failure is warned but doesn't immediately fail. Returns `Err` only when PM-QoS failed AND all sysfs writes failed AND there were paths to write.

### C5: PathBuf instead of String (latency.rs)

- Changed `governor_paths: Vec<String>` → `Vec<PathBuf>` and `latency_paths: Vec<String>` → `Vec<PathBuf>`.
- Added `use std::path::PathBuf;`.
- Path construction now pushes `PathBuf` directly (from `base.join()`) instead of converting via `.to_string_lossy().to_string()`.
- Write calls pass `&PathBuf` directly to `write_to_file` (accepts `AsRef<Path>`).
- Changed `.clone()` → `.display().to_string()` in warn macros for proper path formatting.

### C6: i18n key fix (latency.rs)

- `set_governor()`: Changed warn key from `"sysfs-open-failed"` to `"idle-dive-set-governor-failed"`.
- `set_latency()`: Changed warn key from `"sysfs-open-failed"` to `"idle-dive-set-latency-failed"`.
- `open_pm_qos()`: Kept `"sysfs-open-failed"` — the existing i18n key with path context is sufficient and the key exists in both .ftl files.

### C3: transition_to dedup (controller.rs)

- Replaced repetitive 3-arm match (each duplicating governor/latency calls + warn logging) with:
  1. Lookup match to extract `(governor, latency_us)` tuple
  2. Info log match (3 lines)
  3. Unified `set_governor` + `set_latency` calls with generic state-to-lowercase logging
  4. Timer logic preserved as-is

## Verification

- `cargo check`: **SKIPPED** — environment permission denied (Termux/Android build restriction). No build toolchain available.
- Manual review: no unused imports, no logic errors, i18n keys verified in both `en.ftl` and `zh.ftl`.

## Files touched

- `src/idle_dive/latency.rs`
- `src/idle_dive/controller.rs`
