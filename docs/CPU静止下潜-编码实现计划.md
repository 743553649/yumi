# CPU 静止下潜 — 编码实现计划

## 总体策略

按模块依赖顺序实现：**配置先行 → StillDive → IdleDive → TouchBoost → 联动集成 → 文档更新**

每个阶段完成后执行 `cargo build` 验证编译。

---

## 阶段 1：配置基础 (1h)

### 目标
添加所有新功能的配置结构体，确保 YAML 能正确解析。

### 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/scheduler/config.rs` | 添加 `StillDiveConfig` 结构体，添加到 `Config` |
| `module/config/config.yaml` | 添加 `StillDive` 和 `SchedulerTuning` 配置块 |
| `module/config/i18n/zh.ftl` | 添加 StillDive/IdleDive/TouchBoost 翻译键 |
| `module/config/i18n/en.ftl` | 添加英文翻译键 |

### 实现细节

**1.1 StillDiveConfig (config.rs)**
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

fn sd_enter_thresh() -> f32 { 0.08 }
fn sd_enter_ticks() -> u32 { 10 }
fn sd_exit_thresh() -> f32 { 0.20 }
fn sd_exit_boost() -> u32 { 5 }
fn sd_perf_ceil() -> f32 { 0.30 }
fn sd_smoothing_up() -> f32 { 0.05 }

impl Default for StillDiveConfig { ... }
impl StillDiveConfig { pub fn normalize(&mut self) { ... } }
```

**1.2 Config 扩展**
```rust
pub struct Config {
    // ... 现有字段 ...
    #[serde(default, rename = "StillDive")]
    pub still_dive: StillDiveConfig,
}
```

**1.3 FunctionToggles 扩展**
```rust
pub struct FunctionToggles {
    // ... 现有字段 ...
    #[serde(default, rename = "SchedulerTuning")]
    pub scheduler_tuning: bool,
}
```

### 验证点
- `cargo build` 通过
- `config.yaml` 能正确解析 StillDive 配置块

---

## 阶段 2：StillDive 实现 (2h)

### 目标
在 CLG 中实现亮屏静止下潜功能。

### 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/scheduler/cpu_load_governor.rs` | 添加 StillDive 状态字段和检测逻辑 |
| `src/scheduler/mod.rs` | 传递 still_dive 配置到 CLG |

### 实现细节

**2.1 CpuLoadGovernor 新增字段**
```rust
pub struct CpuLoadGovernor {
    // ... 现有字段 ...
    still_dive: Option<StillDiveConfig>,
    still_mode: bool,
    still_low_ticks: u32,
    still_exit_boost: u32,
}
```

**2.2 init_policies() 签名变更**
```rust
pub fn init_policies(
    &mut self,
    gov_cfg: &CpuLoadGovernorConfig,
    still_dive: Option<StillDiveConfig>
)
```

**2.3 on_load_update() 逻辑扩展**
```rust
pub fn on_load_update(&mut self, core_utils: &[f32]) {
    // 1. StillDive 检测（在 per-cluster 循环前）
    if let Some(ref sd) = self.still_dive {
        let max_util = core_utils.iter().cloned().fold(0.0_f32, f32::max);
        
        if !self.still_mode {
            // 未下潜状态
            if max_util <= sd.enter_threshold {
                self.still_low_ticks += 1;
            } else {
                self.still_low_ticks = 0;
            }
            if self.still_low_ticks >= sd.enter_ticks {
                self.still_mode = true;
                log::info!("{}", t_with_args("clg-still-enter", ...));
            }
        } else {
            // 已下潜状态
            if max_util > sd.exit_threshold {
                self.still_mode = false;
                self.still_exit_boost = sd.exit_boost_ticks;
                log::info!("{}", t_with_args("clg-still-exit", ...));
            }
        }
        
        // 退出助力递减
        if self.still_exit_boost > 0 {
            self.still_exit_boost -= 1;
        }
    }
    
    // 2. per-cluster 循环（修改 clamp 和 smoothing）
    for (i, cluster) in self.clusters.iter_mut().enumerate() {
        let util = core_utils[i];
        
        let (perf_ceil, smoothing_up) = if self.still_mode {
            // 下潜模式：低上限 + 极慢升频
            (self.still_dive.as_ref().unwrap().perf_ceil, 
             self.still_dive.as_ref().unwrap().smoothing_up)
        } else if self.still_exit_boost > 0 {
            // 退出助力：快速恢复
            (self.cfg.perf_ceil, 1.0)
        } else {
            // 正常模式
            (self.cfg.perf_ceil, self.cfg.smoothing_up)
        };
        
        // ... 现有逻辑，使用 effective_perf_ceil 和 effective_smoothing_up ...
    }
}
```

