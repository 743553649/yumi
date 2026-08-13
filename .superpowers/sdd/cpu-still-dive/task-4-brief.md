# Task 4: 阶段四 — TouchBoost 实现

## 目标
从复刻版移植 TouchBoost 模块，实现触摸提频。

## 依赖
- Task 3 已完成：IdleDive 模块已实现

## 文件创建清单

| 文件 | 说明 |
|------|------|
| `src/touch_boost/mod.rs` | 模块入口 |
| `src/touch_boost/config.rs` | TouchBoostConfig 结构体 |
| `src/touch_boost/controller.rs` | Boost 状态机 |
| `src/touch_boost/monitor.rs` | epoll 触摸事件监听器 |
| `module/config/touch_boost.yaml` | 配置文件 |

## 实现细节

### 4.1 config.rs

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct TouchBoostConfig {
    #[serde(default = "crate::utils::default_true")]
    pub enabled: bool,
    #[serde(default = "d_boost_freqs")]
    pub boost_freqs: Vec<u32>,
    #[serde(default = "d_release_delay_ms")]
    pub release_delay_ms: u64,
    #[serde(default = "d_recover_decay")]
    pub recover_decay: f32,
    #[serde(default = "d_min_boost_duration_ms")]
    pub min_boost_duration_ms: u64,
    #[serde(default)]
    pub input_device: String,
}
```

实现 Default 和 normalize() 方法。

### 4.2 monitor.rs - epoll 监听器

```rust
pub struct TouchMonitor {
    epoll_fd: RawFd,
    input_fds: Vec<RawFd>,
    config: TouchBoostConfig,
}

impl TouchMonitor {
    pub fn new(config: TouchBoostConfig) -> Result<Self> { ... }
    pub fn run(&self, tx: Sender<TouchEvent>) -> Result<()> { ... }
}
```

使用 epoll 监听 `/dev/input/event*` 触摸事件。

### 4.3 controller.rs - Boost 控制器

```rust
pub struct TouchBoostController {
    config: TouchBoostConfig,
    cluster_writers: Vec<FastWriter>,
    boost_until: Instant,
    is_boosting: bool,
}

impl TouchBoostController {
    pub fn new(config: TouchBoostConfig) -> Result<Self> { ... }
    pub fn on_touch_start(&mut self) { ... }
    pub fn on_touch_end(&mut self) { ... }
    pub fn update(&mut self) { ... }  // 衰减恢复
    pub fn reload_config(&mut self, config: TouchBoostConfig) { ... }
}
```

### 4.4 mod.rs - 模块入口

```rust
mod config;
mod controller;
mod monitor;

pub use config::TouchBoostConfig;
pub use controller::TouchBoostController;
pub use monitor::TouchMonitor;
```

### 4.5 main.rs 注册

在 src/main.rs 中添加：
```rust
mod touch_boost;
```

### 4.6 scheduler/mod.rs 集成

```rust
// 初始化
let (touch_tx, touch_rx) = mpsc::channel::<TouchEvent>();
let touch_boost = if config.touch_boost.enabled {
    let monitor = TouchMonitor::new(config.touch_boost.clone())?;
    std::thread::spawn(move || monitor.run(touch_tx));
    TouchBoostController::new(config.touch_boost.clone())?
} else {
    TouchBoostController::disabled()
};

// 触摸事件处理（在事件循环中）
if let Ok(event) = touch_rx.try_recv() {
    match event {
        TouchEvent::Start => {
            touch_boost.on_touch_start();
            idle_dive.on_touch_fast_exit();
        }
        TouchEvent::End => touch_boost.on_touch_end(),
    }
}

// 定期更新（衰减恢复）
touch_boost.update();
```

### 4.7 touch_boost.yaml 配置文件

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

## 验证点
- `cargo build` 通过
- 日志输出 `[TouchBoost] 触摸提频控制器初始化完成`
- 日志输出 `[TouchBoost] 触摸开始，应用 boost`
- 日志输出 `[TouchBoost] 松手，开始衰减恢复`

## 代码风格要求
- 遵循现有模块组织方式
- 使用 `t` 或 `t_with_args` 进行 i18n 日志
- 不添加任何注释
- 使用 `anyhow::Result` 进行错误处理
- epoll 使用 `nix` 库
