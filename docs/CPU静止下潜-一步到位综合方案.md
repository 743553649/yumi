# CPU 静止下潜 — 一步到位综合实现方案

> 结合复刻版 (743553649/yumi) 的 IdleDive + TouchBoost，融入自研 StillDive，再加专业优化，一次完成避免反复修改。

---

## 架构总览：三层省电体系

```
┌─────────────────────────────────────────────────────────────┐
│                      TouchBoost                             │
│                 触摸交互 → 快速退出省电                       │
└────────────────────────┬────────────────────────────────────┘
                         │ 触摸事件
┌────────────────────────▼────────────────────────────────────┐
│               StillDive (CLG 内部参数覆盖)                    │
│  亮屏静止检测 → 压低 perf_ceil/smoothing_up → 频率级省电       │
└────────────────────────┬────────────────────────────────────┘
                         │ 同 util 输入
┌────────────────────────▼────────────────────────────────────┐
│              IdleDive (cpuidle C-state 管理)                  │
│  平均负载检测 → 调深 idle state / 放宽 latency → 睡眠级省电    │
└─────────────────────────────────────────────────────────────┘
```

**三层的关系**：StillDive 管"频率跑多高"，IdleDive 管"空闲睡多深"，TouchBoost 管"醒来多快"——覆盖 CPU 省电的完整链路。

---

## 需要创建/修改的文件清单

### 新增模块（从复刻版移植）

| # | 文件 | 来源 | 说明 |
|---|------|------|------|
| 1 | `src/idle_dive/mod.rs` | 移植 | IdleDive 模块入口，导出子模块 |
| 2 | `src/idle_dive/config.rs` | 移植 | IdleDiveConfig 结构体 + 默认值 |
| 3 | `src/idle_dive/controller.rs` | 移植 | 状态机 (Normal→Diving→DozeDiving) |
| 4 | `src/idle_dive/latency.rs` | 移植 | PM-QoS + sysfs 双重写入器 |
| 5 | `src/touch_boost/mod.rs` | 移植 | TouchBoost 模块入口 |
| 6 | `src/touch_boost/config.rs` | 移植 | TouchBoostConfig |
| 7 | `src/touch_boost/controller.rs` | 移植 | 触摸提频衰减控制器 |
| 8 | `src/touch_boost/monitor.rs` | 移植 | epoll 触摸事件监听器 |
| 9 | `module/config/idle_dive.yaml` | 新配置 | IdleDive 配置文件 |
| 10 | `module/config/touch_boost.yaml` | 新配置 | TouchBoost 配置文件 |

### 修改现有文件

| # | 文件 | 说明 |
|---|------|------|
| 11 | `src/scheduler/config.rs` | 新增 StillDiveConfig 结构体，添加到 Config |
| 12 | `src/scheduler/cpu_load_governor.rs` | 核心：StillDive 检测 + 参数覆盖 + 退出恢复 |
| 13 | `src/scheduler/mod.rs` | IPC 线程中创建 IdleDive/TouchBoost 并联动 |
| 14 | `src/scheduler/scheduler.rs` | 扩展系统调优：sched 参数调优 |
| 15 | `src/main.rs` | 注册 idle_dive/touch_boost 模块 |
| 16 | `module/config/config.yaml` | 新增 StillDive + SchedulerTuning 配置块 |
| 17 | `module/config/i18n/zh.ftl` | 新增翻译键 |
| 18 | `module/config/i18n/en.ftl` | 新增翻译键 |
| 19 | `AGENTS.md` | 新增「功能计划文档保存到 docs/」规则 |
| 20 | `README.md` | 文档化新功能 |

---

## 详细设计

### 1. StillDiveConfig (`src/scheduler/config.rs`)

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct StillDiveConfig {
    #[serde(default = "default_true")]  pub enabled: bool,
    #[serde(default = "sd_enter_thresh")]  pub enter_threshold: f32,
    #[serde(default = "sd_enter_ticks")]   pub enter_ticks: u32,
    #[serde(default = "sd_exit_thresh")]   pub exit_threshold: f32,
    #[serde(default = "sd_exit_boost")]    pub exit_boost_ticks: u32,
    #[serde(default = "sd_perf_ceil")]     pub perf_ceil: f32,
    #[serde(default = "sd_smoothing_up")]  pub smoothing_up: f32,
}
```

默认值函数：
- `sd_enter_thresh() -> 0.05` — 前台 app 总 CPU 利用率 < 5%
- `sd_enter_ticks() -> 30` — 持续 30 tick (6s)
- `sd_exit_thresh() -> 0.15` — 前台 app CPU 利用率 > 15% 退出
- `sd_exit_boost() -> 5` — 退出后 5 tick (1s) 升频助力
- `sd_perf_ceil() -> 0.30` — 下潜时 perf 上限 30%
- `sd_smoothing_up() -> 0.05` — 下潜时升频极慢

添加到 Config：
```rust
#[serde(default, rename = "StillDive")]
pub still_dive: StillDiveConfig,
```

实现 `Into<Option<StillDiveConfig>>`（enabled=false 时返回 None）。

### 2. CLG 核心改动 (`cpu_load_governor.rs`)

**新增字段**：
```rust
pub struct CpuLoadGovernor {
    // ... 现有字段 ...
    still_dive: Option<StillDiveConfig>,
    still_mode: bool,
    still_low_ticks: u32,
    still_exit_boost: u32,
}
```

**`on_load_update()`** — 在 per-cluster 循环前插入：

```
1. 如果 still_dive 存在且 enabled:
   a. 计算前台 app 总 CPU 利用率（foreground_max_util）
   b. 未下潜:
      - max ≤ enter_threshold → still_low_ticks++
      - max > enter_threshold → still_low_ticks = 0
      - still_low_ticks ≥ enter_ticks → 进入下潜，日志输出
   c. 已下潜:
      - max > exit_threshold → 退出，设 still_exit_boost = exit_boost_ticks
      - still_exit_boost > 0 → 递减
