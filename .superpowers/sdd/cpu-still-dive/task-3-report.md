# Task 3 Report: IdleDive 实现

## 实现内容

从零实现了 IdleDive 模块，管理 CPU 空闲下潜状态机。模块通过 PM-QoS 和 sysfs 双通道写入 cpuidle governor 和延迟参数。

### 创建的文件

| 文件 | 说明 |
|------|------|
| `src/idle_dive/mod.rs` | 模块入口，导出 IdleDiveConfig 和 IdleDiveController |
| `src/idle_dive/config.rs` | IdleDiveConfig 结构体，含 Default 和 normalize() |
| `src/idle_dive/controller.rs` | 状态机实现 (Normal/Diving/DozeDiving) |
| `src/idle_dive/latency.rs` | PM-QoS + sysfs 双通道写入器 |
| `module/config/idle_dive.yaml` | 独立配置文件 |

### 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `src/main.rs` | 注册 `mod idle_dive` |
| `src/scheduler/config.rs` | Config 结构体添加 `idle_dive: IdleDiveConfig` 字段 |
| `src/scheduler/mod.rs` | 初始化 IdleDiveController，集成到 SystemLoadUpdate/ScreenStateChange/ConfigReload 事件 |
| `module/config/config.yaml` | 添加 IdleDive 配置段 |

### 状态机逻辑

- **Normal → Diving**: avg_util < dive_threshold 持续 dive_delay_ms
- **Diving → Normal**: avg_util > exit_threshold 持续 exit_delay_ms
- **任意 → DozeDiving**: 息屏 enter_doze()
- **DozeDiving → Normal**: 亮屏 exit_doze()
- **任意 → Normal**: on_touch_fast_exit() 触摸快速退出

### i18n 日志键

所有日志使用已有的 zh.ftl 键：
- `idle-dive-init` / `idle-dive-init-failed` / `idle-dive-unavailable`
- `idle-dive-enter` / `idle-dive-exit`
- `idle-dive-enter-dozed` / `idle-dive-exit-dozed`
- `idle-dive-config-reloaded`

## 测试结果

无法运行 `cargo build` — 当前设备未安装 Rust 工具链。代码已通过人工审查验证：
- 所有 import 正确且无冗余
- 状态机逻辑符合规范
- 生命周期管理正确 (LatencyWriter Drop 关闭 PM-QoS fd)
- 与 scheduler 事件循环集成点正确

## 自审发现

- 已清理所有未使用的 import
- 已简化 DozeDiving 分支中重复的日志代码
- 确保 disabled() 构造器创建完全无操作的实例
- LatencyWriter 字段保持 pub(crate) 可见性（通过 disabled() 方法暴露）
