# yumi 源码问题修复清单

> AI 助手按以下任务逐条执行，每条执行完毕打勾。

---

## 任务 1：缓存 `get_module_root()` 结果

**文件：** `src/common.rs`

当前 `get_module_root()` 每次调用都做 6+ 次 `Path::exists()` + exe 回溯，被 logger、i18n、app_detect、ipc_server 等多个模块高频调用。

**操作：**

1. 在文件顶部添加 `use std::sync::OnceLock;`。
2. 在 `get_module_root` 函数上方添加一个静态缓存变量：

```rust
static MODULE_ROOT: OnceLock<PathBuf> = OnceLock::new();
```

3. 将 `get_module_root` 改为：

```rust
pub fn get_module_root() -> PathBuf {
    MODULE_ROOT.get_or_init(|| detect_module_root()).clone()
}

fn detect_module_root() -> PathBuf {
    // 原来的全部探测逻辑移到这里，去掉 pub
    // ...（保持原有逻辑不变）
}
```

4. 执行 `cargo check` 确认编译通过。

---

## 任务 2：修复 `main.rs` 中重复的注释编号

**文件：** `src/main.rs`

当前文件中有两处 `// 3.` 和两处 `// 6.` 注释。

**操作：**

读取文件，找到所有带编号的注释行，重新编号为连续的 1-6：

```
// 1. 环境初始化
// 2. 提前读取配置
// 3. 加载语言
// 4. 初始化日志
// 5. 创建通信通道与共享配置
// 6. 启动 Scheduler
// 7. 启动 IPC Server
// 8. 启动 Monitor
// 9. 挂起等待
```

---

## 任务 3：将 `FastWriter` 的 buffer 从 20 字节扩大到 64 字节

**文件：** `src/utils.rs`

当前 `FastWriter.buf` 是 `[u8; 20]`，cpuset 掩码字符串在极端格式下可能接近上限。

**操作：**

1. 找到 `FastWriter` 结构体定义：

```rust
pub struct FastWriter {
    file: Option<File>,
    buf: [u8; 20],
    path: PathBuf,
}
```

2. 修改为：

```rust
pub struct FastWriter {
    file: Option<File>,
    buf: [u8; 64],
    path: PathBuf,
}
```

3. 找到 `new` 方法中 `buf: [0u8; 20]`，改为 `buf: [0u8; 64]`。
4. 找到 `write_value_force` 和 `write_value_force_str` 中 `local: [0u8; 20]`，改为 `local: [0u8; 64]`。
5. 执行 `cargo check`。

---

## 任务 4：将 `ipc_server.rs` 中的硬编码路径替换为动态路径

**文件：** `src/ipc_server.rs`

`set_mode`、`set_app_mode`、`get_log` 中有多处硬编码路径（`/storage/emulated/0/yumi/`、`/data/adb/modules/yumi/`），应统一从 `root` 参数派生。

**操作：**

1. 在 `process_command` 函数开头，或者在文件顶部，添加一个辅助函数：

```rust
/// 生成规则文件的写入候选路径列表
fn rules_write_targets(root: &PathBuf) -> Vec<PathBuf> {
    let mut paths = vec![root.join("rules.yaml")];
    // 如果 root 是 /data/adb/modules/yumi，也写入 /storage/emulated/0/yumi/
    // 如果 root 是 /storage/emulated/0/yumi，也写入 /data/adb/modules/yumi/
    let alt_bases = [
        PathBuf::from("/data/adb/modules/yumi"),
        PathBuf::from("/storage/emulated/0/yumi"),
        PathBuf::from("/storage/emulated/0/yumi/module"),
    ];
    for base in &alt_bases {
        let p = base.join("rules.yaml");
        if p != root.join("rules.yaml") && !paths.contains(&p) {
            paths.push(p);
        }
    }
    paths
}
```

2. 在 `set_mode` 和 `set_app_mode` 的分支中，将：

```rust
let _ = utils::try_write_file(&rules_path, &yaml_str);
let _ = utils::try_write_file("/storage/emulated/0/yumi/rules.yaml", &yaml_str);
let _ = utils::try_write_file("/storage/emulated/0/yumi/module/rules.yaml", &yaml_str);
let _ = utils::try_write_file("/data/adb/modules/yumi/rules.yaml", &yaml_str);
```

