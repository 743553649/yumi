# Task 2: 阶段二 — StillDive 实现

## 目标
在 CLG 中实现亮屏静止下潜功能。

## 依赖
- Task 1 已完成：StillDiveConfig 已添加到 config.rs

## 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/scheduler/cpu_load_governor.rs` | 添加 StillDive 状态字段和检测逻辑 |
| `src/scheduler/mod.rs` | 传递 still_dive 配置到 CLG |

## 实现细节

### 2.1 CpuLoadGovernor 新增字段 (src/scheduler/cpu_load_governor.rs)

```rust
pub struct CpuLoadGovernor {
    // ... 现有字段 ...
    still_dive: Option<StillDiveConfig>,
    still_mode: bool,
    still_low_ticks: u32,
    still_exit_boost: u32,
}
```

### 2.2 init_policies() 签名变更

```rust
pub fn init_policies(
    &mut self,
    gov_cfg: &CpuLoadGovernorConfig,
    still_dive: Option<StillDiveConfig>
)
```

在初始化时设置 `self.still_dive = still_dive;`

### 2.3 on_load_update() 逻辑扩展

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
                log::info!("{}", t_with_args("clg-still-enter", &fluent_args!(
                    "ceil" => format!("{:.0}%", sd.perf_ceil * 100.0)
                )));
            }
        } else {
            // 已下潜状态
            if max_util > sd.exit_threshold {
                self.still_mode = false;
                self.still_exit_boost = sd.exit_boost_ticks;
                log::info!("{}", t_with_args("clg-still-exit", &fluent_args!(
                    "boost" => sd.exit_boost_ticks.to_string()
                )));
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
        
        // 计算有效的 perf_ceil 和 smoothing_up
        let effective_perf_ceil = if self.still_mode {
            self.still_dive.as_ref().unwrap().perf_ceil
        } else {
            self.cfg.perf_ceil
        };
        
        let effective_smoothing_up = if self.still_mode {
            self.still_dive.as_ref().unwrap().smoothing_up
        } else if self.still_exit_boost > 0 {
            1.0  // 退出助力：快速恢复
        } else {
            self.cfg.smoothing_up
        };
        
        // ... 现有逻辑，使用 effective_perf_ceil 和 effective_smoothing_up ...
        // 注意：perf_floor 在下潜模式下设为 0.0
        let effective_perf_floor = if self.still_mode { 0.0 } else { self.cfg.perf_floor };
    }
}
```

### 2.4 reload_config() 扩展

```rust
pub fn reload_config(&mut self, gov_cfg: &CpuLoadGovernorConfig, still_dive: Option<StillDiveConfig>) {
    // ... 现有逻辑 ...
    self.still_dive = still_dive;
}
```

### 2.5 release() 扩展

```rust
pub fn release(&mut self) {
    // ... 现有逻辑 ...
    self.still_mode = false;
    self.still_low_ticks = 0;
    self.still_exit_boost = 0;
}
```

### 2.6 scheduler/mod.rs 调用点

在 `ConfigReload` 事件处理中：
```rust
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

在初始化时：
```rust
let still_dive = if config.still_dive.enabled {
    Some(config.still_dive.clone())
} else {
    None
};
cpu_governor.init_policies(&mode.cpu_load_governor, still_dive);
```

## 验证点
- `cargo build` 通过
- 日志输出 `[CLG] 亮屏静止下潜: 已进入深度省电模式`
- 日志输出 `[CLG] 亮屏静止下潜: 检测到活动，已退出`

## 代码风格要求
- 遵循现有 CLG 代码的风格
- 使用 `t_with_args` 宏进行带参数的日志翻译
- 不添加任何注释
- 保持现有函数签名的兼容性（除了必要的参数添加）
