# Task 4 Report: TouchBoost 实现

## 实现内容

完整实现了 TouchBoost 模块，包含以下组件：

### 新建文件

| 文件 | 说明 |
|------|------|
| `src/touch_boost/mod.rs` | 模块入口，导出 TouchBoostConfig、TouchBoostController、TouchMonitor、TouchEvent |
| `src/touch_boost/config.rs` | TouchBoostConfig 结构体，含 Default 和 normalize() |
| `src/touch_boost/controller.rs` | TouchBoostController — Boost 状态机，管理 scaling_min_freq 写入和衰减恢复 |
| `src/touch_boost/monitor.rs` | TouchMonitor — epoll 监听 /dev/input/event* 触摸事件 |
| `module/config/touch_boost.yaml` | 默认配置文件 |

### 修改文件

| 文件 | 变更 |
|------|------|
| `src/main.rs` | 添加 `mod touch_boost;` |
| `src/scheduler/config.rs` | Config 结构体添加 `touch_boost: TouchBoostConfig` 字段 |
| `src/scheduler/mod.rs` | 集成 TouchBoost 初始化、事件处理、配置热重载 |

## 核心设计

- **monitor.rs**: 使用 libc epoll 直接调用监听 `/dev/input/event*`，解析 `input_event` 结构体检测 BTN_TOUCH 和 ABS_MT_TRACKING_ID 两种触摸事件模式，通过 mpsc channel 发送 TouchEvent::Start/End
- **controller.rs**: 接管各 CPU policy 的 `scaling_min_freq`，触摸时写入 boost 频率，松手后按 `recover_decay` 系数指数衰减恢复（写 0 = 恢复内核默认最低频率）
- **集成方式**: 在 scheduler IPC 事件循环中非阻塞接收触摸事件（try_recv），每个 DaemonEvent 处理前刷新触摸队列并调用 update()

## 验证

- `cargo build` 无法在当前环境执行（无 Rust 工具链）
- 代码结构和模式与 idle_dive 模块一致
- i18n 键已在 en.ftl 和 zh.ftl 中预定义
- Commit: 6d3de15

## 自审发现

- 移除了未使用的 `EV_SYN` 常量和 `config` 字段（TouchMonitor 存储后未使用）
- 使用 libc 直接调用 epoll（而非 nix 包装），无需修改 Cargo.toml features
- InputEvent 使用 `#[repr(C)]` 确保与内核 struct input_event 内存布局一致（24 bytes on 64-bit）