替换为：

```rust
for target in rules_write_targets(root) {
    let _ = utils::try_write_file(&target, &yaml_str);
}
```

3. 在 `get_log` 分支中，将 9 个硬编码候选路径替换为：

```rust
let candidate_logs = [
    root.join("logs/daemon.log"),
    root.join("module/logs/daemon.log"),
    PathBuf::from("/data/adb/modules/yumi/logs/daemon.log"),
    PathBuf::from("/storage/emulated/0/yumi/logs/daemon.log"),
    PathBuf::from("/storage/emulated/0/yumi/module/logs/daemon.log"),
];
```

减少到 5 个，且优先从 `root` 派生。

4. 执行 `cargo check`。

---

## 任务 5：将 `app_detect.rs` 中的硬编码黑名单移到配置

**文件：** `src/monitor/app_detect.rs` 和 `module/rules.yaml`

当前 `is_valid_user_app` 中硬编码了小米专属应用（`com.xiaomi.vtcamera`、`com.baidu.input_mi` 等）。

**操作：**

1. 读取 `src/monitor/config.rs`，确认 `RulesConfig` 中已有 `ignored_apps: Vec<String>` 字段。
2. 读取 `module/rules.yaml`，在文件末尾追加（如果还没有的话）：

```yaml
ignored_apps:
  - com.xiaomi.vtcamera
  - com.xiaomi.mibrain.speech
  - com.google.android.gms.ui
  - com.android.providers.media.module
  - com.android.permissioncontroller
```

3. 在 `is_valid_user_app` 中，将以下硬编码项从 match 中移除（保留核心系统进程）：

保留这些（通用系统进程）：
```rust
"com.android.systemui" => false,
"system_server" => false,
"surfaceflinger" => false,
"android.hardware.graphics.composer" => false,
"com.android.phone" => false,
"yumi" => false,
```

移除这些（设备专属，应走 ignored_apps）：
```rust
"com.xiaomi.vtcamera" => false,       // 移除
"com.xiaomi.mibrain.speech" => false,  // 移除
"com.google.android.gms.ui" => false,  // 移除
"com.android.providers.media.module" => false, // 移除
"com.android.permissioncontroller" => false,   // 移除
```

4. 确认被移除的项已添加到 `module/rules.yaml` 的 `ignored_apps` 列表中。
5. 执行 `cargo check`。

---

## 任务 6：提取 `FrameTimestampEvent` 到共享定义

**问题：** `EbpfFrameEvent`（eBPF 侧）和 `FrameTimestampEvent`（用户态 `fps_monitor.rs`）各自定义了一次内存布局，靠人工保持一致。

**操作：**

1. 读取 `yumi-ebpf/src/main.rs` 中的 `FrameTimestampEvent` 定义：

```rust
#[repr(C)]
pub struct FrameTimestampEvent {
    pub pid: u32,
    pub ktime_ns: u64,
}
```

2. 读取 `src/monitor/fps_monitor.rs` 中的 `FrameTimestampEvent` 定义：

```rust
#[repr(C)]
struct FrameTimestampEvent {
    pid: u32,
    ktime_ns: u64,
}
```

3. 确认两个定义的字段顺序和类型完全一致。如果不一致，以 eBPF 侧为准，修改用户态的定义。

4. 由于 eBPF crate 是 `#![no_std]`，无法直接共享结构体，但可以在用户态添加注释标注来源。在 `src/monitor/fps_monitor.rs` 的定义上方添加注释：

```rust
/// 帧时间戳事件（必须与 yumi-ebpf/src/main.rs 中的 FrameTimestampEvent 保持二进制一致）
/// 字段顺序: pid(u32) + padding(4) + ktime_ns(u64) = 16 bytes
#[repr(C)]
struct FrameTimestampEvent {
    pid: u32,
    ktime_ns: u64,
}
```

5. 在 `src/ebpf_monitor/mod.rs` 的测试中，添加一个同步校验测试：

