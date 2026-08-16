# Task 4: IdleDive Doze debounce (B2)

## Goal

Add debounce to prevent rapid Normal ↔ DozeDiving transitions from screen state jitter.

## File to modify

`src/idle_dive/controller.rs`

## Current code (lines 96-106)

```rust
pub fn enter_doze(&mut self) {
    if self.disabled { return; }
    self.transition_to(IdleDiveState::DozeDiving);
}

pub fn exit_doze(&mut self) {
    if self.disabled { return; }
    if self.state == IdleDiveState::DozeDiving {
        self.transition_to(IdleDiveState::Normal);
    }
}
```

## Required change

Add a `last_doze_exit: Instant` field to `IdleDiveController`. In `enter_doze()`, check if at least 500ms has elapsed since last doze exit before transitioning. If not, skip.

```rust
// New field in struct:
last_doze_exit: Instant,

// Initialize in new() and disabled():
last_doze_exit: Instant::now(),

// Modified enter_doze:
pub fn enter_doze(&mut self) {
    if self.disabled { return; }
    if self.last_doze_exit.elapsed().as_millis() < 500 {
        return; // debounce: ignore rapid re-entry
    }
    self.transition_to(IdleDiveState::DozeDiving);
}

// Modified exit_doze:
pub fn exit_doze(&mut self) {
    if self.disabled { return; }
    if self.state == IdleDiveState::DozeDiving {
        self.transition_to(IdleDiveState::Normal);
        self.last_doze_exit = Instant::now();
    }
}
```

## Verification

Manual review — cargo check unavailable.