```

per-cluster 循环内替换 clamp 和 smoothing：
```
effective_perf_ceil  = still_mode ? sd.perf_ceil   : cfg.perf_ceil
effective_perf_floor = still_mode ? 0.0             : cfg.perf_floor
effective_smoothing_up =
    still_mode           → sd.smoothing_up    (极慢)
    still_exit_boost > 0 → 1.0                (快速恢复)
    正常                 → cfg.smoothing_up
```

**API 签名变更**：
```rust
pub fn init_policies(&mut self, gov_cfg: &CpuLoadGovernorConfig, still_dive: Option<StillDiveConfig>)
pub fn reload_config(&mut self, gov_cfg: &CpuLoadGovernorConfig, still_dive: Option<StillDiveConfig>)
```

`release()` 不变但重置 still 相关字段。

### 3. IdleDive（从复刻版移植）

3 个文件直接复刻，内容已在前面完整展示，关键点：

- **`config.rs`** — `IdleDiveConfig { enabled, dive_threshold, exit_threshold, dive_delay_ms, exit_delay_ms, governors, params }`
- **`controller.rs`** — `IdleDiveController` 状态机，方法：`init()`, `update(avg_util)`, `enter_dive()`, `exit_dive()`, `enter_doze()`, `exit_doze()`, `on_touch_fast_exit()`
- **`latency.rs`** — `LatencyWriter`，sysfs + PM-QoS 双通道写入

适配注意：
- `FastWriter::write_value_force_str()` 在复刻版有，可能需要移植到 `utils.rs`
- 如现有 `FastWriter` 不支持写字符串，可改用 `try_write_file`

### 4. TouchBoost（从复刻版移植）

3 个文件直接复刻，关键点：

- **`config.rs`** — `TouchBoostConfig { enabled, boost_freqs, release_delay_ms, recover_decay, min_boost_duration_ms, input_device }`
- **`controller.rs`** — Boost 状态机，写 `scaling_min_freq` 提频 + 衰减恢复
- **`monitor.rs`** — epoll 监听 `/dev/input/event*` 触摸事件

### 5. 调度器线程协调 (`scheduler/mod.rs`)

```
SystemLoadUpdate 事件 → 传给 CLG + IdleDive:
  cpu_governor.on_load_update(&core_utils);           // CLG 含 StillDive
  if !core_utils.is_empty() {
      let avg = core_utils.iter().sum::<f32>() / core_utils.len() as f32;
      idle_dive.update(avg);                            // IdleDive
  }

ScreenStateChange → IdleDive:
  screen_off → idle_dive.enter_doze()
  screen_on  → idle_dive.exit_doze()

TouchBoost touch_event → IdleDive on_touch_fast_exit():
  - TouchBoost 收到触摸事件时，通过 channel 通知 scheduler
  - scheduler 收到后调用 idle_dive.on_touch_fast_exit()
```

### 6. 调度器参数调优 (`scheduler.rs`)

在 `FunctionToggles` 中新增：
```rust
#[serde(rename = "SchedulerTuning")] pub scheduler_tuning: bool,
```

在 `apply_system_tweaks()` 中新增：
```rust
fn apply_scheduler_tuning(&self) -> Result<()> {
    let config = self.config.read().unwrap();
    if !config.function.scheduler_tuning { return Ok(()); }
    let _ = utils::try_write_file("/proc/sys/kernel/sched_wakeup_granularity_ms", "15");
    let _ = utils::try_write_file("/proc/sys/kernel/sched_migration_cost_ns", "500000");
    let _ = utils::try_write_file("/proc/sys/kernel/sched_nr_migrate", "8");
    log::info!("{}", t("apply-scheduler-tuning"));
    Ok(())
}
```

### 7. 默认配置变更

**`config.yaml`** 新增：
```yaml
function:
  SchedulerTuning: true
  # ... 原有 ...

