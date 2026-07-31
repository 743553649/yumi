# CLG 重构与 FAS 安全加固 — 完整修改记录

> 实施日期：2026-07-31
> 涉及版本：yumi v2.0.1

---

## 阶段一：Bug 修复（防御性修正）

### 1.1 PascalCase 反序列化修复
- **文件**: `src/scheduler/config.rs:84`
- **问题**: `Mode` 结构体的 `#[serde(rename_all = "PascalCase")]` 导致 YAML 中 `cpu_load_governor` 键无法匹配，用户自定义的 CLG 配置（含 `enabled`）从未生效
- **修复**: 删除 `rename_all` 属性，让 serde 使用字段原名匹配

### 1.2 perf_floor > perf_ceil clamp panic
- **文件**: `src/scheduler/cpu_load_governor.rs:168`
- **问题**: `.clamp(perf_floor, perf_ceil)` 当用户配置 floor > ceil 时 panic
- **修复**: clamp 前规范化，floor > ceil 时回退到 ceil

### 1.3 未知/空模式意外启用 CLG
- **文件**: `src/scheduler/mod.rs:186`
- **问题**: `get_mode("fas")` 返回 `None`，`.unwrap_or_default()` 产生 `enabled: true`，导致 FAS 模式下 CLG 意外激活
- **修复**: 未知模式返回 `enabled: false` 的默认配置

### 1.4 i18n 非法 language 标签 panic
- **文件**: `src/i18n.rs:50`
- **问题**: `lang.parse().unwrap()` 对非法标签（空字符串、特殊字符）panic
- **修复**: `unwrap_or_else` 回退到 `"en"`

---

## 阶段二：FAS 安全加固（异常场景修正）

### 2.1 FAS clamp panic 规范化
- **文件**: `src/scheduler/fas/controller.rs:212` + `src/scheduler/fas/frame_pipeline.rs:166`
- **问题**: `effective_perf_floor()` 返回值可能超过 `perf_ceil`；`fast_decay_max_step < min_step` 时 clamp panic
- **修复**:
  - `effective_perf_floor()` 增加 `.min(ceil)` 约束
  - `fast_decay` 步长取 `max(max_step, min_step)` 保底

### 2.2 floor-rescue 不被 max_inc 截断
- **文件**: `src/scheduler/fas/pid_jank.rs:214`
- **问题**: floor-rescue 将 perf 设为 `perf_cold_boot`，但随后被 `max_inc` 截断回 `old_perf + 0.09`，自救失效
- **修复**: `max_inc` 判断中增加 `|| act == "floor-rescue"`，允许直接跳到目标值

### 2.3 PID 系数非法 target_fps 保护
- **文件**: `src/scheduler/fas/pid.rs:49`
- **问题**: `adapt_to_target_fps()` 对 0、负数、NaN 的 target_fps 未防护
- **修复**: 入口处 guard，非法值回退到 60.0

### 2.4 per-app 帧率档位过滤非法值
- **文件**: `src/scheduler/fas/controller.rs:293`
- **问题**: per-app 配置中的 `target_fps` 数组可能包含 0、负数、NaN
- **修复**: 加载后过滤，仅保留 `is_finite() && > 0.0` 的值

---

## 阶段三：CLG 重构（核心行为变更）

### 3.1 新增 8 个 CLG 配置参数
- **文件**: `src/scheduler/config.rs` + `module/config/config.yaml` + `README.md`
- **新增参数**:

| 参数 | 默认值 | 说明 |
|:---|:---:|:---|
| `headroom_ramp` | 0.15 | headroom 在 up_threshold 附近的过渡带宽度 |
| `up_jump_threshold` | 0.35 | 快速升频通道的跳变幅度阈值 |
| `slow_up_scale` | 0.02 | 滞回带内升频的最低速率基准 |
| `slow_down_scale` | 0.5 | 滞回带内降频的缩放系数 |
| `down_fast_threshold` | 0.15 | 极低负载快速降频的触发阈值 |
| `down_fast_mult` | 3.0 | 极低负载降频放大倍数 |
| `spike_jump_threshold` | 0.35 | 单 tick 尖峰抑制的跳变阈值 |
| `spike_decay` | 0.5 | 尖峰衰减比例 |

### 3.2 重写 on_load_update()
- **文件**: `src/scheduler/cpu_load_governor.rs:263`
- **变更**:
  1. **headroom 平滑过渡**: 二值切换 → 线性渐变（`headroom_ramp` 控制过渡带宽度），消除负载临界时的频率振荡
  2. **滞回带内降频**: 目标低于当前即可降频，按 slow/normal/fast 三档平滑回落（旧逻辑：必须低于 down_threshold 且等待确认）
  3. **中等负载升频提速**: util 接近 up_threshold 时升频速率线性提升（旧逻辑：固定 `smoothing_up * 0.02`）
  4. **尖峰抑制**: 单 tick 负载跳升超过阈值时衰减增量，持续负载下一 tick 即全量生效
  5. **极低负载快速降频**: 低于 `down_fast_threshold` 时跳过降频确认期立即快速回落

### 3.3 release() 状态快照与恢复
- **文件**: `src/scheduler/cpu_load_governor.rs:233`
- **变更**:
  - `ClusterState` 新增 `pre_takeover_gov`、`pre_takeover_min_freq`、`pre_takeover_max_freq` 字段
  - `init_policies()` 接管前保存当前 governor 和频率范围
  - `release()` 按序恢复 governor 和频率，读取失败不写退化值

### 3.4 热重载优先
- **文件**: `src/scheduler/mod.rs:248,318`
- **变更**: 亮屏恢复和模式切换时，如果 CLG 已激活则优先 `reload_config()` 而非 `init_policies()`，避免全量重建 sysfs writer

### 3.5 IPC catch_unwind
- **文件**: `src/scheduler/mod.rs:164,397`
- **变更**: IPC 线程事件循环包裹 `std::panic::catch_unwind`，panic 时输出日志而非静默死亡
- **i18n**: 新增 `scheduler-ipc-panicked` key（en.ftl + zh.ftl）

---

## 阶段四：AppDetect 优化（尚未实施）

- 同模式 ModeChange 去重：配置重载/亮屏恢复不再产生 balance → balance 冗余事件

---

## 修改文件清单

| 文件 | 阶段 | 变更类型 |
|:---|:---:|:---|
| `src/scheduler/config.rs` | 一/三 | 修复 serde + 新增 8 参数 |
| `src/scheduler/cpu_load_governor.rs` | 一/二/三 | clamp 修复 + 重写 on_load_update + release 快照 |
| `src/scheduler/mod.rs` | 一/三 | 未知模式修复 + 热重载优先 + catch_unwind |
| `src/i18n.rs` | 一 | panic 修复 |
| `src/scheduler/fas/controller.rs` | 二 | floor clamp + 帧率过滤 |
| `src/scheduler/fas/frame_pipeline.rs` | 二 | fast_decay 步长修复 |
| `src/scheduler/fas/pid_jank.rs` | 二 | floor-rescue 豁免 max_inc |
| `src/scheduler/fas/pid.rs` | 二 | target_fps guard |
| `module/config/config.yaml` | 三 | 新增 8 参数到 4 种模式 |
| `module/config/i18n/en.ftl` | 三 | 新增 panic 日志 key |
| `module/config/i18n/zh.ftl` | 三 | 新增 panic 日志 key |
| `README.md` | 三 | 同步参数文档 |
| `docs/worklog.md` | 一/二/三 | 工作日志更新 |