```rust
#[test]
fn test_frame_event_layout_matches_ebpf() {
    // 确保用户态结构体与 eBPF 侧二进制兼容
    // FrameTimestampEvent: pid(u32) + ktime_ns(u64) = 16 bytes, align=8
    use std::mem;
    // 只验证大小，字段名可以不同
    assert_eq!(mem::size_of::<crate::monitor::fps_monitor::FrameTimestampEvent>(), 16);
    assert_eq!(mem::align_of::<crate::monitor::fps_monitor::FrameTimestampEvent>(), 8);
}
```

注意：这要求 `fps_monitor.rs` 中的 `FrameTimestampEvent` 是 `pub(crate)` 可见性。将其从 `struct` 改为 `pub(crate) struct`。

6. 执行 `cargo test --lib` 确认测试通过。

---

## 任务 7：`fas_types.rs` 重构默认值管理

**文件：** `src/fas_types.rs`

当前有 40+ 个 `d_xxx()` 函数，命名极度缩写，且 `FasRulesConfig` 的 `Default` impl 手动列出所有字段与 `#[serde(default)]` 重复。

**操作（分步，每步后 cargo check）：**

**Step 1：** 将所有 `d_xxx` 函数改为可读名称。全部替换：

| 原名 | 新名 |
|:---|:---|
| `d_perf_floor` | `default_perf_floor` |
| `d_perf_ceil` | `default_perf_ceil` |
| `d_perf_init` | `default_perf_init` |
| `d_perf_cold` | `default_perf_cold_boot` |
| `d_hysteresis` | `default_freq_hysteresis` |
| `d_heavy_ms` | `default_heavy_frame_threshold_ms` |
| `d_load_ms` | `default_loading_cumulative_ms` |
| `d_load_tol` | `default_loading_normal_tolerance` |
| `d_load_pf` | `default_loading_perf_floor` |
| `d_load_pc` | `default_loading_perf_ceiling` |
| `d_post_ign` | `default_post_loading_ignore_frames` |
| `d_post_perf` | `default_post_loading_perf` |
| `d_post_guard` | `default_post_loading_downgrade_guard` |
| `d_up_confirm` | `default_upgrade_confirm_frames` |
| `d_dn_confirm` | `default_downgrade_confirm_frames` |
| `d_up_cd` | `default_upgrade_cooldown` |
| `d_dampen` | `default_gear_dampen_frames` |
| `d_boost_inc` | `default_downgrade_boost_perf_inc` |
| `d_boost_dur` | `default_downgrade_boost_duration` |
| `d_fd_thresh` | `default_fast_decay_frame_threshold` |
| `d_fd_perf` | `default_fast_decay_perf_threshold` |
| `d_fd_max` | `default_fast_decay_max_step` |
| `d_fd_min` | `default_fast_decay_min_step` |
| `d_jank_cd` | `default_jank_cooldown_frames` |
| `d_max_inc_d` | `default_max_inc_damped` |
| `d_max_inc_n` | `default_max_inc_normal` |
| `d_damped_cap` | `default_damped_perf_cap` |
| `d_switch_ms` | `default_app_switch_gap_ms` |
| `d_switch_perf` | `default_app_switch_resume_perf` |
| `d_force_int` | `default_freq_force_reapply_interval` |
| `d_max_frame` | `default_fixed_max_frame_ms` |
| `d_cold_ms` | `default_cold_boot_ms` |
| `d_verify_interval` | `default_verify_freq_interval_secs` |
| `d_temp_thresh` | `default_core_temp_threshold` |
| `d_temp_perf` | `default_core_temp_throttle_perf` |
| `d_util_cap_divisor` | `default_util_cap_divisor` |
| `d_auto_cap` | `default_auto_capacity_weight` |

每个函数体不变，只改函数名。同时更新 `#[serde(default = "xxx")]` 中的字符串引用。

**Step 2：** `cargo check` 确认全部编译通过。

**Step 3（可选，更彻底）：** 如果想进一步简化，可以把 `Default` impl 中的字段赋值全部删掉，改为在 struct 上加 `#[serde(default)]`，让 serde 自动调用每个字段的 default 函数。但这需要确保每个字段都有 `#[serde(default = "xxx")]` 或类型本身实现了 `Default`。只在确认所有字段都有 serde default 之后才做这步。

---

## 任务 8：添加 IPC 连接数限制

**文件：** `src/ipc_server.rs`

当前每个 TCP 连接 spawn 一个线程，无上限。

**操作：**

