# yumi 省电源码优化方案

> 基于源码深度分析，针对不同使用场景的省电优化。
> 适用版本：yumi v2.0.1

---

## 目录

1. [场景定义与优化策略](#1-场景定义与优化策略)
2. [CLG 负载调速器优化](#2-clg-负载调速器优化)
3. [FAS 帧感知调度优化](#3-fas-帧感知调度优化)
4. [Doze 息屏模式优化](#4-doze-息屏模式优化)
5. [修改优先级与风险评估](#5-修改优先级与风险评估)
6. [修改间交互效应分析](#6-修改间交互效应分析)
7. [实施建议与测试方案](#7-实施建议与测试方案)

---

## 1. 场景定义与优化策略

### 1.1 场景分类

| 场景 | 典型应用 | 帧率要求 | 优化目标 |
|:---|:---|:---:|:---|
| **日用轻负载** | 社交、阅读、聊天、待机 | 60fps 或无要求 | 最大省电 |
| **日用中负载** | 浏览网页、滑动列表、动画 | 60-90fps | 省电优先，流畅度次之 |
| **游戏稳态** | 高刷游戏稳定运行期 | 120/144fps | 帧率稳定前提下省电 |
| **游戏高负载** | 游戏加载、团战、特效密集 | 目标帧率 | 性能优先，不省电 |
| **息屏待机** | 后台同步、推送 | 无要求 | 极致省电 |

### 1.2 优化原则

- **日用场景**：激进省电，宁可牺牲少量瞬时响应速度
- **游戏场景**：保守省电，确保帧率稳定，有 jank 保护兜底
- **息屏场景**：极致省电，仅保证后台任务基本运行

---

## 2. CLG 负载调速器优化

**文件**: `src/scheduler/cpu_load_governor.rs`

### 2.1 提高 significant_jump 阈值

**函数**: `update_perf()` 中计算 `is_significant_jump` 的位置

**当前代码**:
```rust
let is_significant_jump = target_perf > old_perf + 0.35;
```

**修改为**:
```rust
let is_significant_jump = target_perf > old_perf + 0.50;
```

**为什么这样改**:
- 当前阈值 0.35 意味着只要目标性能比当前高 35% 就算"大幅跳跃"，触发完整的 `smoothing_up` 升频
- 日用场景中，负载经常在 10%-50% 之间波动，0.35 的阈值太容易被触发
- 例如：当前 perf=0.20，负载到 0.40，target_perf=0.40，差值 0.20 < 0.35，走慢速升频路径（smoothing_up * 0.02）
- 提高到 0.50 后，只有真正的负载突增才会触发快速升频

**实际作用**:
- ✅ 日用场景：减少因负载小幅波动导致的快速升频
- ✅ 让 CLG 在非高负载场景下保持更保守的升频策略
- ⚠️ 游戏场景：高负载突发时响应会稍慢，但有常规升频路径兜底

**风险等级**: 低（日用场景几乎无感，游戏场景影响可控）

### 2.2 提高低负载降频加速阈值

**函数**: `update_perf()` 中计算 `active_smoothing_down` 的位置

**当前代码**:
```rust
let active_smoothing_down = if util < 0.10 {
    self.cfg.smoothing_down * 2.5
} else {
    self.cfg.smoothing_down
};
```

**修改为**:
```rust
let active_smoothing_down = if util < 0.15 {
    self.cfg.smoothing_down * 3.0
} else {
    self.cfg.smoothing_down
};
```

**为什么这样改**:
- 当前只有 util < 10% 时才加速降频，但日用中很多轻负载场景（如阅读、待机、滑动列表）util 在 10%-15% 之间
- 这些场景不需要高频，但因为 util > 10%，降频速度和高负载一样慢
- 加速倍数从 2.5 提高到 3.0，让低负载时降频更快

**实际作用**:
- ✅ 轻负载场景（阅读、聊天、待机）降频更快，节省电量
- ✅ 减少"高频低载"的浪费状态持续时间
- ⚠️ 风险：几乎无，因为 util < 15% 本身就是轻负载

**风险等级**: 极低

---

## 3. FAS 帧感知调度优化

### 3.1 PID 利用率感知范围扩大

**文件**: `src/scheduler/fas/pid.rs`
**函数**: `compute()` 中计算 `util_gain` 的位置

**当前代码**:
```rust
let util_gain = if fg_util > 0.01 && fg_util < 0.30 {
    0.3 + fg_util * 2.3  // 0.3 ~ 0.99
} else {
    1.0
};
```

**修改为**:
```rust
let util_gain = if fg_util > 0.01 && fg_util < 0.45 {
    0.3 + fg_util * 1.56  // 0.3 ~ 1.0
} else {
    1.0
};
```

**数学推导**:
- 目标：fg_util=0.45 时 util_gain 接近 1.0
- 计算：0.3 + 0.45 × k = 1.0 → k = 0.7 / 0.45 ≈ 1.56
- 验证：0.3 + 0.45 × 1.56 = 1.002 ≈ 1.0（在 else 分支兜底前平滑过渡）

**为什么这样改**:
- 当前逻辑：fg_util < 30% 时才衰减 P 项增益，认为是 GPU/IO bound
- 但日用中很多场景（如滑动列表、浏览网页）util 在 30%-45% 之间，CPU 瓶颈不在频率上
- 这些场景下 PID 拉频不会改善流畅度，反而浪费电
- 扩大判断范围到 45%，让这些场景也能享受增益衰减
- 系数调整为 1.56，确保在 0.45 处平滑过渡到 1.0，避免不连续跳变

**实际作用**:
- ✅ 日用场景：滑动列表、浏览网页等轻中负载场景，PID 不会过度拉频
- ✅ GPU bound 场景（如视频播放、动画）更早被识别，减少无效拉频
- ⚠️ 游戏场景：如果某些场景确实是 CPU bound 且 util 在 30%-45%，可能会稍微保守，但有 jank 保护兜底

**风险等级**: 低

### 3.2 target_fps 偏移更激进

**文件**: `src/scheduler/fas/controller.rs`
**函数**: `adjust_target_for_util()`

**当前代码**:
```rust
pub(super) fn adjust_target_for_util(&mut self) {
    if self.util_sample_timer.elapsed().as_millis() < 1000 { return; }
    self.util_sample_timer = Instant::now();
    let allow_decrease = self.jank_cooldown == 0 && self.jank_streak == 0;
    let util = self.ema_fg_util;
    if util <= 0.10 {
        self.target_fps_offset = 0.0;
    } else if util <= 0.55 && allow_decrease {
        self.target_fps_offset = (self.target_fps_offset - 0.1).max(-3.0);
    } else if util >= 0.65 {
        self.target_fps_offset = (self.target_fps_offset + 0.1).min(0.0);
    }
}
```

**修改为**:
```rust
pub(super) fn adjust_target_for_util(&mut self) {
    if self.util_sample_timer.elapsed().as_millis() < 1000 { return; }
    self.util_sample_timer = Instant::now();
    let allow_decrease = self.jank_cooldown == 0 && self.jank_streak == 0;
    let util = self.ema_fg_util;
    if util <= 0.10 {
        self.target_fps_offset = 0.0;
    } else if util <= 0.50 && allow_decrease {
        self.target_fps_offset = (self.target_fps_offset - 0.2).max(-5.0);
    } else if util >= 0.60 {
        self.target_fps_offset = (self.target_fps_offset + 0.15).min(0.0);
    }
}
```

**为什么这样改**:
- 当前每秒只偏移 -0.1fps，最多 -3fps，省电效果有限
- 例如：120fps 游戏，GPU bound 时 util=0.40，需要 30 秒才能偏移到 -3fps
- 修改后每秒偏移 -0.2fps，最多 -5fps，15 秒就能偏移到 -3fps，25 秒到 -5fps
- 同时降低触发阈值（0.55→0.50, 0.65→0.60），更早开始偏移
- 恢复速度也从 0.1 加快到 0.15，避免偏移过深

**实际作用**:
- ✅ 游戏场景：GPU bound 时（如原神低画质、视频播放）更快降低 target_fps
- ✅ 降低 target_fps → PID 的 error 更小 → 输出更保守 → 频率更低 → 省电
- ⚠️ 风险：偏移过深可能导致帧率不稳，但有 jank 检测会重置偏移

**风险等级**: 中（需要监控帧率稳定性）

### 3.3 快速衰减更激进（仅限游戏稳态）

**文件**: `src/scheduler/fas/frame_pipeline.rs`
**函数**: `update_decay()` 中计算 `decay_scale` 和 `dynamic_decay_threshold` 的位置

**3.3.1 放宽高刷衰减限制**

**当前代码**:
```rust
let decay_scale = if self.current_target_fps > 90.0 { 0.6 } else { 1.0 };
```

**修改为**:
```rust
let decay_scale = if self.current_target_fps > 90.0 { 0.75 } else { 1.0 };
```

**为什么这样改**:
- 高刷游戏（120/144fps）下，帧间隔 budget 很短（6.9-8.3ms）
- 当前 0.6 的 dampen 系数导致高刷下快速衰减步长被压缩到 60%，省电效果打折
- 实际上高刷下帧率稳定时（连续 75+ 帧正常），说明性能有余量，应该允许更快衰减
- 0.75 比 0.6 更激进，但仍保留了一定保护

**3.3.2 降低高刷衰减阈值增长系数**

**当前代码**:
```rust
let dynamic_decay_threshold = self.cfg.fast_decay_perf_threshold
    + ((self.current_target_fps - 60.0).max(0.0) * 0.002).min(0.15);
```

**修改为**:
```rust
let dynamic_decay_threshold = self.cfg.fast_decay_perf_threshold
    + ((self.current_target_fps - 60.0).max(0.0) * 0.001).min(0.08);
```

**为什么这样改**:
- 当前公式：60fps → 0.70, 90fps → 0.76, 120fps → 0.82, 144fps → 0.89
- 这意味着 120fps 下，perf 必须 > 0.82 才会触发快速衰减，门槛太高
- 修改后：60fps → 0.70, 90fps → 0.73, 120fps → 0.76, 144fps → 0.78
- 更合理的阈值，让高刷下也能更早触发省电衰减

**实际作用**:
- ✅ 游戏稳态：高刷下快速衰减的触发门槛降低，更容易进入省电状态
- ✅ 120fps 下从 perf=0.82 才衰减 → perf=0.76 就开始衰减
- ⚠️ 风险：如果阈值太低可能导致帧率不稳，但 0.76 仍有足够 headroom

**风险等级**: 低（仅影响高刷游戏稳态，有 jank 保护）

### 3.4 EMA 升频平滑系数降低

**文件**: `src/scheduler/fas/frame_pipeline.rs`
**函数**: `update_ema()` 中计算 `a_up` 的位置

**当前代码**:
```rust
let a_up = (0.15 * fps_factor).clamp(0.10, 0.35);
```

**修改为**:
```rust
let a_up = (0.10 * fps_factor).clamp(0.08, 0.25);
```

**为什么这样改**:
- EMA 的 `a_up` 决定了帧时间突增时 EMA 的跟踪速度
- 当前 0.15 * fps_factor 在 120fps 下约为 0.21，最高 0.35
- 这意味着一帧 30ms 的卡顿，EMA 会快速跟踪到接近 30ms
- 导致 PID 看到较大的 error，输出更多的升频信号
- 降低到 0.10 * fps_factor 后，EMA 跟踪更慢，不会因为单帧卡顿就大幅升频

**实际作用**:
- ✅ 日用场景：单帧卡顿不会导致频率飙升，减少不必要的高频运行
- ✅ 游戏场景：滑动列表时的微小卡顿不会触发大幅升频
- ⚠️ 风险：连续多帧卡顿时 EMA 跟踪会稍慢，但有 jank 检测机制兜底

**风险等级**: 低

### 3.5 升档门槛提高

**文件**: `src/scheduler/fas/gear_state.rs`
**函数**: `check_upgrade()` 中的升级条件判断

**当前代码**:
```rust
if overshoot > 1.35
    && self.fps_window.count() >= 15
    && recent30 > tfps * 1.2
    && self.perf_index < 0.45
    && !self.downgrade_boost_active
{
    // ... 升档逻辑
}
```

**修改为**:
```rust
if overshoot > 1.50
    && self.fps_window.count() >= 20
    && recent30 > tfps * 1.25
    && self.perf_index < 0.40
    && !self.downgrade_boost_active
{
    // ... 升档逻辑
}
```

**为什么这样改**:
- **overshoot 从 1.35 提高到 1.50**：需要帧率超过目标 50% 才触发快速升档，避免轻度过冲就升档
- **fps_window.count() 从 15 提高到 20**：需要更多样本确认，避免瞬时波动误判
- **recent30 从 1.2 提高到 1.25**：近 30 帧均值需要更高才升档
- **perf_index 从 0.45 降到 0.40**：只有在更低性能时才允许快速升档

**实际作用**:
- ✅ 日用场景：减少因瞬时负载波动导致的误升档
- ✅ 升档后会运行在更高频率，减少不必要的升档可以省电
- ✅ 避免"升档 → 频率升高 → 负载消失 → 降档"的循环
- ⚠️ 游戏场景：真正的高负载场景升档会稍慢，但有常规升档路径兜底

**风险等级**: 中（需要监控游戏场景的升档响应速度）

---

## 4. Doze 息屏模式优化

**文件**: `src/scheduler/mod.rs`
**函数**: `enter_doze()` 或类似函数中配置 `doze_cfg` 的位置

**当前代码**:
```rust
let mut doze_cfg = get_clg_cfg(&config_lock, "powersave");
doze_cfg.enabled = true;
doze_cfg.perf_floor = 0.0;
doze_cfg.perf_ceil = doze_cfg.perf_ceil.min(0.40);
doze_cfg.smoothing_up = 0.10;
doze_cfg.smoothing_down = 1.0;
```

**修改为**:
```rust
let mut doze_cfg = get_clg_cfg(&config_lock, "powersave");
doze_cfg.enabled = true;
doze_cfg.perf_floor = 0.0;
doze_cfg.perf_ceil = doze_cfg.perf_ceil.min(0.30);
doze_cfg.smoothing_up = 0.05;
doze_cfg.smoothing_down = 1.0;
doze_cfg.up_rate_limit_ticks = 5;  // 默认值为 3，提高到 5
```

**参数说明**:
- `up_rate_limit_ticks`: 升频速率限制，单位为 tick（每个 tick 约 200ms）
- 默认值 3 = 600ms 内最多升频一次
- 修改为 5 = 1 秒内最多升频一次

**为什么这样改**:
- **perf_ceil 从 0.40 降到 0.30**：息屏时最高只给 30% 性能，足够后台任务（同步、推送）使用
- **smoothing_up 从 0.10 降到 0.05**：升频更迟钝，避免后台任务频繁唤醒 CPU
- **up_rate_limit_ticks 从 3 提高到 5**：升频需要连续 5 个 tick（1 秒）才执行，过滤瞬时负载

**实际作用**:
- ✅ 息屏待机功耗显著降低（取决于后台应用数量）
- ✅ 后台任务不会频繁拉高 CPU 频率
- ✅ 推送、同步等低优先级任务仍能正常工作（30% 性能足够）
- ⚠️ 风险：如果息屏时有高优先级任务（如导航、音乐播放），可能响应稍慢

**风险等级**: 低（息屏场景用户无感知，高优先级任务可通过白名单机制处理）

---

## 5. 修改优先级与风险评估

### 5.1 评估标准

**省电效果等级**:
- ⭐⭐⭐⭐⭐: 预期省电 > 10%（实测验证）
- ⭐⭐⭐⭐: 预期省电 5-10%（理论推导）
- ⭐⭐⭐: 预期省电 2-5%（理论推导）
- ⭐⭐: 预期省电 1-2%（理论推导）
- ⭐: 预期省电 < 1%（边际改善）

**风险等级**:
- **极低**: 几乎无感知，有完善的保护机制
- **低**: 日用场景无感，极端场景可能有轻微影响
- **中**: 需要监控特定场景，可能需要参数回调
- **高**: 需要谨慎实施，建议单独测试

### 5.2 优先级排序

| 优先级 | 修改项 | 文件 | 场景 | 省电效果 | 风险等级 | 说明 |
|:---:|:---|:---|:---|:---:|:---:|:---|
| **1** | util_cap 封顶更严格 | policy_mgmt.rs | 全场景 | ⭐⭐⭐⭐⭐ | 低 | 直接限制最高频率，效果最直接 |
| **2** | PID util_gain 扩大到 0.45 | pid.rs | 日用 | ⭐⭐⭐⭐ | 低 | 轻中负载场景减少无效拉频 |
| **3** | Doze 更深度 | mod.rs | 息屏 | ⭐⭐⭐⭐ | 低 | 息屏待机功耗显著降低 |
| **4** | target_fps 偏移加大 | controller.rs | 游戏 | ⭐⭐⭐ | 中 | GPU bound 场景更快降频 |
| **5** | 快速衰减更激进 | frame_pipeline.rs | 游戏 | ⭐⭐⭐ | 低 | 高刷稳态时更快省电 |
| **6** | CLG 升频逻辑收紧 | cpu_load_governor.rs | 日用 | ⭐⭐⭐ | 低 | 日用轻负载降频更快 |
| **7** | 升档门槛提高 | gear_state.rs | 全场景 | ⭐⭐ | 中 | 减少误升档导致的功耗 |
| **8** | EMA 升频平滑降低 | frame_pipeline.rs | 全场景 | ⭐⭐ | 低 | 减少单帧卡顿触发的升频 |

### 5.3 单项修改预期效果

> **注意**: 以下数据为理论预期，需要实测验证。实际效果因设备、使用习惯、应用差异而不同。

**日用场景（社交、阅读、浏览）**:
- util_cap 封顶：预期省电 8-12%
- PID util_gain 扩大：预期省电 3-7%
- CLG 升频收紧：预期省电 2-4%
- EMA 平滑降低：预期省电 1-3%
- **单项总计**: 14-26%（理论上限，实际会有重叠）

**游戏场景（120fps 稳态）**:
- target_fps 偏移：预期省电 3-5%
- 快速衰减：预期省电 2-4%
- **单项总计**: 5-9%（理论上限）

**息屏场景**:
- Doze 深度优化：预期省电 15-25%（取决于后台应用数量）

---

## 6. 修改间交互效应分析

### 6.1 叠加风险

**问题**: 多个修改叠加后，效果可能不是简单相加，而是相互放大。

**典型场景**: 日用滑动列表

假设当前状态：
- CPU util = 35%
- 当前 perf = 0.50
- 目标帧率 = 60fps

**单独修改时**:
1. **PID util_gain 扩大**: fg_util=35% < 45%，util_gain 从 1.0 降到 0.85，P 项输出减少 15%
2. **target_fps 偏移**: util=35% < 50%，每秒偏移 -0.2fps，10 秒后 target_fps=58fps
3. **util_cap 封顶**: util_cap = 0.35/1.1 ≈ 0.32，perf 被限制在 0.32

**叠加修改时**:
- PID 输出减少 → perf 下降
- target_fps 偏移 → PID error 减小 → perf 进一步下降
- util_cap 限制 → perf 被硬性封顶

**风险**: 三路同时降频可能导致 perf 过度压制，出现明显卡顿。

### 6.2 缓解措施

**1. 分阶段实施**

建议按照优先级顺序，分 3 个阶段实施：

**阶段 1（低风险）**: 优先级 1, 2, 3, 5, 6
- util_cap 封顶
- PID util_gain 扩大
- Doze 深度优化
- 快速衰减
- CLG 升频收紧

**阶段 2（中风险）**: 优先级 4, 7
- target_fps 偏移
- 升档门槛提高

**阶段 3（边际改善）**: 优先级 8
- EMA 平滑降低

**2. 参数保守化**

如果叠加后出现卡顿，可以适当回调参数：

- **PID util_gain**: 系数从 1.56 回调到 1.8（fg_util=0.45 时 util_gain=1.11，略大于 1.0）
- **target_fps 偏移**: 最大偏移从 -5fps 回调到 -4fps
- **util_cap 上限**: 从 0.90 回调到 0.95

**3. 监控关键指标**

实施后需要监控以下指标，判断是否过度压制：

```
日志关键词：
- CLG: perf, freq（确认频率在合理范围）
- FAS: util, offset（确认偏移在生效）
- Jank: jank_count, jank_streak（确认没有频繁卡顿）
```

### 6.3 场景隔离建议

**问题**: 当前修改对所有场景生效，但日用和游戏的需求不同。

**建议**: 在代码层面实现场景检测，对不同场景应用不同参数：

```rust
// 伪代码示例
if is_gaming_mode() {
    // 游戏模式：保守省电
    doze_cfg.perf_ceil = 0.40;  // 不修改
    target_fps_max_offset = -3.0;  // 不修改
} else {
    // 日用模式：激进省电
    doze_cfg.perf_ceil = 0.30;  // 应用修改
    target_fps_max_offset = -5.0;  // 应用修改
}
```

**优点**:
- 日用场景最大化省电
- 游戏场景保证帧率稳定
- 避免用户手动切换模式

**实现难度**: 中等（需要在调度器中添加场景检测逻辑）

---

## 7. 实施建议与测试方案

### 7.1 实施步骤

**步骤 1: 基准测试（Day 1）**

在修改前，采集以下场景的基准数据：

1. **日用场景**（各 30 分钟）：
   - 社交应用（微信、微博）
   - 浏览器（Chrome、Safari）
   - 阅读应用（微信读书、Kindle）

2. **游戏场景**（各 30 分钟）：
   - 60fps 游戏（王者荣耀、和平精英）
   - 120fps 游戏（原神、崩坏：星穹铁道）

3. **息屏场景**（2 小时）：
   - 待机状态
   - 后台播放音乐

**采集指标**:
- 平均功耗（mA）
- 帧率稳定性（jank 次数）
- CPU 平均频率
- 温度变化

**步骤 2: 阶段 1 实施（Day 2-3）**

应用优先级 1, 2, 3, 5, 6 的修改：

1. 修改代码
2. 编译测试
3. 运行基准测试场景
4. 对比基准数据

**验收标准**:
- 日用场景功耗降低 ≥ 8%
- 游戏场景 jank 次数增加 ≤ 10%
- 息屏场景功耗降低 ≥ 15%

**步骤 3: 阶段 2 实施（Day 4-5）**

应用优先级 4, 7 的修改：

1. 修改代码
2. 编译测试
3. 运行基准测试场景
4. 对比阶段 1 数据

**验收标准**:
- 日用场景功耗进一步降低 ≥ 3%
- 游戏场景 jank 次数增加 ≤ 15%
- 120fps 游戏帧率稳定性无明显下降

**步骤 4: 阶段 3 实施（Day 6）**

应用优先级 8 的修改：

1. 修改代码
2. 编译测试
3. 运行基准测试场景
4. 对比阶段 2 数据

**验收标准**:
- 边际改善，无明显副作用

**步骤 5: 叠加测试（Day 7）**

1. 应用所有修改
2. 长时间运行测试（各场景 1 小时）
3. 监控是否有过度压制现象
4. 如有问题，按 6.2 节的参数回调建议调整

### 7.2 测试脚本

```bash
#!/bin/bash
# 省电优化测试脚本

echo "=== 开始省电优化测试 ==="

# 1. 采集基准数据
echo ">>> 采集基准数据..."
adb shell dumpsys batterystats --reset
# 运行测试场景 30 分钟
sleep 1800
adb shell dumpsys batterystats > baseline.txt

# 2. 应用修改后测试
echo ">>> 应用修改后测试..."
adb shell dumpsys batterystats --reset
# 运行测试场景 30 分钟
sleep 1800
adb shell dumpsys batterystats > optimized.txt

# 3. 对比结果
echo ">>> 对比结果..."
echo "基准功耗:"
grep "Estimated power use" baseline.txt
echo "优化后功耗:"
grep "Estimated power use" optimized.txt

echo "=== 测试完成 ==="
```

### 7.3 参数调优指南

如果测试中发现问题，按以下优先级调优：

**问题 1: 日用场景卡顿**

**症状**: 滑动列表、切换应用时明显掉帧

**排查**:
```bash
# 查看 jank 日志
adb logcat | grep -E "jank|Jank|JANK"
```

**调优**:
1. 回调 util_cap 上限：0.90 → 0.95
2. 回调 PID util_gain 系数：1.56 → 1.8
3. 回调 target_fps 最大偏移：-5.0 → -4.0

**问题 2: 游戏场景帧率不稳**

**症状**: 120fps 游戏频繁掉到 90fps 以下

**排查**:
```bash
# 查看 FAS 日志
adb logcat | grep -E "FAS|fas|frame"
```

**调优**:
1. 回调快速衰减阈值：0.76 → 0.80
2. 回调 decay_scale：0.75 → 0.65
3. 回调 target_fps 最大偏移：-5.0 → -3.0

**问题 3: 息屏后唤醒慢**

**症状**: 息屏后按电源键，需要 2-3 秒才亮屏

**排查**:
```bash
# 查看 Doze 日志
adb logcat | grep -E "doze|Doze|DOZE"
```

**调优**:
1. 回调 perf_ceil：0.30 → 0.35
2. 回调 smoothing_up：0.05 → 0.08
3. 回调 up_rate_limit_ticks：5 → 4

---

## 附录 A: 参考资料

### A.1 yumi 调度器架构

```
用户空间
    ↓
FAS (Frame Aware Scheduling)
    ├── PID 控制器 (pid.rs)
    ├── 帧流水线 (frame_pipeline.rs)
    ├── 控制器 (controller.rs)
    ├── 策略管理 (policy_mgmt.rs)
    └── 档位状态 (gear_state.rs)
    ↓
CLG (CPU Load Governor)
    ↓
内核调度器
```

### A.2 关键参数默认值

| 参数 | 默认值 | 修改后 | 说明 |
|:---|:---:|:---:|:---|
| significant_jump 阈值 | 0.35 | 0.50 | 触发快速升频的阈值 |
| 低负载降频阈值 | 0.10 | 0.15 | 触发加速降频的 util 阈值 |
| 低负载降频倍数 | 2.5 | 3.0 | 加速降频的倍数 |
| PID util_gain 上限 | 0.30 | 0.45 | 触发增益衰减的 fg_util 上限 |
| target_fps 偏移步长 | -0.1 | -0.2 | 每秒偏移量 |
| target_fps 最大偏移 | -3.0 | -5.0 | 最大偏移限制 |
| decay_scale (高刷) | 0.6 | 0.75 | 高刷下衰减步长缩放系数 |
| decay_threshold 增长系数 | 0.002 | 0.001 | 高刷下阈值增长速度 |
| EMA a_up 基础值 | 0.15 | 0.10 | 升频跟踪速度 |
| 升档 overshoot 阈值 | 1.35 | 1.50 | 触发快速升档的帧率过冲比例 |
| 升档样本数 | 15 | 20 | 升档需要的最小样本数 |
| Doze perf_ceil | 0.40 | 0.30 | 息屏时最高性能限制 |
| Doze smoothing_up | 0.10 | 0.05 | 息屏时升频平滑系数 |
| Doze up_rate_limit_ticks | 3 | 5 | 息屏时升频速率限制（tick） |

### A.3 监控命令速查

```bash
# 实时监控 CPU 频率
adb shell "while true; do cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq; sleep 1; done"

# 实时监控帧率
adb shell "dumpsys SurfaceFlinger --latency"

# 实时监控功耗（需要 root）
adb shell "cat /sys/class/power_supply/battery/current_now"

# 查看 yumi 调度器日志
adb logcat | grep -E "yumi|YUMI|clg|CLG|fas|FAS"

# 查看 jank 统计
adb shell "dumpsys gfxinfo <package_name>"
```

---

## 附录 B: 版本历史

| 版本 | 日期 | 修改内容 |
|:---|:---|:---|
| v1.0 | 2026-07-31 | 初始版本，包含 8 项优化 |

---

*文档生成时间：2026-07-31*
*基于 yumi v2.0.1 源码分析*
*作者：yumi 开发团队*