# 🗺️ yumi 项目地图 (Project Map)

> 自动生成于 2026-07-30，基于源码逐文件深度分析。

---

## 📁 项目根目录 `/`

| 文件/目录 | 类型 | 作用 |
|:---|:---:|:---|
| `Cargo.toml` | 📄 | **Rust 工作区与主包清单**。定义 workspace 成员 (`xtask`, `yumi-ebpf`)，声明主包 `yumi` 的元信息 (v2.0.1, GPL-3.0) 及全部依赖 (aya, tokio, serde, log4rs, fluent 等)。release profile 启用 strip + LTO + opt-level=z 极致压缩。 |
| `Cargo.lock` | 📄 | 依赖锁定文件，确保可复现构建。 |
| `build.rs` | 📄 | **构建脚本**。编译时自动安装 `bpf-linker`，然后交叉编译 `yumi-ebpf` crate 为 BPF 目标 (`bpfel-unknown-none`)，产物嵌入到主二进制中。 |
| `README.md` | 📄 | 中文项目文档。详述架构、FAS 原理、配置参数、安装与故障排除。 |
| `README.en.md` | 📄 | 英文版项目文档。 |
| `LICENSE` | 📄 | GNU GPL v3.0 开源许可证。 |
| `.gitignore` | 📄 | Git 忽略规则 (target/, .idea/, *.zip, output/ 等)。 |
| `docs/PROJECT_MAP.md` | 📄 | **本文件** — 项目结构地图。 |

---

## 📁 `.cargo/` — Cargo 别名配置

| 文件 | 作用 |
|:---|:---|
| `config.toml` | 定义 `cargo xtask` 别名，等价于 `cargo run --package xtask`。 |

---

## 📁 `.github/` — CI/CD

| 文件 | 作用 |
|:---|:---|
| `dependabot.yml` | Dependabot 自动依赖更新配置。 |
| `workflows/build.yml` | **GitHub Actions 构建流水线**。触发条件: push main / tag / PR / 手动。流程: checkout → Node.js 24 (编译 WebUI) → Rust nightly + aarch64-linux-android target → Android NDK r29 → `cargo xtask build` → 上传 zip 产物。 |

---

## 📁 `src/` — Rust 守护进程主源码

### 根模块文件

| 文件 | 作用 |
|:---|:---|
| `main.rs` | **程序入口**。初始化工作目录、加载配置、设置语言、初始化日志，创建 `mpsc` 事件通道，启动 Scheduler 线程和 Monitor 线程，然后阻塞等待。 |
| `common.rs` | **共享类型**。定义 `DaemonEvent` 枚举 (5 种事件: ModeChange / FrameUpdate / SystemLoadUpdate / ConfigReload / ScreenStateChange) 和 `get_module_root()` 工具函数。 |
| `fas_types.rs` | **FAS 配置数据结构**。定义 `FasRulesConfig` (50+ 字段的完整 FAS 参数)、`PidCoefficients`、`ClusterProfile`、`PerAppProfile`，含全部默认值和序列化支持。 |
| `i18n.rs` | **国际化系统**。基于 Fluent (`.ftl` 文件) 的 i18n 引擎，全局 `RwLock<FluentBundle>`，提供 `t()` / `t_with_args()` 翻译函数和 `fluent_args!` 宏。 |
| `logger.rs` | **日志系统**。基于 log4rs 的滚动文件日志 (5MB × 3 份)，支持运行时动态更新日志等级。 |
| `utils.rs` | **通用工具库**。包含: `FastWriter` (带 unmount + 去重的 sysfs 高性能写入器)、`SysPathExist` (系统路径探测)、`read_config<T>` (YAML 通用反序列化)、`find_cpu_temp_path()` (温度传感器路径搜索)、`get_ktime_ns()` (BPF 对齐单调时钟)。 |

### 📁 `src/monitor/` — 监控线程组

