# Task 3: StillDive exit smoothing (B3)

## Goal

Smooth the smoothing_up transition when exiting StillDive boost, instead of jumping from 1.0 to the base value abruptly.

## File to modify

`src/scheduler/cpu_load_governor.rs` — the `effective_smoothing_up` computation in `on_load_update`

## Current code (after Task 1 refactor)

```rust
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

## Required change

Replace the `1.0` in the `exit_boost > 0` branch with a linear decay from 1.0 to the base smoothing_up value:

```rust
} else if sd.exit_boost > 0 {
    let base = self.cfg.smoothing_up;
    let progress = 1.0 - (sd.exit_boost as f32 / sd.config.exit_boost_ticks as f32);
    base + (1.0 - base) * progress
}
```

This means:
- When exit_boost just started (exit_boost == exit_boost_ticks): progress = 0.0, effective = 1.0 (fast recovery)
- When exit_boost is about to end (exit_boost == 1): progress ≈ 1.0, effective ≈ base (normal speed)
- Linear interpolation between the two

## Notes

- `sd.config.exit_boost_ticks` is the TOTAL boost ticks (e.g., 5), not the current countdown
- `sd.exit_boost` is the REMAINING ticks (counts down from 5 to 0)
- If `exit_boost_ticks` is 0, avoid division by zero — but normalize() already prevents this (min 1? actually it allows 0). Add `.max(1)` guard.

## Verification

Manual review — cargo check unavailable in this environment.