**2.4 reload_config() 和 release() 扩展**
```rust
pub fn reload_config(&mut self, gov_cfg: &CpuLoadGovernorConfig, still_dive: Option<StillDiveConfig>) {
    // ... 现有逻辑 ...
    self.still_dive = still_dive;
}

pub fn release(&mut self) {
    // ... 现有逻辑 ...
    self.still_mode = false;
    self.still_low_ticks = 0;
    self.still_exit_boost = 0;
}
```

**2.5 scheduler/mod.rs 调用点**
```rust
// 在 ConfigReload 事件处理中
DaemonEvent::ConfigReload(rules_config) => {
    let config = self.config.read().unwrap();
    let still_dive = if config.still_dive.enabled {
        Some(config.still_dive.clone())
    } else {
        None
    };
    cpu_governor.reload_config(&mode.cpu_load_governor, still_dive);
}
```

### 验证点
- `cargo build` 通过
- 日志输出 `[CLG] 亮屏静止下潜: 已进入深度省电模式`
- 日志输出 `[CLG] 亮屏静止下潜: 检测到活动，已退出`

---

## 阶段 3：IdleDive 实现 (3h)

### 目标
从复刻版移植 IdleDive 模块，实现 CPU 空闲下潜。

### 文件创建清单

| 文件 | 说明 |
|------|------|
| `src/idle_dive/mod.rs` | 模块入口，导出 IdleDiveController |
| `src/idle_dive/config.rs` | IdleDiveConfig 结构体 |
| `src/idle_dive/controller.rs` | 状态机实现 |
| `src/idle_dive/latency.rs` | PM-QoS + sysfs 写入器 |
| `module/config/idle_dive.yaml` | 配置文件 |

### 实现细节

**3.1 config.rs**
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

**3.2 controller.rs - 状态机**
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

**3.3 latency.rs - 写入器**
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

**3.4 mod.rs - 模块入口**
```rust
mod config;
mod controller;
mod latency;

pub use config::IdleDiveConfig;
pub use controller::IdleDiveController;
```

**3.5 main.rs 注册**
```rust
mod idle_dive;
```

**3.6 scheduler/mod.rs 集成**
```rust
// 初始化
let idle_dive = if config.idle_dive.enabled {
    IdleDiveController::new(config.idle_dive.clone())?
} else {
    // 创建一个空的控制器，什么都不做
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

### 验证点
- `cargo build` 通过
- 日志输出 `[IdleDive] CPU 静止下潜控制器初始化完成`
- 日志输出 `[IdleDive] 进入下潜状态` / `[IdleDive] 退出下潜状态`
- 息屏时输出 `[IdleDive] 进入息屏下潜状态`

---

## 阶段 4：TouchBoost 实现 (3h)

### 目标
从复刻版移植 TouchBoost 模块，实现触摸提频。

### 文件创建清单

| 文件 | 说明 |
|------|------|
| `src/touch_boost/mod.rs` | 模块入口 |
| `src/touch_boost/config.rs` | TouchBoostConfig 结构体 |
| `src/touch_boost/controller.rs` | Boost 状态机 |
| `src/touch_boost/monitor.rs` | epoll 触摸事件监听器 |
| `module/config/touch_boost.yaml` | 配置文件 |

### 实现细节

**4.1 config.rs**
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

**4.2 monitor.rs - epoll 监听器**
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

**4.3 controller.rs - Boost 控制器**
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

**4.4 mod.rs - 模块入口**
```rust
mod config;
mod controller;
mod monitor;

pub use config::TouchBoostConfig;
pub use controller::TouchBoostController;
pub use monitor::TouchMonitor;
```

**4.5 main.rs 注册**
```rust
mod touch_boost;
```

**4.6 scheduler/mod.rs 集成**
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

// 触摸事件处理
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

### 验证点
- `cargo build` 通过
- 日志输出 `[TouchBoost] 触摸提频控制器初始化完成`
- 日志输出 `[TouchBoost] 触摸开始，应用 boost`
- 日志输出 `[TouchBoost] 松手，开始衰减恢复`

---

## 阶段 5：联动集成与调优 (1h)

### 目标
完成三层联动，添加调度器参数调优。

### 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/scheduler/scheduler.rs` | 添加 `apply_scheduler_tuning()` |
| `src/scheduler/mod.rs` | 完整事件循环整合 |

