# Task 5: Docs update (D1+D2+D3)

## Goal

Update README.md and design doc to match the new StillDive behavior (foreground_max_util) and fix existing inaccuracies.

## Files to modify

1. `README.md` — lines ~117-147 (StillDive/IdleDive description)
2. `docs/CPU静止下潜-一步到位综合方案.md` — StillDive section

## Requirements

### D1: README StillDive update

Line 128: Change "CPU 负载 <8%" to "前台 app CPU 利用率 <5%"
Line 117: Change "CPU idle >12%" to "CPU 利用率 <12%"

Update the StillDive config example (lines 137-145):
```yaml
StillDive:
  enabled: true
  enter_threshold: 0.05    # 前台 app 总 CPU 利用率阈值
  enter_ticks: 30          # 连续多少个 tick 满足条件才进入（30 tick ≈ 6秒）
  exit_threshold: 0.15     # 退出下潜的前台 app CPU 利用率阈值
  exit_boost_ticks: 5      # 退出后 boost 持续 tick 数
  perf_ceil: 0.30          # 下潜时的性能上限
  smoothing_up: 0.05       # 升频平滑系数（越小越平滑）
```

Update the table row for StillDive (line 128):
| **StillDive** | 亮屏、前台 app CPU 利用率 <5% 持续 30 个 tick | perf_ceil 锁死 30%，省电约 15~20% | `config.yaml` 中 `StillDive` 节 |

### D2: README IdleDive description fix

Line 129: The description "CPU idle >12%" is misleading. Change to:
| **IdleDive** | CPU 平均利用率 <12% 持续 500ms | 切换到低功耗 idle governor，延迟从 100μs 提升到 800~1500μs | `module/config/idle_dive.yaml` |

### D3: Design doc update

In `docs/CPU静止下潜-一步到位综合方案.md`:
- Update the StillDive section to mention it uses `foreground_max_util` (前台 app 总 CPU 利用率)
- Change `sd_enter_thresh() -> 0.08` to `sd_enter_thresh() -> 0.05`
- Change `sd_enter_ticks() -> 10` to `sd_enter_ticks() -> 30`
- Change `sd_exit_thresh() -> 0.20` to `sd_exit_thresh() -> 0.15`
- Update the description to say "前台 app 总 CPU 利用率" instead of "所有核心 max util"

## Notes

- Keep the existing document structure and style
- Only change the specific values and descriptions that are wrong
- Do not add new sections or restructure the docs
