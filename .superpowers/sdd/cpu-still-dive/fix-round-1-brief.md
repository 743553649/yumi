# Fix Round 1: 代码审查 Important 问题修复

## 任务

修复代码审查发现的5个Important问题。

## 问题清单

### 1. Touch事件处理被阻塞在`rx.recv()`内

**文件:** `src/scheduler/mod.rs`
**问题:** `touch_rx.try_recv()` 在 `for msg in rx` 循环内部。`rx.recv()` 是阻塞调用，触摸事件会堆积延迟处理。
**修复:** 在主循环开始时先 drain `touch_rx`（non-blocking），然后再处理 daemon 事件。或者使用 `crossbeam_channel::select!` 同时等待两个 channel。

### 2. config.yaml缺少TouchBoost配置块

**文件:** `module/config/config.yaml`
**问题:** 包含 StillDive 和 IdleDive 配置块，但缺少 TouchBoost。
**修复:** 在 config.yaml 中添加 TouchBoost 配置块：
```yaml
TouchBoost:
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

### 3. SchedulerTuning参数硬编码

**文件:** `src/scheduler/scheduler.rs`
**问题:** 三个内核参数值直接硬编码。
**修复:** 将参数值提取为常量或配置。由于计划中只要求 on/off 开关，可以先提取为命名常量提高可读性。

### 4. IdleDive `reload_config`未调用`normalize()`

**文件:** `src/idle_dive/controller.rs`
**问题:** `reload_config` 直接赋值 `self.config = config`，没有调用 `config.normalize()`。
**修复:** 在 `reload_config` 中添加 `config.normalize()` 调用。

### 5. 删除了CLG算法注释

**文件:** `src/scheduler/cpu_load_governor.rs`
**问题:** 删除了约15行解释性注释（尖峰抑制、headroom ramp、升频速率限制、降频门控等）。
**修复:** 恢复被删除的注释。需要查看原始提交 `5c54e0f` 中的注释内容。

## 验证

- 代码逻辑不变
- 所有5个问题都已修复
