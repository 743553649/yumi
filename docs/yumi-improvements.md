# yumi 项目改进任务清单

> 本项目是基于 imacte/yumi 的个人复刻，针对自有机型/处理器定制。
> AI 助手按以下任务逐条执行，每条执行完毕后打勾标记。

---

## 任务 1：清理构建产物，减小仓库体积

当前仓库 71MB，android-app 的 Gradle 构建产物不应提交。

1. 检查 `android-app/.gitignore` 是否存在。如果不存在，创建它；如果存在，读取内容。
2. 确保 `.gitignore` 中包含以下条目（已有的跳过，缺少的追加）：

```
build/
.gradle/
local.properties
*.apk
*.aab
captures/
.externalNativeBuild/
.cxx/
```

3. 检查以下目录是否已被 Git 跟踪（`git ls-files android-app/build/ android-app/.gradle/`）。如果有输出，执行：

```bash
git rm -r --cached android-app/build/ android-app/.gradle/
git commit -m "chore: remove android build artifacts from tracking"
```

如果没有输出则跳过，不需要提交。

---

## 任务 2：收紧 Cargo.toml 的 tokio 依赖

当前 `Cargo.toml` 中 tokio 开了 `full` feature，包含大量本项目不需要的模块。

1. 读取 `Cargo.toml`，找到 `tokio` 那行依赖。
2. 将：

```toml
tokio = { version = "1", features = ["full"] }
```

替换为：

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-util", "sync", "time"] }
```

3. 执行 `cargo check`。如果报错提示缺少某个 feature，根据报错信息把缺失的 feature 加回去，然后再次 `cargo check` 直到通过。
4. 将最终能通过编译的 feature 列表写回 `Cargo.toml`。

---

## 任务 3：为 PID 控制器添加单元测试

文件路径：`src/scheduler/fas/pid.rs`

1. 读取该文件，确认 `PidController` 的公开接口（`new`、`compute`、`adapt_to_target_fps`）的参数和字段可见性。
2. 在文件末尾追加 `#[cfg(test)] mod tests { ... }` 模块，包含以下测试用例：
   - `test_no_error_no_ramp_up`：error=0 时 compute 输出应 <= 0（只有衰减，不应拉频）
   - `test_negative_error_ramps_up`：error<0（帧超时）时输出应 > 0（拉频）
   - `test_high_fps_adaptation`：调用 `adapt_to_target_fps(144.0)` 后 kp 应大于 base_kp
   - `test_integral_saturation`：连续 1000 次负 error 后，integral 绝对值不应超出 integral_limit
   - `test_fps_adapt_idempotent`：对同一 fps 值调用两次 adapt_to_target_fps，kp 不应改变
3. 注意字段可见性：如果 `kp`、`base_kp`、`integral`、`integral_limit` 等字段是 `pub(super)` 或 private，测试模块在同一文件内可以直接访问；如果是其他情况需要调整可见性或通过公共接口间接验证。
4. 执行 `cargo test --lib -- pid::tests`，确保全部通过。如果失败，修复测试或代码直到全部绿色。

---

## 任务 4：为 CPU 负载调速器添加单元测试

文件路径：`src/scheduler/cpu_load_governor.rs`

1. 读取该文件，理解 CLG 的核心计算逻辑（`compute` 或类似方法），找到输入输出的边界。
2. 在文件末尾追加 `#[cfg(test)] mod tests { ... }`，至少覆盖：
   - 低负载（util < down_threshold）时应输出降频信号
   - 高负载（util > up_threshold）时应输出升频信号
   - 中间负载（down_threshold < util < up_threshold）不应有大幅频率变化
   - 极低负载（util < down_fast_threshold）应触发快速降频路径
3. 执行 `cargo test --lib -- cpu_load_governor::tests`，确保全部通过。