| 文件 | 作用 |
|:---|:---|
| `mod.rs` | **Monitor 模块入口**。解除 eBPF 内存锁定限制，初始化共享配置和屏幕状态，启动 5 个子线程: screen_watcher、config_watcher、fps_monitor_ebpf、cpu_monitor_ebpf、app_detection_loop (阻塞主线程)。 |
| `app_detect.rs` | **前台应用检测**。通过 `top-app` cgroup 进程列表检测当前前台应用，无阻塞 500ms 防抖，自动过滤输入法和系统进程，读取 CPU 温度，发送 `ModeChange` 事件。同时包含 `watch_config_file()` (inotify 监听 rules.yaml 热重载)。 |
| `config.rs` | **Rules 配置结构**。定义 `RulesConfig` (yumi_scheduler / dynamic_enabled / global_mode / app_modes / ignored_apps / fas_rules)，re-export `FasRulesConfig`。 |
| `cpu_monitor.rs` | **eBPF CPU 负载监控**。加载 eBPF 程序挂载 `sched_switch` tracepoint，每 200ms 读取 PerCpuArray 计算每核心利用率 (带实时 pending delta 补偿)，通过 TGID 级聚合 map 计算前台进程 CPU 利用率，发送 `SystemLoadUpdate` 事件。降级路径: 逐 TID 遍历。 |
| `fps_monitor.rs` | **eBPF FPS 监控**。单 eBPF 实例 + 多 PID uprobe 架构。挂载 `libgui.so::Surface::queueBuffer` uprobe，通过 RingBuf 零拷贝读取帧时间戳，mio 轮询，PID 切换时自动 detach/attach，发送 `FrameUpdate` 事件。 |
| `screen_detect.rs` | **屏幕状态检测**。通过 Netlink uevent 监听 `power` (early_suspend/late_resume) 和 `backlight` 子系统事件，零轮询检测屏幕亮灭，更新共享状态。 |

### 📁 `src/scheduler/` — 调度器线程组

