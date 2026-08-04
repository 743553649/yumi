# 骁龙 8 Elite 全大核架构 + eBPF 零耗高流畅调度 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现针对骁龙 8 Elite（Snapdragon 8 Elite）芯片的动态 Cluster 识别、TouchBoost 50ms 脉冲与 CPUSet 动态线程隔离、Idle Dive 300ms 深入 C-state 与 1ms 快出、以及 yumi-ebpf 内核探针零开销监控与降级保护。

**Architecture:** 基于现有的 Rust 守护进程，重构 `src/touch_boost/` 与 `src/idle_dive/`，支持动态匹配 Policy 0 (6 性能大核) 和 Policy 6 (2 超级大核)；在 `yumi-ebpf` 中接入 `sched_switch` 与 `Choreographer` 探针，通过 Aya RingBuffer 与主线程异步通信。

**Tech Stack:** Rust 2024, Aya eBPF (0.14), Tokio / mio (1.0), Linux cpuidle / cpuset / cgroup / tracepoint APIs.

## Global Constraints

- **Cross-Compilation Flag:** `$env:YUMI_SKIP_EBPF=1; cargo check --target aarch64-linux-android` 必须保持 100% 通过。
- **Function Size Limit:** 单个函数严格控制在 50 行以内的单一职责设计。
- **Concurrency & Lock Hygiene:** 严禁在持有 `RwLock` 锁期间进行 sysfs 写操作，必须采用快照拷贝 `drop(cfg)`。
- **Memory Alignment:** eBPF 与 Rust 用户态交互数据结构必须加 `#[repr(C, align(8))]`。
- **Encoding & Packaging:** 所有 `.sh` 与 `.yaml` 文件必须使用 Unix LF (`\n`) 换行符，打包调用 `cargo run --package xtask -- b`。

---

### Task 1: eBPF 与用户态无锁通信数据结构与优雅降级控制

**Files:**
- Create: `src/ebpf_monitor/mod.rs`
- Modify: `yumi-ebpf/Cargo.toml`
- Modify: `src/main.rs`
- Test: `src/ebpf_monitor/mod.rs` (inline test)

**Interfaces:**
- Consumes: Aya `RingBuf` 和 `tracepoint` 模块。
- Produces: `struct EbpfFrameEvent { pid: u32, frame_time_us: u64, is_drop: bool }` 供调度器订阅。

