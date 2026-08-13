# Task 4 Report: 清理冗余代码

## 检查结果

### 1. `src/idle_dive/mod.rs` — 无问题
- 仅包含模块声明和 re-export，代码干净。

### 2. `src/idle_dive/config.rs` — 无问题
- `use serde::Deserialize` 被使用。
- 所有字段、默认函数、`normalize()` 均被使用。
- 无死代码或未使用变量。

### 3. `src/idle_dive/controller.rs` — 发现并删除 2 个死字段 ✅已修复
- `low_util_ticks: u32` 和 `high_util_ticks: u32` 在 `new()` 和 `disabled()` 中初始化为 0，但整个模块中**从未被读取或修改**。
- 已从结构体定义、`new()` 和 `disabled()` 中移除。
- 所有 import 均被使用：`Instant`、`Result`、`info`、`warn`、`t`、`t_with_args`、`IdleDiveConfig`、`LatencyWriter`。

### 4. `src/idle_dive/latency.rs` — 无问题
- 所有 import 均被使用。
- `pm_qos_fd`、`governor_paths`、`latency_paths` 字段均被使用。
- `Drop` 实现正确清理 `pm_qos_fd`。

### 5. `src/touch_boost/mod.rs` — 无问题
- 仅包含模块声明和 re-export。

### 6. `src/touch_boost/config.rs` — 无问题
- 所有字段和函数均被使用。

### 7. `src/touch_boost/controller.rs` — 无问题
- 所有 import 均被使用。
- 所有字段在方法中有读写。
- `find_nearest_freq` 是 `pub(crate)` 级别的静态方法，被 `update()` 调用。

### 8. `src/touch_boost/monitor.rs` — 无问题
- `File`、`OpenOptions`、`AsRawFd`、`RawFd` 均被使用。
- 常量 `BTN_TOUCH`、`ABS_MT_TRACKING_ID`、`EV_KEY`、`EV_ABS` 均被使用。
- `Drop` 实现正确清理 `epoll_fd`。

### 9. `src/scheduler/cpu_load_governor.rs` — 无问题
- `StillDiveConfig` 在 `init_policies` 和 `reload_config` 参数中使用。
- `info`、`debug`、`warn` 均被使用。
- `still_mode`、`still_low_ticks`、`still_exit_boost` 字段在 `on_load_update` 和 `release` 中有完整读写。

### 10. `src/scheduler/config.rs` — 无问题
- 所有配置结构体和默认函数均被使用。

### 11. `src/scheduler/mod.rs` — 无问题
- 所有 import 均被使用。
- `is_screen_on`、`fas_suspended_at`、`fas_suspended_package`、`last_temp_update` 等状态变量均有完整生命周期管理。

---

## 删除的冗余代码

| 文件 | 删除内容 | 原因 |
|------|----------|------|
| `src/idle_dive/controller.rs` | `low_util_ticks: u32` 字段声明 | 结构体中声明但从未读写 |
| `src/idle_dive/controller.rs` | `high_util_ticks: u32` 字段声明 | 结构体中声明但从未读写 |
| `src/idle_dive/controller.rs` | `new()` 中 `low_util_ticks: 0` | 对应字段已删除 |
| `src/idle_dive/controller.rs` | `new()` 中 `high_util_ticks: 0` | 对应字段已删除 |
| `src/idle_dive/controller.rs` | `disabled()` 中 `low_util_ticks: 0` | 对应字段已删除 |
| `src/idle_dive/controller.rs` | `disabled()` 中 `high_util_ticks: 0` | 对应字段已删除 |

---

## 代码差异

```diff
--- a/src/idle_dive/controller.rs
+++ b/src/idle_dive/controller.rs
@@ -33,8 +33,6 @@ pub struct IdleDiveController {
     latency_writer: LatencyWriter,
     dive_timer: Instant,
     exit_timer: Instant,
-    low_util_ticks: u32,
-    high_util_ticks: u32,
     disabled: bool,
 }
 
@@ -52,8 +50,6 @@ impl IdleDiveController {
             latency_writer,
             dive_timer: Instant::now(),
             exit_timer: Instant::now(),
-            low_util_ticks: 0,
-            high_util_ticks: 0,
             disabled: false,
         })
     }
@@ -65,8 +61,6 @@ impl IdleDiveController {
             latency_writer: LatencyWriter::disabled(),
             dive_timer: Instant::now(),
             exit_timer: Instant::now(),
-            low_util_ticks: 0,
-            high_util_ticks: 0,
             disabled: true,
         }
     }
```

---

## 验证结果

- `cargo check` 因 **Permission denied** 无法执行（Android/Termux 环境下编译器权限受限）。
- 已通过人工静态分析验证：删除的字段在项目任何文件中均无引用（grep 确认 `low_util_ticks` 和 `high_util_ticks` 仅出现在 `controller.rs` 中）。
- 修改仅涉及删除未使用字段及其初始化赋值，不改变任何运行时逻辑。

---

## 潜在问题（记录但未修改）

1. **`idle_dive/latency.rs`**: `set_governor()` 方法始终返回 `Ok(())`，写入失败时仅 log warning。这是设计选择（尽力写入所有 policy 路径），但调用方的 `if let Err(e)` 永远不会触发。
2. **`scheduler/mod.rs`**: `still_dive` 配置提取模式（`if enabled { Some(...) } else { None }`）在事件循环中重复 4 次（约 L250、L324、L400、L472）。可考虑提取为 `Config` 上的辅助方法，但属于重构范畴，本次未修改。
