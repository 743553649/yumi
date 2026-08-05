# 纯正 iOS 风格 Race-to-Sleep 动态调度实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 摒弃传统“限制频率上限、拖慢升频”的被动省电策略，贯彻纯正 iOS **Race-to-Sleep (快完成、快下线) + 自适应陡峭降频** 调度体系。保持升频敏捷顺滑，一旦负载下行极速下线归位。

**Architecture:**
1. **TouchBoost 按需脉冲 (50ms Pulse Boost)**：按下瞬间给予 50ms 脉冲克服静态卡顿，滑动中解绑 `scaling_min_freq` 锁频，完全交由负载算完即降。
2. **CLG 陡峭降频与归位 (Race-to-Idle Ramp-Down)**：保持敏感升频，在负载下降段（`util < prev_util`）取消确认延迟（`down_wait = 0`），使用 3.0 倍速陡峭拉低频率。
3. **IdleDive 亮屏快速下潜 (Active Idle Dive)**：亮屏且手离开屏幕 300ms 负载低时，快速拉低 CPU 频率上限（`scaling_max_freq`）帮助芯片进入深度 C-state。
4. **CPUSet 线程级 QoS 隔离**：使用 Linux cgroup / cpuset 将前台 UI 绘制主线程独占绑定至大核，隔离后台及异步线程至小核。

## Global Constraints

- **拒绝限制升频/限制性能**：保持 `is_significant_jump` 升频阈值为敏感值，不拖慢升频响应。
- **构建测试**：必须通过 Android aarch64 交叉编译 (`cargo check --target aarch64-linux-android`)。
- **语言规范**：Rust 变量与代码规范，交付总结使用中文。

---

### Task 1: TouchBoost 50ms 按需脉冲化 (Pulse Boost)

**Files:**
- Modify: `src/touch_boost/mod.rs:182-233`
- Test: `src/touch_boost/mod.rs` (单元测试)

**Interfaces:**
- Consumes: `TouchBoostConfig`, `Instant`
- Produces: `TouchBoostController::on_touch_event(touching: bool)`

- [ ] **Step 1: 编写脉冲提频单元测试**

在 `src/touch_boost/mod.rs` 底部 `mod tests` 添加：

```rust
#[test]
fn test_pulse_boost_50ms_release() {
    let cfg = Arc::new(RwLock::new(TouchBoostConfig {
        enabled: true,
        boost_freqs: vec![2000000],
        release_delay_ms: 50,
        min_boost_duration_ms: 50,
        ..Default::default()
    }));
    let mut ctrl = TouchBoostController::new(cfg);
    ctrl.initialized = true;

    // 按下触控：应该处于 Touching
    ctrl.on_touch_event(true);
    assert_eq!(ctrl.state, BoostState::Touching);

    // 持续触摸 60ms：应自动超时释放锁频，进入 Recovering/Idle
    std::thread::sleep(std::time::Duration::from_millis(60));
    ctrl.on_touch_event(true);
    assert_ne!(ctrl.state, BoostState::Touching);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --lib touch_boost`
Expected: FAIL

- [ ] **Step 3: 修改 TouchBoost 状态机为 50ms 按需脉冲**

在 `src/touch_boost/mod.rs` 的 `on_touch_event` 中实现：

```rust
pub fn on_touch_event(&mut self, touching: bool) {
    if !self.initialized || !self.sync_enabled() {
        return;
    }
    let now = Instant::now();
    let cfg = self.config.read().unwrap();
    let min_dur = cfg.min_boost_duration_ms.min(50); // 脉冲最多持续 50ms

    match (self.state, touching) {
        // IDLE -> TOUCHING: 按下瞬间触发 50ms 脉冲
        (BoostState::Idle, true) => {
            drop(cfg);
            self.state = BoostState::Touching;
            self.touch_start = now;
            self.apply_boost();
        }
        // TOUCHING 状态持续触摸：超过 50ms 自动释放高频锁，让 CLG 依据负载接管
        (BoostState::Touching, true) => {
            drop(cfg);
            if now.duration_since(self.touch_start).as_millis() as u64 >= min_dur {
                self.state = BoostState::Recovering;
                self.release_time = now;
                self.recover_all();
            }
        }
        // 松手或恢复
        (BoostState::Touching, false) | (BoostState::Recovering, false) => {
            drop(cfg);
            self.state = BoostState::Idle;
            self.recover_all();
        }
        _ => { drop(cfg); }
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --lib touch_boost`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/touch_boost/mod.rs
git commit -m "refactor(touch_boost): convert continuous lock to 50ms pulse boost"
```

---

### Task 2: CLG 陡峭降频与 Race-to-Idle 极速下线

**Files:**
- Modify: `src/scheduler/config.rs`
- Modify: `src/scheduler/cpu_load_governor.rs:325-350`

**Interfaces:**
- Consumes: `CpuLoadGovernorConfig`, `util: f32`, `prev_util: f32`
- Produces: `CpuLoadGovernor::update_perf()`

- [ ] **Step 1: 在 `src/scheduler/config.rs` 保持敏捷升频，提升加速降频倍数**

```rust
// 保持敏捷升频不变 (不提高 up_jump_threshold)，仅扩大降频门槛与加速倍数
fn d_clg_dn_fast_t() -> f32 { 0.15 }         // 0.10 -> 0.15 (扩充极速降频范围)
fn d_clg_dn_fast_m() -> f32 { 3.0 }          // 2.5 -> 3.0 (降频速度提升至 3 倍)
```

- [ ] **Step 2: 在 `src/scheduler/cpu_load_governor.rs` 中实现 Race-to-Idle 指数级下线**

修改 `update_perf` 降频逻辑：

```rust
} else {
    // ── 降频路径 (Race-to-Idle 极速下线) ──
    cluster.up_wait = 0;
    cluster.down_wait += 1;

    let fast_down = util < self.cfg.down_fast_threshold;
    // 负载正在明显下行（例如渲染完成）：跳过降频等待计数直接陡峭回落
    let is_dropping = util < cluster.prev_util * 0.85;
    let can_down = fast_down || is_dropping || cluster.down_wait >= self.cfg.down_rate_limit_ticks;

    if can_down && target_perf < old_perf {
        let smooth = if fast_down || is_dropping {
            // 极速归位：使用 3.0 倍速陡峭降频
            self.cfg.smoothing_down * self.cfg.down_fast_mult
        } else if util < self.cfg.down_threshold {
            self.cfg.smoothing_down
        } else {
            self.cfg.smoothing_down * self.cfg.slow_down_scale
        };
        cluster.current_perf += (target_perf - old_perf) * smooth;
        if fast_down || is_dropping { cluster.down_wait = 0; }
    }
}
```

- [ ] **Step 3: 运行 Android 交叉编译检查**

Run: `cargo check --target aarch64-linux-android`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/scheduler/config.rs src/scheduler/cpu_load_governor.rs
git commit -m "feat(clg): implement Race-to-Idle steep ramp-down for quick CPU offload"
```