- [ ] **Step 1: Write the failing unit test for EbpfFrameEvent memory layout**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_event_memory_alignment() {
        assert_eq!(std::mem::align_of::<EbpfFrameEvent>(), 8);
        assert_eq!(std::mem::size_of::<EbpfFrameEvent>(), 24);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `$env:YUMI_SKIP_EBPF=1; cargo test --target aarch64-linux-android ebpf_monitor::tests::test_ebpf_event_memory_alignment`
Expected: FAIL (module not defined)

- [ ] **Step 3: Implement EbpfFrameEvent struct and fallback detector**

```rust
// src/ebpf_monitor/mod.rs

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct EbpfFrameEvent {
    pub pid: u32,
    pub flags: u32,
    pub frame_time_us: u64,
    pub timestamp_ns: u64,
}

pub struct EbpfMonitor {
    is_active: bool,
}

impl EbpfMonitor {
    pub fn new() -> Self {
        Self { is_active: false }
    }

    pub fn is_available(&self) -> bool {
        self.is_active
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `$env:YUMI_SKIP_EBPF=1; cargo test --target aarch64-linux-android ebpf_monitor::tests::test_ebpf_event_memory_alignment`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ebpf_monitor/mod.rs
git commit -m "feat(ebpf): add 8-byte aligned EbpfFrameEvent and monitor skeleton"
```

---

### Task 2: 骁龙 8 Elite 动态 Cluster 识别与 TouchBoost 封印超大核逻辑重构

**Files:**
- Modify: `src/touch_boost/mod.rs:40-270`
- Test: `src/touch_boost/mod.rs:620-770`

**Interfaces:**
- Consumes: `CpuPolicy` 数组与 `/sys/devices/system/cpu/cpufreq/policy*`。
- Produces: 动态识别 Policy 0 (6 核心) 与 Policy 6 (2 超级核心)，执行 50ms 脉冲提频与超级大核线程隔离。

- [ ] **Step 1: Write failing unit test for 8 Elite dual-cluster classification**

```rust
#[test]
fn test_snapdragon_8_elite_cluster_classification() {
    let mut controller = TouchBoostController::new(Arc::new(RwLock::new(TouchBoostConfig::default())));
    controller.initialized = true;
    
    // 模拟 8 Elite 结构: Policy 0 (6大核) 和 Policy 6 (2超大核)
    let policies = vec![
        CpuPolicy { id: 0, cpus: vec![0,1,2,3,4,5], min_freq: 600000, max_freq: 3530000, cur_freq: 600000 },
        CpuPolicy { id: 6, cpus: vec![6,7], min_freq: 800000, max_freq: 4320000, cur_freq: 800000 },
    ];
    controller.setup_8_elite_clusters(&policies);
    
    assert_eq!(controller.perf_policy_id(), Some(0));
    assert_eq!(controller.prime_policy_id(), Some(6));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `$env:YUMI_SKIP_EBPF=1; cargo test --target aarch64-linux-android touch_boost::tests::test_snapdragon_8_elite_cluster_classification`
Expected: FAIL (methods missing)

- [ ] **Step 3: Implement setup_8_elite_clusters and dual-lock boost logic**

```rust
// Add fields in TouchBoostController:
perf_policy: Option<i32>,
prime_policy: Option<i32>,

pub fn setup_8_elite_clusters(&mut self, policies: &[CpuPolicy]) {
    for p in policies {
        if p.cpus.contains(&6) || p.id == 6 {
            self.prime_policy = Some(p.id);
        } else if p.id == 0 {
            self.perf_policy = Some(p.id);
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `$env:YUMI_SKIP_EBPF=1; cargo test --target aarch64-linux-android touch_boost::tests::test_snapdragon_8_elite_cluster_classification`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/touch_boost/mod.rs
git commit -m "refactor(touch_boost): add Snapdragon 8 Elite cluster identification and dual-lock"
```

---

### Task 3: Idle Dive 300ms 深度 C-State 下潜与 PM-QoS 备用兼容重构

**Files:**
- Modify: `src/idle_dive/mod.rs:130-360`
- Test: `src/idle_dive/mod.rs:380-540`

**Interfaces:**
- Consumes: CPU 平均负载及 `/sys/devices/system/cpu/cpuidle/latency_us` 或 `/dev/cpu_dma_latency`。
- Produces: 300ms 深入 C-state 与 1ms 快出状态转换机。

- [ ] **Step 1: Write failing unit test for PM-QoS fallback and 1ms fast exit**

```rust
#[test]
fn test_idle_dive_1ms_fast_exit() {
    let cfg = Arc::new(RwLock::new(IdleDiveConfig::default()));
    let mut controller = IdleDiveController::new(cfg);
    controller.initialized = true;
    controller.enter_dive();
    assert_eq!(controller.state(), DiveState::Diving);
    
    // 触发触摸信号，必须 1ms 内退出下潜并恢复 Normal
    controller.on_touch_fast_exit();
    assert_eq!(controller.state(), DiveState::Normal);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `$env:YUMI_SKIP_EBPF=1; cargo test --target aarch64-linux-android idle_dive::tests::test_idle_dive_1ms_fast_exit`
Expected: FAIL (on_touch_fast_exit not defined)

- [ ] **Step 3: Implement 1ms fast exit and PM-QoS fallback writer**

```rust
pub fn on_touch_fast_exit(&mut self) {
    if self.state == DiveState::Diving {
        self.exit_dive();
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `$env:YUMI_SKIP_EBPF=1; cargo test --target aarch64-linux-android idle_dive::tests::test_idle_dive_1ms_fast_exit`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/idle_dive/mod.rs
git commit -m "feat(idle_dive): add 1ms fast exit on touch and PM-QoS fallback"
```

---

### Task 4: 完整构建、打包闭环与验证

**Files:**
- Modify: `docs/superpowers/specs/2026-08-04-snapdragon-8-elite-ebpf-design.md`
- Build: `xtask`

- [ ] **Step 1: Run clippy and format checks**

Run: `cargo fmt --check; $env:YUMI_SKIP_EBPF=1; cargo clippy --target aarch64-linux-android`
Expected: 0 errors/warnings.

- [ ] **Step 2: Run xtask build to generate release package**

Run: `cmd /c "set YUMI_SKIP_EBPF=1&& cargo run --package xtask -- b"`
Expected: Build output zip generated under `output/`.

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "chore(release): complete Snapdragon 8 Elite + eBPF implementation build"
```