1. 在文件顶部添加：

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_CONCURRENT_CONNECTIONS: usize = 8;
static ACTIVE_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);
```

2. 在 `start_with_listener` 的 `handle_client` spawn 处，改为：

```rust
Ok(stream) => {
    let current = ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
    if current >= MAX_CONCURRENT_CONNECTIONS {
        log::debug!("IPC connection rejected: max {} concurrent connections reached", MAX_CONCURRENT_CONNECTIONS);
        drop(stream);
        continue;
    }
    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    let tx = tx.clone();
    let root = root.clone();
    std::thread::spawn(move || {
        handle_client(stream, tx, root);
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    });
}
```

3. 执行 `cargo check`。

---

## 任务 9：`app_detect.rs` 减少不必要的 String clone

**文件：** `src/monitor/app_detect.rs`

**操作：**

读取 `app_detection_loop` 函数，找到防抖逻辑部分。将：

```rust
let mut final_pkg = last_package.clone();
let mut final_pid = get_current_pid();

if detected_pkg != last_package && !detected_pkg.is_empty() {
    if detected_pkg != pending_package {
        pending_package = detected_pkg.clone();
        pending_pid = detected_pid;
        debounce_start = Instant::now();
    } else if debounce_start.elapsed() >= Duration::from_millis(500) {
        final_pkg = pending_package.clone();
        final_pid = pending_pid;
        pending_package.clear();
    }
} else {
    pending_package.clear();
}
```

改为使用 `Option<String>` 减少 clone：

```rust
let mut final_pkg: Option<String> = None;
let mut final_pid = get_current_pid();

if detected_pkg != last_package && !detected_pkg.is_empty() {
    if detected_pkg != pending_package {
        pending_package = detected_pkg;
        pending_pid = detected_pid;
        debounce_start = Instant::now();
    } else if debounce_start.elapsed() >= Duration::from_millis(500) {
        final_pkg = Some(std::mem::take(&mut pending_package));
        final_pid = pending_pid;
    }
} else {
    pending_package.clear();
}

let final_pkg = final_pkg.unwrap_or_else(|| last_package.clone());
```

注意 `detected_pkg` 的来源也需要适配（它目前已经是 `String`）。确认改完后 `cargo check` 通过。

---

## 任务 10：eBPF 程序中添加 `PERCPU_ARRAY` 的注释说明 max_entries=1 的原因

**文件：** `yumi-ebpf/src/main.rs`

当前 `PerCpuArray::with_max_entries(1, 0)` 的 `1` 看起来像是 bug（只有一个 entry），但实际上 PerCpuArray 的每个 entry 是 per-CPU 的，所以 1 个 key 0 就够了。这个设计对不熟悉 Aya 的读者不直观。

**操作：**

在第一个 `PerCpuArray` 声明上方添加注释：

```rust
// PerCpuArray(key=0): Aya 的 PerCpuArray 在 BPF 侧使用 __uint(max_entries, 1)，
// 每个 entry 实际是一个 per-CPU 数组（num_possible_cpus 个副本）。
// key=0 表示"全局唯一的这个计数器"，get_ptr_mut(0) 自动定位到当前 CPU 的副本。
// 这是 Aya/Ebpf per-CPU map 的标准用法，不是只有一个 CPU 能写。
```

---

## 执行顺序建议

```
1.  任务 1 (缓存 get_module_root)         ← 基础设施，其他模块受益
2.  任务 2 (修复注释编号)                   ← 2 分钟
3.  任务 3 (FastWriter buffer 扩大)        ← 5 分钟
4.  任务 4 (ipc_server 硬编码路径)          ← 15 分钟
5.  任务 5 (app_detect 黑名单移到配置)       ← 10 分钟
6.  任务 6 (FrameTimestampEvent 共享)       ← 10 分钟
7.  任务 7 (fas_types 默认值重命名)          ← 20 分钟，纯重命名，无逻辑变更
8.  任务 8 (IPC 连接数限制)                  ← 5 分钟
9.  任务 9 (减少 clone)                     ← 10 分钟
10. 任务 10 (eBPF 注释)                     ← 2 分钟
```

每步完成后执行 `cargo check` 确认编译通过。任务 7 是纯重命名不改逻辑，放在后面做比较安全。
