# Task 2: IdleDive code quality (C1+C3+C5+C6)

## Goal

Fix error propagation in LatencyWriter, deduplicate transition_to code, use PathBuf for paths, and fix i18n key usage.

## Files to modify

1. `src/idle_dive/latency.rs` — C1 (error propagation), C5 (PathBuf), C6 (i18n keys)
2. `src/idle_dive/controller.rs` — C3 (transition_to dedup)

## Detailed requirements

### C1: LatencyWriter error propagation (latency.rs)

**set_governor()** (lines 100-109): Currently always returns `Ok(())` even when writes fail. Change to:
- Track if ANY write succeeded (at least one path wrote OK)
- If all paths failed AND there were paths to write, return `Err`
- If at least one succeeded, return `Ok(())`
- Keep the `warn!` on individual failures

```rust
pub fn set_governor(&self, governor: &str) -> Result<()> {
    if self.governor_paths.is_empty() { return Ok(()); }
    let mut any_ok = false;
    for path in &self.governor_paths {
        if let Err(e) = crate::utils::write_to_file(path, governor.as_bytes()) {
            warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!(
                "path" => path.display().to_string(),
                "error" => e.to_string()
            )));
        } else {
            any_ok = true;
        }
    }
    if any_ok { Ok(()) } else { Err(anyhow::anyhow!("all governor writes failed")) }
}
```

**set_latency()** (lines 112-140): Same pattern. Track PM-QoS write result + sysfs writes:
- If PM-QoS fd exists and write fails, warn but don't immediately fail
- Track if ANY sysfs write succeeded
- Return `Err` only if PM-QoS failed AND all sysfs writes failed AND there were paths

### C5: PathBuf (latency.rs)

Change `governor_paths: Vec<String>` to `governor_paths: Vec<PathBuf>` and `latency_paths: Vec<String>` to `latency_paths: Vec<PathBuf>`.

Add `use std::path::PathBuf;` import.

When constructing paths (lines 44-56), use `PathBuf` directly:
```rust
governor_paths.push(gov_path);  // already a PathBuf from base.join()
```

When writing, pass `path` directly to `write_to_file` (it accepts `AsRef<Path>`).

### C6: i18n keys (latency.rs)

Currently all 4 error sites use `"sysfs-open-failed"`. Change to:
- Line 91 (PM-QoS open fail): use `"pm-qos-open-failed"` — but check if this key exists in i18n files. If not, use `"sysfs-open-failed"` with a descriptive path string (since adding i18n keys is out of scope for this task). Actually, the safest approach: keep using the existing key but make the path descriptive enough to distinguish in logs. The path already distinguishes (`/dev/cpu_dma_latency` vs sysfs paths), so this is already fine.

**Revised C6**: Skip changing i18n keys — the paths already distinguish the errors in log output. Changing keys would require updating i18n .ftl files which is scope creep. Instead, use the controller-level keys (`idle-dive-set-governor-failed`, `idle-dive-set-latency-failed`) which already exist and are used in controller.rs.

### C3: transition_to dedup (controller.rs)

Replace the repetitive match in `transition_to` (lines 121-157) with a lookup pattern:

```rust
fn transition_to(&mut self, new_state: IdleDiveState) {
    if self.state == new_state { return; }

    let (governor, latency_us) = match new_state {
        IdleDiveState::Normal => (&self.config.governors.normal, self.config.params.normal_latency_us),
        IdleDiveState::Diving => (&self.config.governors.diving, self.config.params.diving_latency_us),
        IdleDiveState::DozeDiving => (&self.config.governors.doze, self.config.params.doze_latency_us),
    };

    match new_state {
        IdleDiveState::Normal => info!("{}", t("idle-dive-exit")),
        IdleDiveState::Diving => info!("{}", t("idle-dive-enter")),
        IdleDiveState::DozeDiving => info!("{}", t("idle-dive-enter-dozed")),
    }

    if let Err(e) = self.latency_writer.set_governor(governor) {
        warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => format!("{:?}", new_state).to_lowercase(), "error" => e.to_string())));
    }
    if let Err(e) = self.latency_writer.set_latency(latency_us) {
        warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => format!("{:?}", new_state).to_lowercase(), "error" => e.to_string())));
    }

    if new_state == IdleDiveState::Normal {
        self.dive_timer = Instant::now();
    } else if new_state == IdleDiveState::Diving {
        self.exit_timer = Instant::now();
    }

    self.state = new_state;
}
```

## Verification

1. `cargo check` must pass
2. `cargo test` must pass (if tests exist)
3. Confirm no unused imports

## Conventions

- Follow existing code style
- Use i18n for log messages (existing keys)
- No new dependencies
- No comments unless necessary for clarity
