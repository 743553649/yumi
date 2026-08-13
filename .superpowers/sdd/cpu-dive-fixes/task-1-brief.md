# Task 1+3 简报：修复 IdleDive 逻辑问题

## 任务概述
修复 `src/idle_dive/controller.rs` 中的两个问题：
1. dive_timer 逻辑 bug
2. PM-QoS 错误被静默忽略

## 问题 1：dive_timer 逻辑 bug

### 当前代码位置
`src/idle_dive/controller.rs` 的 `update()` 方法，第 75-99 行

### 问题描述
```rust
// 当前代码（有问题）
IdleDiveState::Normal => {
    if avg_util < self.config.dive_threshold {
        if self.dive_timer.elapsed().as_millis() as u64 >= self.config.dive_delay_ms {
            self.transition_to(IdleDiveState::Diving);
        }
    } else {
        self.dive_timer = Instant::now();  // ✅ 这里重置了
    }
}
IdleDiveState::Diving => {
    if avg_util > self.config.exit_threshold {
        if self.exit_timer.elapsed().as_millis() as u64 >= self.config.exit_delay_ms {
            self.transition_to(IdleDiveState::Normal);
            // ❌ 问题：这里没有重置 dive_timer！
        }
    } else {
        self.exit_timer = Instant::now();
    }
}
```

### 修复方案
在 `transition_to(IdleDiveState::Normal)` 时，`dive_timer` 会在 `transition_to` 方法中被重置（第 134 行），所以实际上已经修复了。

**但是**，需要检查：当从 Diving 状态退出时，`dive_timer` 是否被正确重置？

查看 `transition_to` 方法：
```rust
IdleDiveState::Normal => {
    info!("{}", t("idle-dive-exit"));
    let _ = self.latency_writer.set_governor(&self.config.governors.normal);
    let _ = self.latency_writer.set_latency(self.config.params.normal_latency_us);
    self.dive_timer = Instant::now();  // ✅ 这里已经重置了
}
```

**结论**: 问题 1 实际上已经被正确处理了！`transition_to(Normal)` 会重置 `dive_timer`。

**验证任务**: 确认代码逻辑是否正确，如果正确则无需修改。

## 问题 2：PM-QoS 错误被静默忽略

### 当前代码位置
`src/idle_dive/controller.rs` 的 `transition_to()` 方法，第 126-150 行

### 问题描述
```rust
let _ = self.latency_writer.set_governor(&self.config.governors.normal);  // ❌ 忽略错误
let _ = self.latency_writer.set_latency(self.config.params.normal_latency_us);  // ❌ 忽略错误
```

### 修复方案
使用 `if let Err(e)` 模式记录警告日志：

```rust
if let Err(e) = self.latency_writer.set_governor(&self.config.governors.normal) {
    log::warn!("Failed to set governor: {}", e);
}
if let Err(e) = self.latency_writer.set_latency(self.config.params.normal_latency_us) {
    log::warn!("Failed to set latency: {}", e);
}
```

## 执行要求
1. 仔细检查 dive_timer 逻辑，确认是否真的有问题
2. 如果有问题，修复它
3. 为 PM-QoS 错误添加警告日志
4. 不要添加不必要的注释
5. 保持现有代码风格

## 验证
修改完成后，运行 `cargo check` 确保编译通过（如果有权限问题则跳过）。

## 报告格式
请将完整报告写入 `/.superpowers/sdd/cpu-dive-fixes/task-1-report.md`，包含：
1. 对 dive_timer 逻辑的分析
2. PM-QoS 错误处理的修改
3. 修改的代码差异
4. 验证结果