StillDive:
  enabled: true
  enter_threshold: 0.05
  enter_ticks: 30
  exit_threshold: 0.15
  exit_boost_ticks: 5
  perf_ceil: 0.30
  smoothing_up: 0.05
```

**`idle_dive.yaml`**（新建）：
```yaml
enabled: true
dive_threshold: 0.12
exit_threshold: 0.18
dive_delay_ms: 500
exit_delay_ms: 30
governors:
  normal: "menu"
  diving: "menu"
  doze: "menu"
params:
  normal_latency_us: 100
  diving_latency_us: 800
  doze_latency_us: 1500
```

**`touch_boost.yaml`**（新建）：
```yaml
enabled: true
boost_freqs:
  - 2500000
  - 0
  - 2000000
release_delay_ms: 100
recover_decay: 0.15
min_boost_duration_ms: 50
input_device: ""
```

### 8. i18n 新增键

**zh.ftl**：
```ftl
# --- StillDive ---
clg-still-enter = [CLG] 亮屏静止下潜: 已进入深度省电模式 (perf ≤ { $ceil })
clg-still-exit = [CLG] 亮屏静止下潜: 检测到活动，已退出 (升频助力 { $boost } ticks)

# --- IdleDive ---
idle-dive-init = [IdleDive] CPU 静止下潜控制器初始化完成
idle-dive-init-failed = [IdleDive] 初始化失败: { $error }
idle-dive-unavailable = [IdleDive] cpuidle 节点不可用，CPU 静止下潜已禁用
idle-dive-enter = [IdleDive] 进入下潜状态
idle-dive-exit = [IdleDive] 退出下潜状态
idle-dive-enter-dozed = [IdleDive] 进入息屏下潜状态
idle-dive-exit-dozed = [IdleDive] 退出息屏下潜状态
idle-dive-config-reloaded = [IdleDive] 配置已热重载

# --- TouchBoost ---
touch-boost-init = [TouchBoost] 触摸提频控制器初始化完成
touch-boost-init-failed = [TouchBoost] 初始化失败: { $error }
touch-boost-no-device = [TouchBoost] 未找到触摸设备，TouchBoost 已禁用
touch-boost-listener-started = [TouchBoost] 监听器已启动，监听 { $count } 个设备
touch-boost-thread-started = [TouchBoost] 线程已启动
touch-boost-start = [TouchBoost] 触摸开始，应用 boost
touch-boost-release = [TouchBoost] 松手，开始衰减恢复
touch-boost-recovered = [TouchBoost] 恢复完成
touch-boost-config-reloaded = [TouchBoost] 配置已热重载

# --- Scheduler Tuning ---
apply-scheduler-tuning = 内核调度器节能参数已应用
```

**en.ftl** 对应英文翻译（从复刻版提取）。

### 9. 模块注册与依赖

`src/main.rs` 添加：
```rust
pub mod idle_dive;
pub mod touch_boost;
```

`Cargo.toml`：无需新增依赖（使用现有 `nix`, `libc`, `log`）

---

## 验证方案

### 编译验证
```bash
cargo build   # 确认无编译错误
```

### 功能验证（观察日志）

| 操作 | 预期日志 |
|------|----------|
| 亮屏 + 静止 3s | `[CLG] 亮屏静止下潜: 已进入` |
| | `[IdleDive] 进入下潜状态` |
| 触摸屏幕 | `[TouchBoost] 触摸开始` |
| | `[IdleDive] 快速退出下潜` |
| | `[CLG] 亮屏静止下潜: 检测到活动，已退出` |
| 打开游戏 (FAS) | 无 StillDive/IdleDive/TouchBoost 干扰日志 |
| 息屏 | `[IdleDive] 进入息屏下潜状态` |
| | `[Scheduler] 息屏: 启用极致深度睡眠模式` |
| 亮屏 | `[IdleDive] 退出息屏下潜状态` |
| | `[Scheduler] 亮屏: 恢复之前的性能限制` |

### 回归保证
- `enabled: false` 时行为与修改前完全一致
- FAS 游戏模式不触发任何省电模块干扰（CLG 已 release，IdleDive 维持 normal）
- 息屏 Doze 优先于 StillDive（Doze 时不传 still_dive config）
- 配置热重载：修改 yaml 文件后自动生效

---

## 与复刻版的关键差异

| 维度 | 复刻版 | 本方案 |
|------|--------|--------|
| StillDive | ❌ 不存在 | ✅ CLG 内部 max util 检测 + perf 覆盖 |
| IdleDive 检测 | 仅平均 util | 平均 util（移植） |
| TouchBoost | 独立运行 | ✅ 联动 IdleDive.on_touch_fast_exit() |
| 调度器调优 | ❌ 不存在 | ✅ sched_wakeup_granularity + migration + nr_migrate |
| 三层联动 | ❌ 各管各的 | ✅ StillDive ↔ IdleDive ↔ TouchBoost 协调 |
| 配置热重载 | 部分支持 | ✅ 全部支持 |