# Task 1: 阶段一 — 配置基础

## 目标
添加所有新功能的配置结构体，确保 YAML 能正确解析。

## 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/scheduler/config.rs` | 添加 `StillDiveConfig` 结构体，添加到 `Config`；`FunctionToggles` 添加 `scheduler_tuning` |
| `module/config/config.yaml` | 添加 `StillDive` 和 `SchedulerTuning` 配置块 |
| `module/config/i18n/zh.ftl` | 添加 StillDive/IdleDive/TouchBoost 翻译键 |
| `module/config/i18n/en.ftl` | 添加英文翻译键 |

## 实现细节

### 1.1 StillDiveConfig (src/scheduler/config.rs)

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct StillDiveConfig {
    #[serde(default = "crate::utils::default_true")]
    pub enabled: bool,
    #[serde(default = "sd_enter_thresh")]
    pub enter_threshold: f32,
    #[serde(default = "sd_enter_ticks")]
    pub enter_ticks: u32,
    #[serde(default = "sd_exit_thresh")]
    pub exit_threshold: f32,
    #[serde(default = "sd_exit_boost")]
    pub exit_boost_ticks: u32,
    #[serde(default = "sd_perf_ceil")]
    pub perf_ceil: f32,
    #[serde(default = "sd_smoothing_up")]
    pub smoothing_up: f32,
}
```

默认值函数：
```rust
fn sd_enter_thresh() -> f32 { 0.08 }
fn sd_enter_ticks() -> u32 { 10 }
fn sd_exit_thresh() -> f32 { 0.20 }
fn sd_exit_boost() -> u32 { 5 }
fn sd_perf_ceil() -> f32 { 0.30 }
fn sd_smoothing_up() -> f32 { 0.05 }
```

实现 `Default` 和 `normalize()` 方法（参考现有配置结构体的模式）。

### 1.2 Config 扩展

在 `Config` 结构体中添加：
```rust
#[serde(default, rename = "StillDive")]
pub still_dive: StillDiveConfig,
```

### 1.3 FunctionToggles 扩展

在 `FunctionToggles` 结构体中添加：
```rust
#[serde(default, rename = "SchedulerTuning")]
pub scheduler_tuning: bool,
```

### 1.4 config.yaml 配置

```yaml
function:
  SchedulerTuning: true
  # ... 原有字段 ...

StillDive:
  enabled: true
  enter_threshold: 0.08
  enter_ticks: 10
  exit_threshold: 0.20
  exit_boost_ticks: 5
  perf_ceil: 0.30
  smoothing_up: 0.05
```

### 1.5 i18n 翻译键

**zh.ftl** 添加：
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

**en.ftl** 添加对应的英文翻译。

## 验证点
- `cargo build` 通过
- 配置文件能正确解析

## 代码风格要求
- 遵循现有配置结构体的定义模式（derive 组合、serde 属性、默认值函数）
- 使用 `crate::utils::default_true` 作为布尔默认值
- 所有浮点字段在 `normalize()` 中做 `is_finite()` 校验
- 不添加任何注释
