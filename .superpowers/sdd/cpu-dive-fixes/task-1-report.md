# Task 1+3 报告：IdleDive 代码质量修复

## 1. dive_timer 逻辑分析

### 分析结论：逻辑正确，无需修改

`dive_timer` 的完整生命周期：

1. **初始化** (`new()` L54, `disabled()` L67)：`dive_timer = Instant::now()`
2. **Normal 状态，负载高** (`update()` L85)：`dive_timer = Instant::now()` — 持续重置，防止误触发下潜
3. **Normal → Diving** (`update()` L82)：调用 `transition_to(Diving)`，此时 `dive_timer` 不需要重置（已满足延迟条件）
4. **Diving → Normal** (`update()` L91)：调用 `transition_to(Normal)`，在 `transition_to()` L134 执行 `self.dive_timer = Instant::now()` — **正确重置**
5. **DozeDiving → Normal** (`exit_doze()` L109)：同上，`transition_to(Normal)` 重置 `dive_timer`
6. **on_touch_fast_exit** (L116)：同上

**关键路径验证**：从 Diving 退出回到 Normal 时，`transition_to(IdleDiveState::Normal)` 在 L134 重置了 `dive_timer`。这确保了退出下潜后不会立即重新进入下潜状态（需要重新满足 `dive_delay_ms` 延迟）。

**结论**：dive_timer 逻辑无 bug，无需修改。

---

## 2. PM-QoS 错误处理修改

### 问题描述

`transition_to()` 方法中有 6 处 PM-QoS 写操作（3 个状态 × 2 个操作：set_governor + set_latency），全部使用 `let _ =` 静默忽略错误。如果 sysfs 写入失败（权限问题、节点不存在等），用户和开发者完全无法感知。

### 修改内容

将所有 6 处 `let _ =` 替换为 `if let Err(e)` + `warn!()` 日志模式，并使用项目现有的 i18n 系统（`t_with_args` + `fluent_args!`）实现中英文警告日志。

新增 i18n key（zh.ftl / en.ftl）：
- `idle-dive-set-governor-failed`
- `idle-dive-set-latency-failed`

---

## 3. 代码差异

### controller.rs

```diff
 use log::{info, warn};
 
-use crate::i18n::t;
+use crate::i18n::{t, t_with_args};
 use crate::idle_dive::config::IdleDiveConfig;
 use crate::idle_dive::latency::LatencyWriter;
```

```diff
         match new_state {
             IdleDiveState::Normal => {
                 info!("{}", t("idle-dive-exit"));
-                let _ = self.latency_writer.set_governor(&self.config.governors.normal);
-                let _ = self.latency_writer.set_latency(self.config.params.normal_latency_us);
+                if let Err(e) = self.latency_writer.set_governor(&self.config.governors.normal) {
+                    warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => "normal", "error" => e.to_string())));
+                }
+                if let Err(e) = self.latency_writer.set_latency(self.config.params.normal_latency_us) {
+                    warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => "normal", "error" => e.to_string())));
+                }
                 self.dive_timer = Instant::now();
             }
             IdleDiveState::Diving => {
                 info!("{}", t("idle-dive-enter"));
-                let _ = self.latency_writer.set_governor(&self.config.governors.diving);
-                let _ = self.latency_writer.set_latency(self.config.params.diving_latency_us);
+                if let Err(e) = self.latency_writer.set_governor(&self.config.governors.diving) {
+                    warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => "diving", "error" => e.to_string())));
+                }
+                if let Err(e) = self.latency_writer.set_latency(self.config.params.diving_latency_us) {
+                    warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => "diving", "error" => e.to_string())));
+                }
                 self.exit_timer = Instant::now();
             }
             IdleDiveState::DozeDiving => {
                 info!("{}", t("idle-dive-enter-dozed"));
-                let _ = self.latency_writer.set_governor(&self.config.governors.doze);
-                let _ = self.latency_writer.set_latency(self.config.params.doze_latency_us);
+                if let Err(e) = self.latency_writer.set_governor(&self.config.governors.doze) {
+                    warn!("{}", t_with_args("idle-dive-set-governor-failed", &fluent_args!("state" => "doze", "error" => e.to_string())));
+                }
+                if let Err(e) = self.latency_writer.set_latency(self.config.params.doze_latency_us) {
+                    warn!("{}", t_with_args("idle-dive-set-latency-failed", &fluent_args!("state" => "doze", "error" => e.to_string())));
+                }
             }
         }
```

### zh.ftl (新增)

```diff
 idle-dive-config-reloaded = [IdleDive] 配置已热重载
+idle-dive-set-governor-failed = [IdleDive] 设置 { $state } 调速器失败: { $error }
+idle-dive-set-latency-failed = [IdleDive] 设置 { $state } 延迟失败: { $error }
```

### en.ftl (新增)

```diff
 idle-dive-config-reloaded = [IdleDive] Config hot-reloaded
+idle-dive-set-governor-failed = [IdleDive] Failed to set { $state } governor: { $error }
+idle-dive-set-latency-failed = [IdleDive] Failed to set { $state } latency: { $error }
```

---

## 4. 验证结果

| 项目 | 结果 |
|------|------|
| `cargo check` | ❌ Permission denied (Android 环境无执行权限，非代码问题) |
| 代码审查 | ✅ 所有 `let _ =` 已替换为 `if let Err(e)` + `warn!` |
| i18n 完整性 | ✅ zh.ftl 和 en.ftl 均已添加对应 key |
| dive_timer 逻辑 | ✅ 确认正确，无需修改 |
| 代码风格 | ✅ 与项目现有模式一致（参考 `latency.rs`、`cpu_load_governor.rs`） |

---

## 修改文件清单

1. `src/idle_dive/controller.rs` — 替换 6 处 `let _ =` 为 i18n 包装的 `warn!` 日志
2. `module/config/i18n/zh.ftl` — 新增 2 个 i18n key
3. `module/config/i18n/en.ftl` — 新增 2 个 i18n key
