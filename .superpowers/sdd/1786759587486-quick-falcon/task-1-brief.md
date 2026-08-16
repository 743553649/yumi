# Task 1: StillDive core refactor (A1+C2+C4)

## Goal

Refactor StillDive to use `foreground_max_util` instead of `max_util`, extract StillDive state into a dedicated struct, and eliminate duplicate computation.

## Files to modify

1. `src/scheduler/config.rs` — change default threshold values
2. `src/scheduler/cpu_load_governor.rs` — refactor StillDive logic, change `on_load_update` signature
3. `src/scheduler/mod.rs` — update call site at line 427
4. `module/config/config.yaml` — update StillDive section

## Detailed requirements

### 1. config.rs — StillDiveConfig default values

Change these default functions (lines 186-191):

```
sd_enter_thresh() -> 0.05   (was 0.20)
sd_enter_ticks()  -> 30     (was 10)
sd_exit_thresh()  -> 0.15   (was 0.25)
```

Keep `sd_exit_boost() -> 5`, `sd_perf_ceil() -> 0.30`, `sd_smoothing_up() -> 0.05` unchanged.

Also update the `normalize()` method: change the `enter_ticks == 0` fallback to `30`, and the exit_threshold minimum gap check from `+0.05` to `+0.05` (this is fine as-is).

### 2. cpu_load_governor.rs — Extract StillDiveRuntime

Replace the 4 scattered fields on `CpuLoadGovernor` (lines 115-118):
```rust
still_dive: Option<StillDiveConfig>,
still_mode: bool,
still_low_ticks: u32,
still_exit_boost: u32,
```

With a single field:
```rust
still_dive: Option<StillDiveRuntime>,
```

Create a new struct `StillDiveRuntime` (can be in the same file, before `CpuLoadGovernor`):
```rust
struct StillDiveRuntime {
    config: StillDiveConfig,
    mode: bool,
    low_ticks: u32,
    exit_boost: u32,
}

impl StillDiveRuntime {
    fn new(config: StillDiveConfig) -> Self {
        Self { config, mode: false, low_ticks: 0, exit_boost: 0 }
    }

    fn reset(&mut self) {
        self.mode = false;
        self.low_ticks = 0;
        self.exit_boost = 0;
    }
}
```

### 3. cpu_load_governor.rs — Change on_load_update signature

Change `on_load_update(&mut self, core_utils: &[f32])` to:
```rust
pub fn on_load_update(&mut self, core_utils: &[f32], foreground_max_util: f32)
```

### 4. cpu_load_governor.rs — StillDive logic refactor

In `on_load_update`, replace the current StillDive block (lines 356-384) with logic that uses `foreground_max_util` instead of `max_util`:

```rust
if let Some(ref mut sd) = self.still_dive {
    if !sd.mode {
        if foreground_max_util <= sd.config.enter_threshold {
            sd.low_ticks += 1;
        } else {
            sd.low_ticks = 0;
        }
        if sd.low_ticks >= sd.config.enter_ticks {
            sd.mode = true;
            log::info!("{}", t_with_args("clg-still-enter", &fluent_args!(
                "ceil" => format!("{:.0}%", sd.config.perf_ceil * 100.0)
            )));
        }
    } else {
        if foreground_max_util > sd.config.exit_threshold {
            sd.mode = false;
            sd.exit_boost = sd.config.exit_boost_ticks;
            log::info!("{}", t_with_args("clg-still-exit", &fluent_args!(
                "boost" => sd.config.exit_boost_ticks.to_string()
            )));
        }
    }

    if sd.exit_boost > 0 {
        sd.exit_boost -= 1;
    }
}
```

Update the effective parameter computation (lines 386-398) to use `sd.mode` and `sd.config`:
```rust
let effective_perf_ceil = if let Some(ref sd) = self.still_dive {
    if sd.mode { sd.config.perf_ceil } else { self.cfg.perf_ceil }
} else {
    self.cfg.perf_ceil
};
let effective_perf_floor = if let Some(ref sd) = self.still_dive {
    if sd.mode { 0.0 } else { self.cfg.perf_floor }
} else {
    self.cfg.perf_floor
};
let effective_smoothing_up = if let Some(ref sd) = self.still_dive {
    if sd.mode {
        sd.config.smoothing_up
    } else if sd.exit_boost > 0 {
        1.0
    } else {
        self.cfg.smoothing_up
    }
} else {
    self.cfg.smoothing_up
};
```

### 5. cpu_load_governor.rs — Debug log (C4 fix)

The debug log block (around line 490-495) currently recomputes `max_util`. Replace it to use `foreground_max_util` (already available as parameter):

```rust
if let Some(ref sd) = self.still_dive {
    debug!("[StillDive] fg_util={:.1}% threshold={:.0}% mode={} ticks={}/{}",
        foreground_max_util * 100.0, sd.config.enter_threshold * 100.0,
        sd.mode, sd.low_ticks, sd.config.enter_ticks);
}
```

### 6. cpu_load_governor.rs — init_policies, reload_config, release

Update these methods to work with `StillDiveRuntime`:

- `init_policies`: when `still_dive` is `Some(config)`, create `StillDiveRuntime::new(config)`
- `reload_config`: same — create new `StillDiveRuntime::new(config)`, resetting state
- `release`: call `self.still_dive.as_mut().map(|sd| sd.reset())` (or just set to None — either is fine since release already clears clusters)

Actually, looking at the code: `init_policies` and `reload_config` accept `Option<StillDiveConfig>` and store it. Change them to convert to `Option<StillDiveRuntime>`:

```rust
// In init_policies:
self.still_dive = still_dive.map(|c| { let mut c = c; c.normalize(); StillDiveRuntime::new(c) });

// In reload_config:
self.still_dive = still_dive.map(|c| { let mut c = c; c.normalize(); StillDiveRuntime::new(c) });

// In release:
if let Some(ref mut sd) = self.still_dive {
    sd.reset();
}
```

### 7. mod.rs — Update call site

Line 427: change `cpu_governor.on_load_update(&core_utils);` to:
```rust
cpu_governor.on_load_update(&core_utils, foreground_max_util);
```

The `foreground_max_util` variable is already destructured from the event at line 418.

### 8. config.yaml — Update StillDive section

```yaml
StillDive:
  enabled: true
  enter_threshold: 0.05
  enter_ticks: 30
  exit_threshold: 0.15
  exit_boost_ticks: 5
  perf_ceil: 0.30
  smoothing_up: 0.05
```

## Verification

1. `cargo check` must pass
2. `cargo test` must pass (if tests exist)
3. Confirm no unused imports or variables

## Conventions

- Follow existing code style (no comments unless asked)
- Use i18n for all log messages (already in place, just update the debug format string)
- No new dependencies
