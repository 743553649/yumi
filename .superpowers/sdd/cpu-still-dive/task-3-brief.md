# Task 3: 阶段三 — IdleDive 实现

## 目标
从复刻版移植 IdleDive 模块，实现 CPU 空闲下潜。

## 依赖
- Task 1 已完成：StillDiveConfig 已添加
- Task 2 已完成：CLG StillDive 逻辑已实现

## 文件创建清单

| 文件 | 说明 |
|------|------|
| `src/idle_dive/mod.rs` | 模块入口，导出 IdleDiveController |
| `src/idle_dive/config.rs` | IdleDiveConfig 结构体 |
| `src/idle_dive/controller.rs` | 状态机实现 |
| `src/idle_dive/latency.rs` | PM-QoS + sysfs 写入器 |
| `module/config/idle_dive.yaml` | 配置文件 |

## 实现细节

### 3.1 config.rs

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct IdleDiveConfig {
    #[serde(default = "crate::utils::default_true")]
    pub enabled: bool,
    #[serde(default = "d_dive_threshold")]
    pub dive_threshold: f32,
    #[serde(default = "d_exit_threshold")]
    pub exit_threshold: f32,
    #[serde(default = "d_dive_delay_ms")]
    pub dive_delay_ms: u64,
    #[serde(default = "d_exit_delay_ms")]
    pub exit_delay_ms: u64,
    #[serde(default)]
    pub governors: IdleDiveGovernors,
    #[serde(default)]
    pub params: IdleDiveParams,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IdleDiveGovernors {
    pub normal: String,
    pub diving: String,
    pub doze: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IdleDiveParams {
    pub normal_latency_us: i32,
    pub diving_latency_us: i32,
    pub doze_latency_us: i32,
}
```

实现 Default 和 normalize() 方法。

### 3.2 controller.rs - 状态机

```rust
pub enum IdleDiveState {
    Normal,
    Diving,
    DozeDiving,
}

pub struct IdleDiveController {
    config: IdleDiveConfig,
    state: IdleDiveState,
    latency_writer: LatencyWriter,
    dive_timer: Instant,
    exit_timer: Instant,
    low_util_ticks: u32,
    high_util_ticks: u32,
}

impl IdleDiveController {
    pub fn new(config: IdleDiveConfig) -> Result<Self> { ... }
    pub fn update(&mut self, avg_util: f32) { ... }
    pub fn enter_doze(&mut self) { ... }
    pub fn exit_doze(&mut self) { ... }
    pub fn on_touch_fast_exit(&mut self) { ... }
    pub fn reload_config(&mut self, config: IdleDiveConfig) { ... }
}
```

状态机逻辑：
- Normal → Diving: avg_util < dive_threshold 持续 dive_delay_ms
- Diving → Normal: avg_util > exit_threshold 持续 exit_delay_ms
- Diving → DozeDiving: 收到 enter_doze() 调用
- DozeDiving → Diving: 收到 exit_doze() 调用
- 任何状态 → Normal: 收到 on_touch_fast_exit() 调用

### 3.3 latency.rs - 写入器

```rust
pub struct LatencyWriter {
    pm_qos_fd: Option<RawFd>,
    governor_paths: Vec<String>,
    latency_paths: Vec<String>,
}

impl LatencyWriter {
    pub fn new() -> Result<Self> { ... }
    pub fn set_governor(&self, governor: &str) -> Result<()> { ... }
    pub fn set_latency(&self, latency_us: i32) -> Result<()> { ... }
}
```

使用 PM-QoS 和 sysfs 双通道写入。

### 3.4 mod.rs - 模块入口

```rust
mod config;
mod controller;
mod latency;

pub use config::IdleDiveConfig;
pub use controller::IdleDiveController;
```

### 3.5 main.rs 注册

在 src/main.rs 中添加：
```rust
mod idle_dive;
```

### 3.6 scheduler/mod.rs 集成

```rust
// 初始化
let idle_dive = if config.idle_dive.enabled {
    IdleDiveController::new(config.idle_dive.clone())?
} else {
    IdleDiveController::disabled()
};

// SystemLoadUpdate 事件处理
DaemonEvent::SystemLoadUpdate { core_utils, .. } => {
    cpu_governor.on_load_update(&core_utils);
    if !core_utils.is_empty() {
        let avg = core_utils.iter().sum::<f32>() / core_utils.len() as f32;
        idle_dive.update(avg);
    }
}

// ScreenStateChange 事件处理
DaemonEvent::ScreenStateChange(screen_on) => {
    if screen_on {
        idle_dive.exit_doze();
    } else {
        idle_dive.enter_doze();
    }
}
```

### 3.7 idle_dive.yaml 配置文件

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

## 验证点
- `cargo build` 通过
- 日志输出 `[IdleDive] CPU 静止下潜控制器初始化完成`
- 日志输出 `[IdleDive] 进入下潜状态` / `[IdleDive] 退出下潜状态`
- 息屏时输出 `[IdleDive] 进入息屏下潜状态`

## 代码风格要求
- 遵循现有模块组织方式（mod.rs 入口 + 子模块）
- 使用 `t` 或 `t_with_args` 进行 i18n 日志
- 不添加任何注释
- 使用 `anyhow::Result` 进行错误处理