### 实现细节

**5.1 scheduler.rs - 调度器参数调优**
```rust
fn apply_scheduler_tuning(&self) -> Result<()> {
    let config = self.config.read().unwrap();
    if !config.function.scheduler_tuning {
        return Ok(());
    }
    let _ = utils::try_write_file("/proc/sys/kernel/sched_wakeup_granularity_ms", "15");
    let _ = utils::try_write_file("/proc/sys/kernel/sched_migration_cost_ns", "500000");
    let _ = utils::try_write_file("/proc/sys/kernel/sched_nr_migrate", "8");
    log::info!("{}", t("apply-scheduler-tuning"));
    Ok(())
}
```

**5.2 完整事件循环**
```rust
loop {
    match rx.recv() {
        Ok(event) => match event {
            DaemonEvent::SystemLoadUpdate { core_utils, .. } => {
                cpu_governor.on_load_update(&core_utils);
                if !core_utils.is_empty() {
                    let avg = core_utils.iter().sum::<f32>() / core_utils.len() as f32;
                    idle_dive.update(avg);
                }
                touch_boost.update();  // 衰减恢复
            }
            DaemonEvent::ScreenStateChange(screen_on) => {
                if screen_on {
                    idle_dive.exit_doze();
                } else {
                    idle_dive.enter_doze();
                }
            }
            DaemonEvent::ConfigReload(rules_config) => {
                // 热重载所有模块配置
                let config = self.config.read().unwrap();
                let still_dive = if config.still_dive.enabled {
                    Some(config.still_dive.clone())
                } else {
                    None
                };
                cpu_governor.reload_config(&mode.cpu_load_governor, still_dive);
                idle_dive.reload_config(config.idle_dive.clone());
                touch_boost.reload_config(config.touch_boost.clone());
            }
            _ => {}
        }
        Err(_) => {}
    }
    
    // 处理触摸事件（非阻塞）
    if let Ok(event) = touch_rx.try_recv() {
        match event {
            TouchEvent::Start => {
                touch_boost.on_touch_start();
                idle_dive.on_touch_fast_exit();
            }
            TouchEvent::End => touch_boost.on_touch_end(),
        }
    }
}
```

### 验证点
- `cargo build` 通过
- 日志输出 `内核调度器节能参数已应用`
- 三层联动正常：触摸 → TouchBoost + IdleDive 快速退出

---

## 阶阶段 6：配置文件与文档 (30min)

### 目标
完善配置文件，更新文档。

### 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `module/config/idle_dive.yaml` | 创建配置文件 |
| `module/config/touch_boost.yaml` | 创建配置文件 |
| `module/config/config.yaml` | 添加完整配置示例 |
| `README.md` | 文档化新功能 |
| `docs/工作日志.md` | 记录实现过程 |

### 配置文件内容

**idle_dive.yaml**
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

**touch_boost.yaml**
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

---

## 验证方案

### 编译验证
```bash
cargo build
```

### 功能验证（观察日志）

| 操作 | 预期日志 |
|------|----------|
| 亮屏 + 静止 3s | `[CLG] 亮屏静止下潜: 已进入深度省电模式` |
| | `[IdleDive] 进入下潜状态` |
| 触摸屏幕 | `[TouchBoost] 触摸开始，应用 boost` |
| | `[IdleDive] 快速退出下潜` |
| | `[CLG] 亮屏静止下潜: 检测到活动，已退出` |
| 息屏 | `[IdleDive] 进入息屏下潜状态` |
| 亮屏 | `[IdleDive] 退出息屏下潜状态` |

### 回归保证
- `enabled: false` 时行为与修改前完全一致
- FAS 游戏模式不触发任何省电模块干扰
- 配置热重载正常工作

---

## 风险与注意事项

1. **FastWriter 兼容性** — 复刻版的 `FastWriter::write_value_force_str()` 可能需要适配
2. **epoll 权限** — 监听 `/dev/input/event*` 可能需要 root 权限
3. **PM-QoS 接口** — 不同内核版本接口可能不同，需要兼容处理
4. **配置热重载** — 确保所有模块都支持运行时配置更新

---

## 预计总工时

| 阶段 | 预计时间 |
|------|----------|
| 阶段 1：配置基础 | 1h |
| 阶段 2：StillDive | 2h |
| 阶段 3：IdleDive | 3h |
| 阶段 4：TouchBoost | 3h |
| 阶段 5：联动集成 | 1h |
| 阶段 6：配置文档 | 30min |
| **总计** | **~10.5h** |
