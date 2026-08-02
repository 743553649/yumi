# yumi 项目 Claude 规则

## 语言规范

- **始终使用中文回复**：所有解释、注释、交流均使用中文
- **代码注释使用中文**：Rust 源码及 Java/XML 资源注释使用中文编写
- **技术术语保留英文**：如 PID、EMA、FAS、CLG、CPU、GPU、IPC 等缩写保持原样
- **变量名/函数名保持英文**：遵循 Rust 及 Java 命名规范

## 项目持久化记忆

### 项目基本信息
- **项目名称**: yumi
- **项目类型**: Android CPU 调度器（Rust 核心守护进程 + Android 14 控制端 App）
- **主要功能**: 帧感知调度（FAS）、CPU 负载调速器（CLG）、CPUSet 绑核、TouchBoost 触摸提频、Idle Dive 静止下潜、TCP IPC 通信、iOS 26 极简冰雪白毛玻璃控制端 App
- **源码位置**: `/storage/emulated/0/yumi/`
- **测试设备**: 骁龙 8 Elite 处理器 / Android 14 (API 34)

### 核心模块
1. **FAS (Frame Aware Scheduling)** - 帧感知调度
   - `src/scheduler/fas/pid.rs` - PID 控制器
   - `src/scheduler/fas/controller.rs` - 主控制器
   - `src/scheduler/fas/frame_pipeline.rs` - 帧流水线处理
   - `src/scheduler/fas/gear_state.rs` - 档位状态管理

2. **CLG (CPU Load Governor)** - CPU 负载调速器
   - `src/scheduler/cpu_load_governor.rs` - 主控制器

3. **Doze 模式** - 息屏省电
   - `src/scheduler/mod.rs` - 状态机与事件处理

4. **CPUSet Manager** - CPU 核心绑核与 Cgroup 策略管理
   - `src/cpuset_manager/mod.rs` - CPUSet 管理器

5. **Idle Dive** - CPU 静止下潜与 C-State 空闲深度管理
   - `src/idle_dive/mod.rs` - IdleDive 控制器

6. **TouchBoost** - 触摸响应提频与输入监听
   - `src/touch_boost/mod.rs` - TouchBoost 控制器与 epoll 监听器

7. **IPC Server 通信服务端** - 守护进程 TCP 本地通信接口
   - `src/ipc_server.rs` - TCP 127.0.0.1:14567 监听，响应 `ping`、`get_mode`、`set_mode` 与带有终止符的 `get_log` 运行日志流协议

8. **yumi Bridge Android 控制端 App** - iOS 26 冰雪白毛玻璃控制中心 App
   - `android-app/` - Target SDK 34 (Android 14) Edge-to-Edge 无界全屏沉浸控制端，使用高对比度冰粹浅色毛玻璃 UI，支持 5 级中文日志实时分级过滤与 Root `su -c` 物理日志通道

### 重要配置文件
- `src/scheduler/config.rs` - 调度器配置
- `src/monitor/config.rs` - 监控配置
- `module/config/config.yaml` - 运行时主配置（含 IPC 服务端端口与语言）
- `module/config/cpuset.yaml` - CPUSet 绑核配置
- `module/config/idle_dive.yaml` - 静止下潜配置
- `module/config/touch_boost.yaml` - 触摸提频配置
- `android-app/app/src/main/AndroidManifest.xml` - Android App 清单（uses-sdk targetSdkVersion 34）
- `android-app/app/src/main/java/com/yumi/bridge/MainActivity.java` - App 控制 Activity
- `android-app/app/src/main/res/values/{themes.xml, colors.xml}` - iOS 26 浅色毛玻璃视觉 Token 与全屏沉浸主题

### 关键参数说明
- **perf**: 性能指标，范围 [0, 1]，值越高频率越高
- **target_fps**: 目标帧率
- **util**: CPU 利用率
- **smoothing_up/down**: 升频/降频平滑系数
- **perf_floor/ceil**: 性能地板/天花板

## 代码风格与修改原则

### 🎯 核心修改原则（必须严格遵守）
1. **严格限定修改范围**：只修改与当前任务直接相关的代码或文件，绝不随手变动无关代码或重构无关逻辑。
2. **严禁“拆东墙补西墙”**：修复新问题或调整局部样式/逻辑时，必须确保原有组件（如守护进程状态卡片、模式切换卡片、日志终端面板）完好无损，严禁通过隐藏或剥离已有功能来解决问题。