---

### Task 3: IdleDive 亮屏快速下潜 (300ms Active Idle)

**Files:**
- Modify: `src/idle_dive/mod.rs:40-70`

**Interfaces:**
- Consumes: `IdleDiveConfig`
- Produces: `IdleDiveController::update(avg_util: f32)`

- [ ] **Step 1: 修改 IdleDive 判定时间**

在 `src/idle_dive/mod.rs` 将默认判定延迟调小：

```rust
impl Default for IdleDiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dive_threshold: 0.15,
            exit_threshold: 0.25,
            dive_delay_ms: 300,   // 从 2000ms 缩短到 300ms (亮屏静止 300ms 立即下潜省电)
            exit_delay_ms: 50,    // 退出响应维持 50ms 毫秒级极速唤醒
            governors: IdleDiveGovernors::default(),
            params: IdleDiveParams::default(),
        }
    }
}
```

- [ ] **Step 2: 编译测试**

Run: `cargo check --target aarch64-linux-android`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/idle_dive/mod.rs
git commit -m "feat(idle_dive): shorten active idle dive latency to 300ms"
```

---

### Task 4: CPUSet 线程级 QoS 前台 UI 独占隔离

**Files:**
- Modify: `src/cpuset_manager/mod.rs`
- Modify: `src/scheduler/mod.rs`

**Interfaces:**
- Consumes: `pid: i32`
- Produces: `CpuSetManager::apply_ui_qos(pid: i32)`

- [ ] **Step 1: 在 `CpuSetManager` 中实现线程级 QoS 绑定**

```rust
impl CpuSetManager {
    /// 读取前台进程的线程并分类绑定 QoS
    pub fn apply_ui_qos(&self, pid: i32) {
        if pid <= 0 { return; }
        let task_dir = format!("/proc/{}/task", pid);
        if let Ok(entries) = std::fs::read_dir(task_dir) {
            for entry in entries.flatten() {
                let tid_str = entry.file_name();
                let comm_path = entry.path().join("comm");
                if let Ok(comm) = std::fs::read_to_string(comm_path) {
                    let comm_trimmed = comm.trim();
                    // UI 主线程 / 渲染线程绑定至高算力大核 (foreground/performance)
                    if comm_trimmed == "UI Thread" || comm_trimmed == "RenderThread" || comm_trimmed.starts_with("mali-") || comm_trimmed.starts_with("KGSL-") {
                        let _ = utils::try_write_file("/dev/cpuset/foreground/tasks", tid_str.to_str().unwrap().as_bytes());
                    } else if comm_trimmed.contains("async") || comm_trimmed.contains("log") || comm_trimmed.contains("Rx") {
                        // 非 UI/后台 Task 隔离压制至小核 (system-background/background)
                        let _ = utils::try_write_file("/dev/cpuset/system-background/tasks", tid_str.to_str().unwrap().as_bytes());
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: 在应用切换事件触发线程 QoS**

在 `src/scheduler/mod.rs` 的 `ModeChange` 处理流程中引入：

```rust
if mode != "fas" {
    cpuset_manager.apply_ui_qos(pid);
}
```

- [ ] **Step 3: 编译测试**

Run: `cargo check --target aarch64-linux-android`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/cpuset_manager/mod.rs src/scheduler/mod.rs
git commit -m "feat(cpuset): isolate foreground UI threads to big cores via QoS"
```

---

## Plan Self-Review

1. **Spec Coverage**: 剔除了所有拖慢升频、限制性能上限的防拉频修改；完全集中在“按需 50ms 脉冲”、“极速降频 Race-to-Idle”、“300ms 快速下潜”和“线程 QoS 隔离”。
2. **Performance Guarantee**: 保持升频敏感度，确保应用响应极速丝滑，仅在任务完成下行期进行陡峭降频省电。
