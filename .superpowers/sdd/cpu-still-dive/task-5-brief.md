# Task 5: 阶段五 — 联动集成与调优

## 目标
完成三层联动，添加调度器参数调优。

## 依赖
- Task 3 已完成：IdleDive 模块已实现
- Task 4 已完成：TouchBoost 模块已实现

## 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/scheduler/scheduler.rs` | 添加 `apply_scheduler_tuning()` |
| `src/scheduler/mod.rs` | 完整事件循环整合 |

## 实现细节

### 5.1 scheduler.rs - 调度器参数调优

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

在 `apply_system_tweaks()` 中调用 `apply_scheduler_tuning()`。

### 5.2 完整事件循环 (scheduler/mod.rs)

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

## 验证点
- `cargo build` 通过
- 日志输出 `内核调度器节能参数已应用`
- 三层联动正常：触摸 → TouchBoost + IdleDive 快速退出

## 代码风格要求
- 遵循现有事件循环模式
- 使用 `t` 或 `t_with_args` 进行 i18n 日志
- 不添加任何注释