### Rust & Java 编码规范
- Rust 使用 4 空格缩进，函数 snake_case，结构体 PascalCase，每行原则上不超过 100 字符
- Java / XML 保持标准 Android 工程代码风格与高可读性
- 复杂逻辑必须添加清晰中文注释

### 日志规范
- Rust 端使用 `log::info!`, `log::debug!`, `log::warn!`, `log::error!` 并配合 `t()` 国际化
- Android 端使用中文呈现 5 级日志（全部、调试、信息、警告、错误）

## 工作流程

### 修改代码前
1. 阅读相关文档（如 `docs/省电优化方案.md` / `docs/yumi_Bridge_毛玻璃UI设计规范书.md` / `docs/superpowers/specs/2026-08-02-light-glassmorphism-theme-design.md`）
2. 理解现有架构和参数含义
3. 确认修改精确影响范围

### 修改代码时
1. 保持代码风格一致
2. 添加必要的中文注释
3. **只修改任务相关代码**，防止破坏非相关模块

### 修改代码后
1. 更新 `docs/工作日志.md` 记录修改
2. 执行打包脚本验证编译结果
3. 确保功能与日志输出清晰可读

## 测试与验证

### 编译与打包测试
- **Rust 核心与单元测试**：
  ```bash
  cargo check
  cargo test
  ```
- **Android App 打包与签名**：
  ```bash
  bash /storage/emulated/0/yumi/android-app/build_apk.sh
  ```
  产物路径：`/storage/emulated/0/yumi/yumi-bridge.apk`

### 功能验证
- 日用场景：社交、阅读、浏览
- 游戏场景：60fps/120fps 游戏
- 息屏场景：待机、后台同步
- 控制端场景：模式切换、状态同步、5 级中文日志过滤与浅色毛玻璃全屏沉浸渲染

## 重要文件索引

| 文件 | 用途 |
|:---|:---|
| `docs/yumi_Bridge_毛玻璃UI设计规范书.md` | yumi Bridge Android App 真实毛玻璃 (Glassmorphism) UI 设计规范书 (v5.0.0) |
| `docs/superpowers/specs/2026-08-02-light-glassmorphism-theme-design.md` | iOS 26 极简冰雪白毛玻璃 (Light Glassmorphism) 重构规格设计书 |
| `docs/省电优化方案.md` | 省电优化方案文档 |
| `docs/TouchBoost实现方案.md` | TouchBoost、CPUSet、IdleDive 实现方案文档 |
| `docs/工作日志.md` | 修改工作日志（记录各阶段实施状态） |
| `docs/项目结构.md` | 项目结构地图 |
| `docs/编译指南.md` | 编译构建指南 |
| `docs/测试指南.md` | 测试与验证指南 |
| `docs/CLG重构记录.md` | CLG 调速器重构日志与参数规范 |
| `README.md` | 项目说明文档 |
| `android-app/build_apk.sh` | Android App 打包与签名构建脚本 |
| `src/ipc_server.rs` | TCP Loopback IPC 通信服务端 |
| `android-app/app/src/main/java/com/yumi/bridge/MainActivity.java` | App 主控 Activity 实现 |
| `src/scheduler/config.rs` | 调度器配置定义 |
| `src/scheduler/mod.rs` | 调度器状态机与事件处理 |
| `src/scheduler/cpu_load_governor.rs` | CLG 负载调速器实现 |
| `src/cpuset_manager/mod.rs` | CPUSet 核心绑定管理实现 |
| `src/idle_dive/mod.rs` | Idle Dive 静止下潜实现 |
| `src/touch_boost/mod.rs` | TouchBoost 触摸提频实现 |
| `src/scheduler/fas/pid.rs` | FAS PID 控制器 |
| `src/scheduler/fas/controller.rs` | FAS 主控制器 |
| `src/scheduler/fas/frame_pipeline.rs` | FAS 帧流水线处理 |

## 注意事项

### 修改风险与原则
- **只改相关代码**：变动必须精准聚焦于任务目标
- **保持现有功能完整**：严禁“拆东墙补西墙”
- **回滚与回调预案**：记录修改原始值，准备参数回调预案

---

*最后更新: 2026-08-02*
*项目版本: yumi v3.1.0*