| 文件 | 作用 |
|:---|:---|
| `mod.rs` | **Scheduler 模块入口**。启动 Config Watcher 线程 (监听 config.yaml 热重载) 和 IPC 主线程 (事件循环)。IPC 线程管理 FasController 和 CpuLoadGovernor 的生命周期，处理 5 种 DaemonEvent，实现息屏 Doze 模式和 FAS 挂起/恢复 (5 秒宽限期)。含 `get_cpu_policies()` / `auto_compute_capacity_weights()` 工具函数。 |
| `config.rs` | **核心配置结构**。定义 `Config` (Meta / FunctionToggles / IOSettings / CpuIdle / 4 种 Mode)、`CpuLoadGovernorConfig` (CLG 的 10 个参数)，`from_file()` YAML 解析。 |
| `scheduler.rs` | **系统级调度器**。`CpuScheduler` 执行一次性系统调整: CPU Idle Governor 设置、I/O 优化 (遍历 /sys/block/* 设置调度器/预读/合并/iostats)。 |
| `cpu_load_governor.rs` | **CPU 负载调速器 (CLG)**。替代内核原生调速器，基于 eBPF 实时负载数据自适应调频。每 cluster 独立 `ClusterState`，EMA 平滑 + 升降频速率限制 + headroom 因子，通过 `FastWriter` 写入 scaling_min/max_freq。 |

### 📁 `src/scheduler/fas/` — FAS 帧感知调度引擎

| 文件 | 作用 |
|:---|:---|
| `mod.rs` | FAS 子模块声明，re-export `FasController`。 |
| `controller.rs` | **FAS 主控制器**。定义 `FasController` 结构体 (40+ 状态字段)，实现 `update_frame()` 主入口 (6 阶段管线)、游戏生命周期管理 (`set_game` / `clear_game`)、CPU 负载接口、动态 target_fps 偏移 (GPU bound 场景省电)、`effective_perf_floor()` (高刷动态抬高地板)。 |
| `fps_window.rs` | **帧率滑动窗口**。环形缓冲区 (120 帧)，O(1) 推入/均值/标准差计算，64 帧周期性重算抑制浮点累积误差，支持 `recent_mean(n)` 近 N 帧均值。 |
| `pid.rs` | **PID 控制器**。带动态系数缩放 (按 target_fps 线性/sqrt/0.3 次幂)、积分泄漏防饱和、动态低通微分滤波、利用率感知增益调制 (GPU bound 衰减 P 项)。`fps_norm()` / `scale_frames()` 工具函数。 |
| `gear_state.rs` | **帧率档位决策**。`GearDecision` 枚举 (Hold/Upgrade/Downgrade)，`evaluate_gear()` 实现升档 (过冲检测 + 连续确认 + 低 perf 稳帧升档) 和降档 (极端帧率原生档位检测 + boost 提频确认 + 指数退避冷却)。 |
| `frame_pipeline.rs` | **帧处理管线**。实现 `update_frame()` 的 6 个阶段: ①冷启动/应用切换 ②加载检测 ③齿轮决策 ④EMA 更新 ⑤PID+Jank ⑥快速衰减。含心跳日志 (每 30 帧)。 |
| `pid_jank.rs` | **PID + Jank 处理**。`update_pid_and_jank()` 实现 crit/heavy/normal 三级帧时间响应，jank streak 指数递增，紧急跳频 (>50ms 帧直接跳 0.70)，post-jank 恢复保护 (防断崖衰减)，perf_floor 死锁检测与救援。 |
| `policy_controller.rs` | **单 Cluster 频率控制器**。`PolicyController` 管理单个 CPU policy 的频率写入、迟滞防抖、频率校验 (1.5s 周期读回对比，不匹配则 unmount 重写)、`force_reapply()`。 |
| `policy_mgmt.rs` | **策略管理**。`load_policies()` 初始化所有 CPU policy (读取频率列表、合并 boost 频率、自动容量权重、创建 FastWriter)。`reload_rules()` 热重载 FAS 参数 (不影响状态机连续性)。`apply_freqs()` 利用率软封顶 + capacity_weight 频率分配 + 频率迟滞。 |

---

## 📁 `yumi-ebpf/` — eBPF 内核探针

| 文件 | 作用 |
|:---|:---|
| `Cargo.toml` | eBPF crate 清单。`no_std` 二进制，依赖 `aya-ebpf 0.2.1`，目标 `bpfel-unknown-none`。 |
| `.cargo/config.toml` | eBPF 专用 Cargo 配置 (build-std=core, bpf target)。 |
| `src/main.rs` | **eBPF 程序本体** (`#![no_std]`, `#![no_main]`)。包含两个探针: ① `handle_frame` (uprobe): 挂载 `Surface::queueBuffer`，通过 RingBuf 发送 `FrameTimestampEvent {pid, ktime_ns}`。② `handle_sched_switch` (tracepoint): 挂载 `sched/sched_switch`，累计每核心 idle/busy 时间、线程运行时间 (THREAD_RUN_TIME HashMap)、TGID 聚合运行时间 (TGID_RUN_TIME HashMap)，更新每核当前 TID/TGID。 |

---

## 📁 `xtask/` — 构建打包工具

| 文件 | 作用 |
|:---|:---|
| `Cargo.toml` | xtask 包清单。依赖 clap (CLI)、xshell (子进程)、zip (打包)、chrono (时间戳)。 |
| `src/main.rs` | **构建系统入口**。`cargo xtask build` 命令: 编译 WebUI (npm run build) → 编译 Rust Core (cargo ndk arm64-v8a release) → 拷贝 module 目录 → 组装 core/bin/yumi + webroot → 打包为 `yumi-{version}-{git_count}-{date}.zip`。 |
| `src/zip_ext.rs` | ZIP 打包扩展。递归遍历目录创建 ZIP 归档，支持自定义压缩选项。 |

---

## 📁 `webui/` — Vue.js WebUI 管理界面

### 根配置文件

| 文件 | 作用 |
|:---|:---|
| `package.json` | Node.js 项目清单。Vue 3 + Vant 4 + Pinia + vue-i18n + vue-router，Vite 构建。 |
| `package-lock.json` | npm 依赖锁定。 |
| `vite.config.ts` | Vite 配置。Vue 插件 + Vant 自动导入 + `base: './'` (相对路径，适配 WebUI 环境)。 |
| `tsconfig.json` / `tsconfig.app.json` / `tsconfig.node.json` | TypeScript 配置。 |
| `components.d.ts` | Vant 组件自动导入类型声明。 |
| `env.d.ts` | Vite 环境变量类型声明。 |
| `index.html` | HTML 入口，SPA 挂载点。 |
| `.gitignore` | 忽略 node_modules/ dist/ 等。 |
| `README.md` | WebUI 子项目说明。 |
| `.vscode/extensions.json` | VS Code 推荐扩展。 |

### 📁 `webui/public/` — 静态资源

| 文件 | 作用 |
|:---|:---|
| `favicon.ico` | 浏览器标签页图标。 |

### 📁 `webui/src/` — 源码

| 文件 | 作用 |
|:---|:---|
| `main.ts` | Vue 应用入口。创建 app，挂载 Pinia / Router / Vant / i18n。 |
| `App.vue` | 根组件。Vant ConfigProvider + router-view，全局样式重置。 |

### 📁 `webui/src/router/` — 路由

| 文件 | 作用 |
|:---|:---|
| `index.ts` | Vue Router 配置。Hash 模式，4 个路由: `/` (Home)、`/apps` (AppRules)、`/config` (ConfigEditor)、`/log` (LogViewer)。 |

### 📁 `webui/src/stores/` — Pinia 状态管理

| 文件 | 作用 |
|:---|:---|
| `scheduler.ts` | **调度器状态**。`currentMode` / `appRules` / `isDaemonRunning`，`initData()` 并行获取三个接口，`switchMode()` 切换模式。 |
| `counter.ts` | 示例计数器 store (未使用，脚手架残留)。 |

### 📁 `webui/src/views/` — 页面视图

| 文件 | 作用 |
|:---|:---|
| `HomeView.vue` | **主页**。显示守护进程运行状态、当前模式、4 种模式切换卡片 (省电/均衡/性能/极速)、QQ 群/TG/GitHub 链接、跳转到应用管理/配置编辑/日志查看。支持中英文切换。 |
| `AppRulesView.vue` | **应用规则管理**。列出已安装应用 (通过 KernelSU API)，支持搜索 (应用名+包名)，点击弹出 ActionSheet 选择模式 (含 FAS)，设置 per-app 性能策略。 |
| `ConfigEditorView.vue` | **配置编辑器**。双 Tab (Schedule Rules / Core Config)，递归渲染 YAML 配置为可折叠列表，支持内联编辑 (string/number/array)、布尔开关、模式选择器，实时保存。 |
| `LogViewerView.vue` | **日志查看器**。终端风格 UI (伪 Mac 窗口控制栏)，读取 `daemon.log`，正则高亮 (时间戳绿色、INFO 蓝色、WARN 黄色、ERROR 红色、模块标签紫色)，自动滚动到底部。 |

### 📁 `webui/src/utils/` — 工具层

| 文件 | 作用 |
|:---|:---|
| `bridge.ts` | **Bridge 抽象层**。生产环境通过 KernelSU `exec()` 读写文件 (cat/echo)、获取/保存 YAML 配置、切换模式、获取已安装应用列表、读取日志。开发环境自动切换到 MockBridge。 |
| `mock.ts` | **开发 Mock 数据**。模拟所有 Bridge API 响应，含 mock rules/config/apps/delay，用于浏览器独立开发调试。 |

### 📁 `webui/src/kernelsu/` — KernelSU 原生桥接

| 文件 | 作用 |
|:---|:---|
| `index.js` | **KernelSU JS Bridge 实现**。封装 `ksu.exec()` / `ksu.spawn()` / `ksu.toast()` / `ksu.listPackages()` / `ksu.getPackagesInfo()` 等原生 API，带回调管理。 |
| `index.d.ts` | TypeScript 类型声明。定义 `ExecResults` / `ChildProcess` / `PackagesInfo` 等接口。 |

### 📁 `webui/src/i18n/` — WebUI 国际化

| 文件 | 作用 |
|:---|:---|
| `index.ts` | vue-i18n 初始化。默认从 localStorage 读取语言，fallback 为英文。 |
| `locales/en.ts` | 英文翻译 (70+ 条目)。 |
| `locales/zh.ts` | 中文翻译 (70+ 条目)。 |

---

## 📁 `module/` — Magisk/KernelSU 模块

| 文件 | 作用 |
|:---|:---|
| `module.prop` | 模块元信息 (id=yumi, v2.0.1, updateJson 指向 GitHub)。 |
| `customize.sh` | **安装脚本**。自动检测系统语言 (中文/英文)，输出欢迎信息。不做文件操作。 |
| `service.sh` | **启动脚本**。等待 `sys.boot_completed` → 禁用 Oiface (OPPO) / Joyose (小米) → 清理旧进程 → 设置权限 → `nohup` 启动 yumi 守护进程。 |
| `uninstall.sh` | **卸载脚本**。恢复 Oiface / Joyose 服务，提示重启。 |
| `rules.yaml` | **调度规则配置**。定义全局模式、per-app 模式映射 (原神/王者/和平精英等 → FAS)、FAS 全部参数 (帧率档位/PID 系数/per-app 配置等)。 |
| `config/config.yaml` | **核心配置**。Meta (日志等级/语言)、功能开关、I/O 优化、CPU Idle、4 种性能模式的 CLG 参数。 |
| `config/i18n/en.ftl` | **Rust 守护进程英文日志翻译**。Fluent 格式，覆盖所有模块 (Monitor/Scheduler/FAS/CLG/SysFS)。 |
| `config/i18n/zh.ftl` | **Rust 守护进程中文日志翻译**。与 en.ftl 一一对应。 |
| `scripts/disable_boost.sh` | **Boost 禁用脚本**。禁用内核级 boost (touch_boost/sched_boost/cpu_boost 等)、系统级服务 (miuibooster/perfd)、cpuset/schedtune/uclamp boost、CoreCtl、温控降频。当前在 service.sh 中被注释。 |
| `META-INF/com/google/android/update-binary` | Magisk 模块安装标准入口。 |
| `META-INF/com/google/android/updater-script` | Magisk 模块更新脚本。 |

---

## 📁 `updateInformation/` — OTA 更新

| 文件 | 作用 |
|:---|:---|
| `update.json` | 模块更新信息 (版本号、下载链接、changelog URL)。KernelSU 通过此文件检查更新。 |
| `changelog.md` | 更新日志。 |

---

## 📁 `.backup/` — 备份数据

| 目录 | 作用 |
|:---|:---|
| `chats/` | 会话备份 (JSON 格式的对话状态)。 |
| `objects/` | 内容寻址对象存储 (类似 Git object)。 |

---

## 🏗️ 架构总览

```
┌─────────────────────────────────────────────────────┐
│                    yumi-ebpf (内核态)                  │
│  ┌──────────────────┐  ┌──────────────────────────┐ │
│  │ handle_frame      │  │ handle_sched_switch      │ │
│  │ (uprobe queueBuffer) │ │ (tracepoint sched_switch)│ │
│  │ → RingBuf          │  │ → PerCpuArray + HashMap  │ │
│  └──────────────────┘  └──────────────────────────┘ │
└─────────────────────────┬───────────────────────────┘
                          │ eBPF Maps
┌─────────────────────────▼───────────────────────────┐
│                   yumi 守护进程 (用户态)                │
│                                                      │
│  ┌─── Monitor 线程组 ───┐  ┌─── Scheduler 线程组 ──┐ │
│  │ app_detect (Cgroup)   │  │ IPC 事件循环          │ │
│  │ fps_monitor (eBPF)    │→│ FasController (游戏)  │ │
│  │ cpu_monitor (eBPF)    │  │ CpuLoadGovernor (日常)│ │
│  │ screen_detect (Netlink)│  │ CpuScheduler (系统)  │ │
│  │ config_watcher (inotify)│  │ ConfigWatcher       │ │
│  └───────────────────────┘  └──────────────────────┘ │
└──────────────────────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────┐
│                   WebUI (Vue 3 + Vant)                │
│  HomeView / AppRulesView / ConfigEditor / LogViewer  │
│  ← Bridge (KernelSU exec) → rules.yaml / config.yaml│
└──────────────────────────────────────────────────────┘
```

---

## 📊 文件统计

| 类别 | 文件数 | 主要语言 |
|:---|:---:|:---|
| Rust 源码 (src/) | 18 | Rust |
| eBPF 探针 (yumi-ebpf/) | 1 | Rust (no_std) |
| 构建工具 (xtask/) | 2 | Rust |
| WebUI (webui/src/) | 14 | TypeScript / Vue |
| 模块脚本 (module/) | 8 | Shell / YAML / Fluent |
| 配置与文档 | 8 | Markdown / YAML / JSON |
| CI/CD | 2 | YAML |
| **总计** | **~53** | — |

---

*文档生成时间：2026年7月30日*
