# Task 2 Report: StillDive 实现

## 实现内容

### Task 1 前置依赖（一并完成）
Task 1 尚未完成，因此一并实现了 StillDiveConfig 配置基础：

- `src/scheduler/config.rs`: 添加 `StillDiveConfig` 结构体（7个字段），含 `Default` 和 `normalize()` 方法；`Config` 新增 `still_dive` 字段；`FunctionToggles` 新增 `scheduler_tuning` 字段
- `module/config/config.yaml`: 添加 `StillDive` 配置块（位于 CpuIdle 之后、性能模式之前）
- `module/config/i18n/zh.ftl`: `clg-still-enter` 和 `clg-still-exit` 翻译键（已存在，无重复）
- `module/config/i18n/en.ftl`: 对应英文翻译键（已存在，无重复）

### Task 2 CLG 修改

**`src/scheduler/cpu_load_governor.rs`:**
1. 新增 `StillDiveConfig` import
2. `CpuLoadGovernor` 新增 4 个字段：`still_dive`, `still_mode`, `still_low_ticks`, `still_exit_boost`
3. `new()` 初始化新字段
4. `init_policies()` 签名扩展为 `(gov_cfg, still_dive: Option<StillDiveConfig>)`，设置并 normalize
5. `on_load_update()` 在 per-cluster 循环前实现 StillDive 状态机：
   - 未下潜：检测 `max_util <= enter_threshold` 连续 `enter_ticks` 次后进入
   - 已下潜：检测 `max_util > exit_threshold` 后退出，设置 exit_boost
   - 退出助力递减
   - 计算 effective_perf_ceil / effective_perf_floor / effective_smoothing_up
6. per-cluster 循环使用 effective 值替代原 cfg 值（clamp 和 smoothing 都用 effective 值）
7. `reload_config()` 签名扩展，热更新 still_dive 并 normalize
8. `release()` 重置 StillDive 状态（still_mode, still_low_ticks, still_exit_boost）

**`src/scheduler/mod.rs`:**
- 所有 8 个 `init_policies` / `reload_config` 调用点已更新
- 亮屏场景从 config 提取 `still_dive`（enabled 时传 `Some`，否则 `None`）
- Doze 息屏场景传 `None`（息屏不启用 StillDive，使用独立的 Doze 配置）

## 验证
- `cargo build` 无法验证（环境中未安装 Rust 工具链）
- 代码逻辑手动审查通过
- i18n 键无重复，config.yaml 无重复

## 自审发现
- `still_dive` 为 `None` 时 `still_mode` 永远为 `false`，`unwrap()` 路径安全
- `t_val` 变量名避免了与 i18n `t` 函数的 shadowing 冲突
- StillDiveConfig 的 `normalize()` 在 init 和 reload 时都会调用
- `exit_threshold <= enter_threshold` 时自动修正为 `enter_threshold + 0.05`

## 文件变更清单
| 文件 | 变更类型 |
|------|----------|
| `src/scheduler/config.rs` | 新增 StillDiveConfig + Config.still_dive + FunctionToggles.scheduler_tuning |
| `src/scheduler/cpu_load_governor.rs` | 新增字段、修改签名、StillDive 检测逻辑 |
| `src/scheduler/mod.rs` | 8 个调用点更新 |
| `module/config/config.yaml` | 新增 StillDive 配置块 |
| `module/config/i18n/zh.ftl` | 已有翻译键（无新增） |
| `module/config/i18n/en.ftl` | 已有翻译键（无新增） |
