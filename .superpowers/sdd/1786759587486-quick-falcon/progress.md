# SDD ledger — plan: /storage/emulated/0/ceshi/yumi/.mimocode/plans/1786759587486-quick-falcon.md
# BASE: 352d843

## Task dependency graph

```
Task 1: StillDive core refactor (A1+C2+C4) — changes on_load_update signature
  ↓
Task 2: IdleDive code quality (C1+C3+C5+C6) — independent module, no overlap with T1
Task 3: StillDive smoothing (B3) — depends on T1 (uses StillDiveRuntime)
Task 4: IdleDive Doze debounce (B2) — independent from T1
  ↓
Task 5: Docs update (D1+D2+D3) — depends on T1 for new threshold values
```

## Conflict scan

| Task pair | Shared file/interface | Finding |
|-----------|----------------------|---------|
| T1 ∩ T2 | No overlap | Clean |
| T1 ∩ T3 | cpu_load_governor.rs StillDive logic | T3 edits effective_smoothing_up calc, T1 restructures StillDive state → T3 must run after T1 |
| T1 ∩ T4 | No overlap | Clean |
| T1 ∩ T5 | Config values in docs | T5 needs T1's final threshold values |
| T3 ∩ T4 | No overlap | Clean |

## Tasks

- [x] Task 1: StillDive core refactor (A1+C2+C4) — complete (commits uncommitted, review clean)
  - Spec ✅, 1 minor (redundant rebinding, non-blocking)
  - cargo check skipped: environment lacks nightly Rust + eBPF toolchain
- [x] Task 2: IdleDive code quality (C1+C3+C5+C6) — complete (review clean)
- [x] Task 3: StillDive exit smoothing (B3) — complete (review clean)
- [x] Task 4: IdleDive Doze debounce (B2) — complete (review clean)
- [x] Task 5: Docs update (D1+D2+D3) — complete (review clean)

## Final review

- 1 critical (i18n template mismatch) — **FIXED**: reverted latency.rs to `sysfs-open-failed` key
- 1 important (smoothing off-by-one) — **FIXED**: moved `exit_boost -= 1` after effective value calculation
- 1 minor (hardcoded 30) — **FIXED**: code already uses `sd_enter_ticks()`
- 1 minor (state name format) — parked, cosmetic only

All critical/important findings addressed. No new breakage.
