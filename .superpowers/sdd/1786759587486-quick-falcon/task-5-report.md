# Task 5 Report: Docs Update

## Status: ✅ Complete

## Changes Made

### README.md
- **Line 117**: Changed diagram IdleDive description from "CPU idle >12%" to "CPU 利用率 <12%"
- **Line 128**: Updated StillDive table row — "CPU 负载 <8%" → "前台 app CPU 利用率 <5%", "持续 10 个 tick" → "持续 30 个 tick"
- **Line 129**: Updated IdleDive table row — "CPU idle >12%" → "CPU 平均利用率 <12%"
- **Lines 137-145**: Updated StillDive config example:
  - `enter_threshold: 0.08` → `0.05` with comment "前台 app 总 CPU 利用率阈值"
  - `enter_ticks: 10` → `30` with comment "30 tick ≈ 6秒"
  - `exit_threshold: 0.20` → `0.15` with comment "前台 app CPU 利用率阈值"

### docs/CPU静止下潜-一步到位综合方案.md
- Updated default values section:
  - `sd_enter_thresh() -> 0.08` → `0.05`, description changed to "前台 app 总 CPU 利用率 < 5%"
  - `sd_enter_ticks() -> 10` → `30`, description changed to "持续 30 tick (6s)"
  - `sd_exit_thresh() -> 0.20` → `0.15`, description changed to "前台 app CPU 利用率 > 15% 退出"
- Updated CLG core logic: "计算 core_utils 最大 util" → "计算前台 app 总 CPU 利用率（foreground_max_util）"
- Updated config section (section 7): `enter_threshold: 0.08→0.05`, `enter_ticks: 10→30`, `exit_threshold: 0.20→0.15`

## Notes
- All changes are documentation-only, no code modified
- No commit made (per instructions)
