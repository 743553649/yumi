# Yumi 项目架构地图

> 生成日期: 2026-08-13  
> 项目版本: v2.0.2  
> 仓库: https://github.com/imacte/yumi  
文档版本: 2.0 (全面更新)

---

## 1. 项目概览

**Yumi** 是一个面向 Android 平台的 CPU 调度守护进程，使用 Rust 编写，通过 eBPF 内核探针实现零开销的帧率与 CPU 负载监控，结合 PID 控制的 FAS（帧感知调度）引擎与 CLG（CPU 负载调速器），为游戏场景和日常使用提供自适应频率调节。

### 1.1 核心特性

- **eBPF 内核探针**: 零开销监控 `Surface::queueBuffer` 帧提交事件与 `sched_switch` CPU 调度事件
- **FAS 帧感知调度**: 基于 PID 控制的帧率感知 CPU 频率调节，支持动态帧率档位匹配
- **CLG CPU 负载调速器**: 日常场景下基于 CPU 利用率的自适应频率调节
- **应用感知**: 通过 Cgroup 检测前台应用，自动切换调度策略
- **温度感知**: 实时温度监控与降频保护
- **WebUI 配置**: 基于 Vue3 的 KernelSU/Magisk 管理器内嵌配置界面
- **国际化**: 基于 Fluent 的多语言支持

### 1.2 目标平台

- **系统**: Android 8.0+ (API 26+)
- **架构**: ARM64 (AArch64)
- **要求**: Root 权限 (Magisk / KernelSU)
- **内核要求**: eBPF 支持 (`CONFIG_BPF`, `CONFIG_BPF_SYSCALL`)

---

## 2. 仓库结构

```
yumi/
├── Cargo.toml                  # 工作区根配置 (workspace: xtask + yumi-ebpf)
├── Cargo.lock                  # 依赖锁文件
├── build.rs                    # eBPF 编译脚本 (安装 bpf-linker, 编译 yumi-ebpf)
├── CLAUDE.md                   # AI Agent 协作与开发规范 (给 AI 的指令)
├── AGENTS.md                   # 同上, 别名文件
├── .cargo/
│   └── config.toml             # cargo 配置 (Android NDK 链接器设置)
├── .gitignore
│
├── src/                        # ★ Rust 守护进程源码 (核心)
│   ├── main.rs                 # 入口: 初始化日志/i18n, 启动 Monitor + Scheduler
│   ├── common.rs               # 公共类型 (DaemonEvent 总线枚举), 模块根路径计算
│   ├── utils.rs                # 工具函数: FastWriter, SysPathExist, 文件/温度读写
│   ├── logger.rs               # 日志系统: log4rs 滚动文件, 5MB×3 轮转, 动态级别
│   ├── i18n.rs                 # 国际化: Fluent FTL 加载, 运行时切换语言
│   ├── fas_types.rs            # FAS 配置类型: PID 系数, ClusterProfile, FasRulesConfig
│   │
│   ├── monitor/                # 监控子系统 (数据采集)
│   │   ├── mod.rs              # 线程编排: 启动 4 个工作线程 + 1 主循环
│   │   ├── app_detect.rs       # Cgroup 前台应用检测 + 500ms 防抖
│   │   ├── fps_monitor.rs      # eBPF uprobe → libgui::queueBuffer, mio 轮询 RingBuf
│   │   ├── cpu_monitor.rs      # eBPF tracepoint → sched_switch, TGID/线程双路径
│   │   ├── screen_detect.rs    # Netlink uevent 监听屏幕亮灭
│   │   └── config.rs           # RulesConfig 结构 + get_rules_path()
│   │
│   └── scheduler/              # 调度子系统 (决策与执行)
│       ├── mod.rs              # IPC 事件循环, 状态机, 线程编排
│       ├── scheduler.rs        # 一次性系统调优: CPU idle governor, IO 设置
│       ├── config.rs           # 主配置结构: Meta, Mode, CpuLoadGovernorConfig, IOSettings
│       ├── cpu_load_governor.rs# ★ CLG: CPU 负载调速器 (sysfs 快照/恢复, 调频算法)
│       └── fas/                # ★ FAS: 帧感知调度器
│           ├── mod.rs          # 子模块公开
│           ├── controller.rs   # FasController 主结构 + 生命周期管理 + CPU 负载接口
│           ├── frame_pipeline.rs# 帧流水线: Phase 1~2 (冷启动, 加载检测)
│           ├── gear_state.rs   # Phase 3: 齿轮决策 (升/降档, 阻尼, 原生档位探测)
│           ├── pid.rs          # PID 控制器 (动态系数缩放, util-gain 调制)
│           ├── pid_jank.rs     # Phase 4: Jank 检测 + PID 计算 + 紧急跳频 + 恢复保护
│           ├── fps_window.rs   # 120 帧滑动窗口 (均值/标准差/近期均值)
│           ├── policy_controller.rs # Phase 5: 单 Policy 频率写入 + 1.5s 验证
│           └── policy_mgmt.rs  # Policy 初始化/热重载/频率分配 (capacity_weight)
│
├── yumi-ebpf/                  # ★ eBPF 内核探针程序 (no_std)
│   ├── Cargo.toml              # 依赖: aya-ebpf
│   └── src/main.rs             # handle_frame(uprobe) + handle_sched_switch(tracepoint)
│
├── webui/                      # Vue3 WebUI 配置界面
│   ├── package.json
│   ├── vite.config.ts
│   ├── index.html
│   ├── src/
│   │   ├── main.ts             # 入口: 挂载 Pinia/Router/Vant/i18n
│   │   ├── App.vue             # Root: Vant ConfigProvider + RouterView
│   │   ├── i18n/               # vue-i18n 翻译
│   │   ├── router/index.ts     # 4 路由: Home / Apps / Config / Log
│   │   ├── stores/
│   │   │   ├── scheduler.ts    # Pinia: currentMode, appRules, isDaemonRunning
│   │   │   └── counter.ts      # 示例 (开发用)
│   │   ├── utils/
│   │   │   ├── bridge.ts       # KernelSU Magisk 桥接 (读写配置, exec, listPackages)
│   │   │   └── mock.ts         # 开发模式 Mock 桥接
│   │   └── views/
│   │       ├── HomeView.vue    # 主页: 模式切换, 守护进程状态, QQ 群链接
│   │       ├── AppRulesView.vue# 应用规则: 为每个应用绑定调度模式
│   │       ├── ConfigEditorView.vue # 核心配置编辑 (原 CoreConfigView)
│   │       └── LogViewerView.vue    # 日志查看器
│   └── ...
│
├── xtask/                      # 构建编排工具
│   ├── Cargo.toml              # 依赖: clap, xshell, zip, fs_extra, toml, chrono
│   └── src/
│       ├── main.rs             # cargo xtask build: 编译 WebUI → Rust → 打包 zip
│       └── zip_ext.rs          # 目录递归压缩工具
│
├── module/                     # Magisk/KernelSU 模块打包
│   ├── module.prop             # 模块元数据 (id, name, version, author)
│   ├── customize.sh            # 安装脚本: 欢迎信息, 自动检测语言
│   ├── service.sh              # 开机启动: 等待系统启动, kill 旧进程, nohup 启动
│   ├── uninstall.sh            # 卸载脚本
│   ├── META-INF/               # Magisk 签名/兼容
│   ├── config/
│   │   ├── config.yaml         # 主配置文件
│   │   └── i18n/               # Fluent 翻译文件
│   │       ├── en.ftl
│   │       └── zh.ftl
│   ├── rules.yaml              # 应用规则 (WebUI 配置入口)
│   ├── scripts/                # 辅助脚本 (预留)
│   └── .gitignore
│
├── updateInformation/          # OTA 更新
│   ├── changelog.md            # 版本更新日志
│   └── update.json             # 更新元数据 (供 Manager 检测)
│
├── docs/
│   └── architecture.md         # 此文: 项目架构地图
│
└── .github/
    ├── dependabot.yml          # 依赖自动更新
    └── workflows/build.yml     # CI/CD: 每次推送自动构建发布包
```

---

## 3. 运行时架构

### 3.1 进程模型

```
┌─────────────────────────────────────────────────────────────┐
│                        yumi 守护进程                          │
│                                                              │
│  ┌─────────────┐         mpsc channel         ┌───────────┐  │
│  │  Monitor    │  ───── DaemonEvent ──────▶  │ Scheduler │  │
│  │  Thread     │                              │  Thread   │  │
│  └──────┬──────┘                              └─────┬─────┘  │
│         │                                           │        │
│  ┌──────┴──────┐                            ┌──────┴──────┐  │
│  │ screen_watcher│                          │ config_watcher│ │
│  │ config_watcher│                          │   scheduler   │  │
│  │ fps_monitor_ebpf│                        │     IPC       │  │
│  │ cpu_monitor_ebpf│                        └───────────────┘  │
│  │ app_detection   │                                          │
│  │   (blocking)    │                                          │
│  └─────────────────┘                                          │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 线程职责

| 线程名 | 来源 | 职责 |
|--------|------|------|
| `main` | `src/main.rs` | 初始化日志、i18n、chdir，启动 Monitor 和 Scheduler 线程 |
| `screen_watcher` | `monitor/mod.rs` | 监听 Netlink uevent，检测屏幕亮/灭状态 |
| `config_watcher` | `monitor/mod.rs` | 使用 inotify 监听 `rules.yaml` 变更 |
| `fps_monitor_ebpf` | `monitor/mod.rs` | Tokio 运行时，管理 eBPF uprobe FPS 探针 |
| `cpu_monitor_ebpf` | `monitor/mod.rs` | Tokio 运行时，管理 eBPF tracepoint CPU 探针 |
| `app_detection` | `monitor/mod.rs` | 主循环，从 Cgroup 读取前台应用，500ms 防抖 |
| `config_watcher` | `scheduler/mod.rs` | 监听 `config/` 目录变更，热重载配置 |
| `scheduler_ipc` | `scheduler/mod.rs` | 事件循环，驱动 FAS/CLG，写入 sysfs |

---

## 4. 事件总线 (DaemonEvent)

```rust
pub enum DaemonEvent {
    // 低频事件: 前台应用切换或环境温度变化
    ModeChange { package_name: String, pid: i32, mode: String, temperature: f64 },
    // 高频事件: eBPF 捕获的帧间隔 (纳秒)
    FrameUpdate { frame_delta_ns: u64 },
    // 中频事件: eBPF 系统负载更新
    SystemLoadUpdate { core_utils: Vec<f32>, foreground_max_util: f32 },
    // 配置热重载
    ConfigReload(RulesConfig),
    // 屏幕状态变化
    ScreenStateChange(bool),
}
```

### 4.1 事件流向

```
Monitor 子系统                              Scheduler 子系统
┌─────────────────┐                        ┌─────────────────┐
│ app_detect.rs   │──ModeChange──────────▶│                 │
│ fps_monitor.rs  │──FrameUpdate─────────▶│   scheduler/    │──▶ sysfs (scaling_max/min_freq)
│ cpu_monitor.rs  │──SystemLoadUpdate────▶│   mod.rs 事件循环│
│ screen_detect.rs│──ScreenStateChange───▶│                 │
│ config.rs       │──ConfigReload────────▶│                 │
└─────────────────┘                        └─────────────────┘
```

---

## 5. 监控子系统 (Monitor)

### 5.1 前台应用检测 (`app_detect.rs`)

**机制**: 读取 Cgroup 的 `top-app` 组中的进程列表，通过 `/proc/<pid>/cmdline` 获取包名。

**搜索路径** (按优先级):
1. `/dev/cpuset/top-app/cgroup.procs`
2. `/sys/fs/cgroup/cpuset/top-app/cgroup.procs`
3. `/dev/stune/top-app/cgroup.procs`

**过滤规则**:
- 排除系统应用 (systemui, system_server, surfaceflinger 等)
- 排除输入法 (自动检测系统输入法列表，含备用硬编码)
- 排除用户配置的 `ignored_apps`
- 500ms 无阻塞防抖，避免快速切换导致频繁模式变更

**全局变量**:
- `CURRENT_PID`: 当前前台应用 PID (AtomicI32)
- `CURRENT_PACKAGE`: 当前前台包名 (Arc<Mutex<String>>)

### 5.2 FPS 监控 (`fps_monitor.rs`)

**机制**: eBPF uprobe 附加到 `libgui.so` 的 `Surface::queueBuffer` 符号。

**架构**:
- `FpsManager`: 单 eBPF 实例，支持多 PID 动态 attach/detach
- `ProbeState`: 单 PID 的帧统计，使用 144 帧滑动窗口
- 使用 `mio` 轮询 RingBuf fd，避免忙等待
- PID 变化通过 `tokio::sync::watch` 通道通知

**符号匹配** (fallback):
- 主: `_ZN7android7Surface11queueBufferEP19ANativeWindowBufferi`
- 备: `_ZN7android7Surface11queueBufferEP19ANativeWindowBufferiPNS_24SurfaceQueueBufferOutputE`

### 5.3 CPU 监控 (`cpu_monitor.rs`)

**机制**: eBPF tracepoint 附加到 `sched/sched_switch`。

**双路径利用率计算**:
1. **主路径 (TGID 聚合)**: 通过 `TGID_RUN_TIME` map 查询前台进程的总运行时间
   - 只需查询 1 个 key
   - 1024 条目，无 HASH 驱逐问题
   - 包含 pending delta 补偿

2. **降级路径 (线程级)**: 遍历 `/proc/<pid>/task/` 下的所有 TID，查询 `THREAD_RUN_TIME`
   - 防驱逐保护: 如果新值 < 旧值，跳过该 TID

**全局利用率**: 每核 idle/busy 时间累计，含实时 pending delta 补偿。

### 5.4 屏幕检测 (`screen_detect.rs`)

**机制**: Netlink `NETLINK_KOBJECT_UEVENT` socket 监听内核 uevent。

**事件来源**:
- `power` subsystem: `early_suspend` / `late_resume`
- `backlight` subsystem: 读取 `bl_power` 或 `actual_brightness` 确认状态

---

## 6. 调度子系统 (Scheduler)

### 6.1 事件循环状态机 (`scheduler/mod.rs`)

Scheduler IPC 线程从 mpsc 接收器消费 `DaemonEvent`，维护以下状态:

- `current_mode`: 当前性能模式 (powersave/balance/performance/fast/fas)
- `is_screen_on`: 屏幕状态标记
- `fas_suspended_at`: FAS 挂起时间戳 (5 秒 grace period)
- `fas_suspended_package`: 挂起时的包名

**状态转换**:
```
亮屏 + 非游戏 ──▶ CLG 接管 (balance/performance/powersave/fast)
亮屏 + 游戏    ──▶ FAS 接管
息屏          ──▶ Doze 模式 (强制 CLG powersave, perf_ceil ≤ 0.40)
```

### 6.2 CLG - CPU 负载调速器 (`cpu_load_governor.rs`)

**接管流程**:
1. 对每个 CPU Policy 快照原始状态 (`PolicyRestore`): governor, min_freq, max_freq, hw_max
2. 设置 governor 为 `performance`
3. 通过 `FastWriter` 直接写入 `scaling_max_freq` / `scaling_min_freq`

**释放流程** (安全恢复):
1. 恢复 governor
2. 先放宽 max 到 hw_max (恒 >= 当前 min)
3. 恢复 min_freq
4. 恢复 max_freq

**调频算法**:
- 每 tick 计算每 cluster 的 `max_util`
- 尖峰抑制: 单 tick util 跳升超过阈值时，增量按 `spike_decay` 衰减
- Headroom 因子: 在 `up_threshold` 附近线性过渡，避免阶跃振荡
- 升频: 需连续 `up_rate_limit_ticks` 才执行，分高速/中速/滞回带慢速三档
- 降频: 极低负载立即快速降频，正常负载需 `down_rate_limit_ticks` 确认

**配置参数** (关键值):
```
up_threshold: 0.80         # 升频阈值
down_threshold: 0.50       # 降频阈值
smoothing_up: 0.60         # 升频平滑系数
smoothing_down: 0.30       # 降频平滑系数
perf_floor: 0.15           # 性能下限
perf_ceil: 1.0             # 性能上限
spike_jump_threshold: 0.35 # 尖峰检测阈值
```

### 6.3 FAS - 帧感知调度 (fas/)

#### 6.3.1 整体架构

```
FasController
├── cfg: FasRulesConfig         # 完整配置 (含 normalize 校验)
├── FpsWindow                   # 120 帧滑动窗口 (均值/标准差/近期均值)
├── PidController               # 动态 PID (系数按 target_fps 缩放)
├── PolicyController[]          # 每 CPU Policy 一个，管理 FastWriter + 频率验证
├── fps_gears: Vec<f32>         # 当前有效帧率档位 [30,60,90,120,144]
├── current_target_fps: f32     # 当前目标档位 (如 60/90/120)
├── perf_index: f32             # 当前性能指数 (0.0~1.0)
├── ema_actual_ms: f32          # 实际帧时间 EMA
├── fps_margin: f32             # 帧率余量 (默认 3.0fps)
│
├── 加载检测状态
│   ├── is_loading: bool
│   ├── loading_frames / loading_cumulative_ms
│   └── post_loading_ignore / post_loading_downgrade_guard
│
├── 齿轮档位状态
│   ├── upgrade_confirm_frames / downgrade_confirm_frames
│   ├── gear_dampen_frames      # 齿轮切换阻尼期
│   ├── consecutive_downgrade_count / stable_gear_frames
│   └── downgrade_boost_active  # 降档时临时抬频防卡顿
│
├── Jank 保护状态 (v2.0.2 新增)
│   ├── jank_cooldown / jank_streak
│   ├── post_jank_perf_floor    # crit/heavy 后的 perf 最低保护
│   ├── post_jank_guard_frames  # 保护期剩余帧数 (线性衰减)
│   └── floor_stuck_frames      # perf 地板死锁检测
│
├── CPU 负载集成 (v2.0.2 新增)
│   ├── foreground_max_util     # 前台最重线程利用率 (raw)
│   ├── ema_fg_util             # EMA 平滑后 (升 0.40 / 降 0.15)
│   ├── core_utils: Vec<f32>    # 各核心利用率快照
│   ├── target_fps_offset       # 基于 util 的 target_fps 偏移 [-3.0, 0.0]
│   └── util_sample_timer       # 每秒采样一次
│
├── 温度感知
│   ├── current_temperature     # 当前 CPU 温度 (℃)
│   └── temp_threshold          # 降频阈值
│
└── 其他
    ├── current_package: String  # 当前游戏包名
    ├── active_profile: Option<PerAppProfile>
    ├── freq_force_counter       # 强制重写间隔
    └── init_time: Instant       # 冷启动计时
```

#### 6.3.2 帧处理流水线 (6 阶段)

**Phase 1: 冷启动 & 应用切换**
- 启动后 `cold_boot_ms` (默认 3500ms) 内锁定 `perf_cold_boot` (0.85)
- 检测到应用切换 (帧间隔 > `app_switch_gap_ms`) 时重置状态

**Phase 2: 加载检测**
- 重帧 (> `heavy_frame_threshold_ms`) 累计超过 `loading_cumulative_ms` 判定为加载中
- 加载期间 perf 锁定在 `loading_perf_floor` ~ `loading_perf_ceiling`
- 加载结束后进入 `post_loading_perf` 保护期

**Phase 3: 齿轮决策 (`gear_state.rs`)**
- 检测游戏原生帧率档位 (30/60/90/120/144)
- 升档/降档需连续确认帧数 (`upgrade_confirm_frames` / `downgrade_confirm_frames`)
- 齿轮切换后进入 `gear_dampen_frames` 阻尼期，PID 重置
- 降档触发 `downgrade_boost`: 临时提升 perf 防止卡顿

**Phase 4: PID 计算 (`pid.rs` + `pid_jank.rs`)**
- 动态 PID 系数: `kp` 线性缩放, `ki` sqrt 缩放, `kd` pow(0.3) 缩放
- Util-gain 调制: 前台利用率低时衰减 P 项，避免 GPU-bound 场景无效拉频
- Jank 检测: 帧时间超过 crit_ms 时触发紧急升频，重置 util 偏移

**Phase 5: 频率应用 (`policy_mgmt.rs`)**
- perf_index 按 cluster capacity_weight 分配到各 Policy
- 通过 `FastWriter` 写入 sysfs
- 每 1.5s 验证 `scaling_cur_freq`，如被内核覆盖则重写

**Phase 6: 温度保护**
- 每 3 秒读取 CPU 温度
- 超过 `core_temp_threshold` 时强制限制 perf 到 `core_temp_throttle_perf`

---

## 7. eBPF 程序 (`yumi-ebpf/src/main.rs`)

### 7.1 FPS 探针 (uprobe)

```c
// 附加到 libgui.so::Surface::queueBuffer
handle_frame() {
    pid = current_pid_tgid >> 32;
    ktime_ns = bpf_ktime_get_ns();
    RING_BUF.reserve({pid, ktime_ns});
}
```

### 7.2 CPU 探针 (tracepoint)

```c
// 附加到 sched/sched_switch
handle_sched_switch(prev_pid, next_pid) {
    now = bpf_ktime_get_ns();
    delta = now - last_switch_time;
    
    if (prev_pid == 0) {
        CORE_IDLE_TIME += delta;
    } else {
        CORE_BUSY_TIME += delta;
        THREAD_RUN_TIME[prev_tid] += delta;
        TGID_RUN_TIME[prev_tgid] += delta;
    }
    
    CORE_LAST_TIME = now;
    CORE_CURRENT_TID = next_pid;
    CORE_CURRENT_TGID = next_tgid;
}
```

**eBPF Maps**:
| Map | 类型 | 用途 |
|-----|------|------|
| `RING_BUF` | RingBuf | FPS 事件输出到用户态 |
| `CORE_IDLE_TIME` | PerCpuArray | 每核累计 Idle 时间 |
| `CORE_BUSY_TIME` | PerCpuArray | 每核累计 Busy 时间 |
| `CORE_LAST_TIME` | PerCpuArray | 每核上次切换时间戳 |
| `CORE_CURRENT_TID` | PerCpuArray | 每核当前 TID |
| `CORE_CURRENT_TGID` | PerCpuArray | 每核当前 TGID |
| `THREAD_RUN_TIME` | HashMap | 线程级运行时间 (32768 条目) |
| `TGID_RUN_TIME` | HashMap | TGID 级聚合运行时间 (1024 条目) |

---

## 8. WebUI 架构

### 8.1 技术栈

- **框架**: Vue 3 + Vite
- **UI 组件**: Vant 4
- **状态管理**: Pinia
- **路由**: Vue Router (hash 模式)
- **国际化**: vue-i18n
- **构建输出**: 静态文件嵌入模块 `webroot/` 目录

### 8.2 桥接层 (`bridge.ts`)

WebUI 通过 KernelSU/Magisk 提供的 `window.ksu` API 执行 shell 命令与系统交互:

```typescript
// 核心操作
exec("cat /data/adb/modules/yumi/rules.yaml")      // 读取配置
exec("echo ... > /data/adb/modules/yumi/rules.yaml") // 写入配置
exec("pidof yumi")                                  // 检测守护进程
listPackages('user')                                // 获取已安装应用
```

**开发模式**: `window.ksu` 未定义时自动降级到 `MockBridge`。

### 8.3 视图

| 视图 | 文件 | 功能 |
|------|------|------|
| 主页 | `HomeView.vue` | 模式切换 (powersave/balance/performance/fast)、守护进程状态、QQ群 |
| 应用规则 | `AppRulesView.vue` | 为每个应用绑定调度模式，自动初始化 FAS 配置 |
| 核心配置 | `ConfigEditorView.vue` | FAS 参数、CLG 参数、IO 设置、Cpuidle Governor |
| 日志查看 | `LogViewerView.vue` | 查看 `logs/daemon.log` 运行日志 |

---

## 9. 模块架构

### 9.1 模块结构 (安装后)

```
/data/adb/modules/yumi/
├── module.prop           # 模块元数据
├── service.sh            # 开机启动脚本
├── customize.sh          # 安装脚本
├── core/
│   └── bin/
│       └── yumi          # Rust 守护进程二进制
├── config/
│   ├── config.yaml       # 主配置
│   └── i18n/             # 翻译文件
├── logs/
│   └── daemon.log        # 运行日志 (5MB × 3 轮转)
├── current_mode.txt      # 当前模式标记
├── rules.yaml            # 应用规则
└── webroot/              # WebUI 静态文件
    ├── index.html
    └── assets/
```

### 9.2 启动流程

1. **系统启动完成**: `service.sh` 等待 `sys.boot_completed=1`
2. **清理旧进程**: `killall -9 yumi`
3. **设置权限**: `chmod 755 $MODDIR/core/bin/yumi`
4. **启动守护进程**: `nohup $DAEMON_PATH > /dev/null 2>&1 &`

---

## 10. 配置系统

### 10.1 主配置 (`config/config.yaml`)

```yaml
Meta:
  Loglevel: "INFO"
  Language: "zh"

# 功能开关
CpuIdleScalingGovernor: true
IOOptimization: true

# IO 设置
IO_Settings:
  Scheduler: "none"
  read_ahead_kb: "128"
  nomerges: "2"
  iostats: "0"

# Cpuidle Governor
CpuIdle:
  current_governor: "menu"

# 各模式 CLG 配置
balance:
  CpuLoadGovernor:
    enabled: true
    up_threshold: 0.80
    down_threshold: 0.50
    smoothing_up: 0.60
    smoothing_down: 0.30
    perf_floor: 0.15
    perf_ceil: 1.0
    ...

powersave:  { CpuLoadGovernor: ... }
performance: { CpuLoadGovernor: ... }
fast:       { CpuLoadGovernor: ... }
```

### 10.2 规则配置 (`rules.yaml`)

```yaml
yumi_scheduler: true
dynamic_enabled: true
global_mode: "balance"
app_modes:
  "com.miHoYo.GenshinImpact": "fas"
  "com.tencent.tmgp.sgame": "fas"
ignored_apps: ["com.example.benchmark"]

fas_rules:
  fps_gears: [30, 60, 90, 120, 144]
  fps_margin: 3.0
  pid:
    kp: 0.050
    ki: 0.010
    kd: 0.006
  cluster_profiles:
    - { capacity_weight: 1.0 }   # 小核
    - { capacity_weight: 1.5 }   # 中核
    - { capacity_weight: 2.5 }   # 大核
    - { capacity_weight: 3.5 }   # 超大核
  per_app_profiles:
    "com.miHoYo.GenshinImpact":
      target_fps: [30, 60]
      fps_margin: 4.0
```

---

## 11. 构建系统

### 11.1 依赖工具链

- Rust nightly + `rust-src` component
- `cargo-ndk` (Android NDK 交叉编译)
- Android NDK r29
- `bpf-linker` (eBPF 链接器)
- Node.js 24 (WebUI 构建)

### 11.2 构建流程

```
cargo xtask build
    ├── 1. npm run build (webui/)          → webroot/
    ├── 2. cargo-ndk aarch64 release (src/) → core/bin/yumi
    │       └── build.rs 编译 yumi-ebpf (bpfel-unknown-none)
    └── 3. 打包: module/ + webroot/ + core/ → yumi-v2.0.2.zip
```

### 11.3 CI/CD

GitHub Actions (`.github/workflows/build.yml`):
- Ubuntu runner
- 安装 Node.js 24, Rust nightly, Android NDK r29
- 运行 `cargo xtask build`
- 上传 `yumi-*.zip` artifact

---

## 12. 安全与鲁棒性机制

### 12.1 FastWriter (`utils.rs`)

写入 sysfs 前调用 `libc::umount2(MNT_DETACH)` 解除可能的 mount 覆盖，确保写入成功。

### 12.2 频率验证 (`policy_controller.rs`)

每 1.5 秒读取 `scaling_cur_freq`，若内核/驱动覆盖了写入值，自动重新 unmount 并重写。

### 12.3 Panic 捕获

Scheduler IPC 事件循环包裹在 `std::panic::catch_unwind` 中，防止调度线程静默死亡导致频率悬停。

### 12.4 配置安全

所有浮点配置均经过 `normalize()` 处理:
- 非有限值 (NaN/Inf) 回退默认值
- 阈值限制在 [0, 1] 区间
- 交叉约束 (floor ≤ ceil ≤ init) 防止 `clamp` panic

### 12.5 Doze 保护

息屏时自动生成极端省电配置，无视用户配置:
- `perf_ceil ≤ 0.40`
- `smoothing_up = 0.10` (升频迟钝)
- `smoothing_down = 1.0` (瞬间降频)

---

## 13. 数据流全景图

```
┌──────────────┐     uprobe      ┌─────────────┐
│ libgui.so    │────────────────▶│  RING_BUF   │────┐
│ queueBuffer  │                 │  (eBPF)     │    │
└──────────────┘                 └─────────────┘    │
                                                   ▼
┌──────────────┐   tracepoint   ┌─────────────┐  ┌──────────────┐
│ sched_switch │───────────────▶│ CORE_BUSY   │  │ fps_monitor  │
│ (kernel)     │                │ THREAD_RUN  │  │  (mio poll)  │
└──────────────┘                │ TGID_RUN    │  └──────┬───────┘
                                └─────────────┘         │
                                   │                    │ FrameUpdate
                                   │ SystemLoadUpdate   │
                                   ▼                    ▼
                         ┌─────────────────────────────────┐
                         │      Scheduler IPC Thread       │
                         │  ┌─────────┐    ┌─────────────┐ │
                         │  │  FAS    │ or │    CLG      │ │
                         │  │Controller│   │  Governor   │ │
                         │  └───┬─────┘    └──────┬──────┘ │
                         └──────┼─────────────────┼────────┘
                                │                 │
                    ┌───────────┘                 └───────────┐
                    ▼                                         ▼
            ┌─────────────┐                          ┌─────────────┐
            │ PolicyCtrl[]│                          │ PolicyCtrl[]│
            │ FastWriter  │                          │ FastWriter  │
            └──────┬──────┘                          └──────┬──────┘
                   │                                         │
                   ▼                                         ▼
        ┌─────────────────┐                        ┌─────────────────┐
        │ scaling_max_freq│                        │ scaling_max_freq│
        │ scaling_min_freq│                        │ scaling_min_freq│
        │ scaling_governor│                        │ scaling_governor│
        └─────────────────┘                        └─────────────────┘
```

---

## 14. 术语表

| 术语 | 全称 | 说明 |
|------|------|------|
| **FAS** | Frame-Aware Scheduling | 帧感知调度，基于帧时间的游戏场景 CPU 频率控制 |
| **CLG** | CPU Load Governor | CPU 负载调速器，日常场景的自适应频率调节 |
| **eBPF** | extended Berkeley Packet Filter | Linux 内核可编程探针框架 |
| **uprobe** | User-space probe | 用户态函数探针 |
| **tracepoint** | Kernel tracepoint | 内核静态插桩点 |
| **PID** | Proportional-Integral-Derivative | PID 控制器，用于闭环反馈控制 |
| **Cgroup** | Control Group | Linux 进程分组机制，用于识别前台应用 |
| **TGID** | Thread Group ID | 线程组 ID，即进程 PID |
| **TID** | Thread ID | 线程 ID |
| **Policy** | CPUFreq Policy | Linux CPU 频率策略，通常对应一个 CPU Cluster |
| **Perf** | Performance Index | 性能指数，0.0~1.0 表示从最低到最高频率的比例 |
| **Gear** | FPS Gear | 帧率档位 (30/60/90/120/144fps) |
| **Jank** | Frame Jank | 帧卡顿，指帧时间超过预算的渲染帧 |
| **Doze** | Deep Doze | 息屏深度睡眠模式 |
| **Headroom** | Performance Headroom | 性能余量，在目标利用率之上预留的缓冲 |
| **Magisk** | - | Android Root 方案，支持模块系统 |
| **KernelSU** | - | 基于内核的 Android Root 方案 |

---

---

## 15. 构建与部署流水线

### 15.1 .cargo/config.toml

```toml
[target.aarch64-linux-android]
linker = "aarch64-linux-android21-clang"
```

指定 Android NDK 的交叉链接器，确保 Rust 可以编译出 ARM64 原生二进制。

### 15.2 build.rs (编译期 eBPF 构建)

在 `cargo build` 主 Rust 程序之前自动执行：

1. `cargo install bpf-linker --force` — 安装 eBPF 链接器
2. `cargo build --target bpfel-unknown-none -Z build-std=core` — 编译 `yumi-ebpf/` 中的 no_std eBPF 程序
3. 产物路径: `target/.../bpfel-unknown-none/release/yumi-ebpf`

产物在 Rust 主程序中以 `include_bytes!()` 方式嵌入为静态字节数组。

### 15.3 xtask 构建编排 (cargo xtask build)

| 步骤 | 命令 | 产物 |
|------|------|------|
| 1. WebUI | `npm run build` (在 webui/ 目录) | `webui/dist/` → 模块 webroot/ |
| 2. Rust Core | `cargo +nightly ndk --platform 26 -t arm64-v8a build -Z build-std -r` | `target/aarch64-linux-android/release/yumi` → `core/bin/yumi` |
| 3. 打包 | 拷贝 module/ + webroot/ + core/bin → 临时目录 → zip | `output/yumi-v2.0.2-<commit>-<datetime>.zip` |

构建产物是一个标准的 Magisk/KernelSU 模块 ZIP，可直接刷入或通过 Manager 安装。

---

## 16. 开发规范与 AI 协作 (CLAUDE.md / AGENTS.md)

项目包含两份给 AI Agent 的开发规范文件:

### 架构原则

| 原则 | 说明 |
|------|------|
| **极简实现** | 选择满足当前需求的最简实现，绝不搞推测性抽象 |
| **端到端先行** | 先跑通 Rust 核心闭环，不为未来复杂度拆除可运行功能 |
| **模块解耦** | FAS / CLG / IPC 严格解耦，强类型约束 |
| **成熟库优先** | 新增依赖前先盘点 `Cargo.toml` 已有能力 |
| **终态决策** | 每次修改必须做可持续的终态架构决策，不做"先这样以后换" |

### 文档规范

- 功能/逻辑/IPC/配置/交互/模块职责变化时必须同步更新 README、架构文档、接口文档
- 宣布完成前必须检查文档同步
- 正式说明书语气，严禁补丁式措辞（如"修订说明""当前改为"）

### 编码协作

- 跨模块公共文件或核心配置变动必须立刻停止并请示用户
- 用户是底层小白，使用通俗大白话与生活比喻沟通
- Git 提交遵循 Conventional Commits: `feat(fas): ...`, `fix(clg): ...`

---

## 17. 模块生命周期

### 17.1 安装

1. Magisk Manager / KernelSU Manager 刷入 `yumi-*.zip`
2. `customize.sh` 执行: 显示欢迎信息 (自动检测中文)
3. 模块文件解压到 `/data/adb/modules/yumi/`
4. 下次重启时 `service.sh` 自动启动守护进程

### 17.2 启动流程

```
系统启动 → boot_completed=1 → service.sh 执行
    ├── killall -9 yumi            # 清理残留进程
    ├── chmod 755 core/bin/yumi    # 设置权限
    └── nohup core/bin/yumi > /dev/null &
        └── main.rs:
            ├── chdir 到模块根目录
            ├── 读取 config.yaml → 加载语言 → 初始化日志
            ├── 创建 mpsc channel
            ├── 启动 Scheduler IPC 线程 (事件循环)
            └── 启动 Monitor 线程 (4 个子线程 + app_detection 主循环)
```

### 17.3 OTA 更新

`updateInformation/update.json` 被 KernelSU/Magisk Manager 读取，自动检测新版本。  
`changelog.md` 记录每个版本的变更明细。

### 17.4 卸载

`uninstall.sh` 在 Manager 卸载模块时执行，清理运行时文件。

---

## 18. Rust 依赖全景

| 分类 | 依赖 | 用途 |
|------|------|------|
| **eBPF** | `aya 0.14` | 加载 eBPF 程序，操作 RingBuf/PerCpuArray/HashMap |
| **eBPF (probe)** | `aya-ebpf 0.2` | eBPF 探针端 (no_std)，宏与 maps |
| **异步** | `tokio 1` (full) | FPS/CPU 监控环的 Tokio 运行时 |
| **IO 轮询** | `mio 1` (os-ext) | 非阻塞轮询 eBPF RingBuf fd |
| **序列化** | `serde 1`, `serde_yaml 0.9` | 配置/规则 YAML 读写 |
| **Netlink** | `netlink-sys 0.8`, `kobject-uevent 0.2` | 屏幕状态 uevent 监听 |
| **文件监控** | `inotify 0.11` | 配置热重载 |
| **日志** | `log4rs 1.4`, `log 0.4` | 滚动文件日志 (5MB×3) |
| **国际化** | `fluent 0.17`, `fluent-bundle 0.16` | FTL 格式多语言 |
| **系统接口** | `nix 0.31` (fs, signal, inotify) | umount2, sysfs 操作 |
| **sysfs 写入** | `libc 0.2` | setrlimit(RLIMIT_MEMLOCK), umount2(MNT_DETACH) |

---

## 19. 关键数据结构总览

### eBPF Maps (内核侧)

```
RING_BUF        RingBuf       帧事件输出队列 (0x8000 bytes)
CORE_IDLE_TIME  PerCpuArray   每核累计 Idle 时间 (ns)
CORE_BUSY_TIME  PerCpuArray   每核累计 Busy 时间 (ns)
CORE_LAST_TIME  PerCpuArray   每核上次 sched_switch 时间戳
CORE_CURRENT_TID PerCpuArray  每核当前运行 TID
CORE_CURRENT_TGID PerCpuArray 每核当前运行 TGID
THREAD_RUN_TIME HashMap<u32, u64> 线程级运行时间 (32768 条目)
TGID_RUN_TIME   HashMap<u32, u64> TGID 级运行时间 (1024 条目)
```

### DaemonEvent 总线 (用户态)

```
ModeChange        { package_name, pid, mode, temperature }  — 应用/温度变化
FrameUpdate       { frame_delta_ns }                       — eBPF 帧间隔 (ns)
SystemLoadUpdate  { core_utils, foreground_max_util }      — eBPF CPU 负载
ConfigReload      (RulesConfig)                            — 配置热重载
ScreenStateChange (bool)                                   — 屏幕亮灭
```

### 配置层级

```
┌───────────────────────────────────────────────────────┐
│  config.yaml (主配置)                                    │
│  ├── Meta: loglevel, language                          │
│  ├── FunctionToggles: cpu_idle, io_optimization        │
│  ├── IO_Settings: scheduler, read_ahead_kb, etc        │
│  ├── CpuIdle: current_governor                         │
│  └── Modes: powersave, balance, performance, fast      │
│       └── CpuLoadGovernorConfig (per-mode CLG 参数)      │
├───────────────────────────────────────────────────────┤
│  rules.yaml (应用规则)                                   │
│  ├── global_mode, dynamic_enabled                      │
│  ├── app_modes: HashMap<包名, 模式>                     │
│  ├── ignored_apps                                      │
│  └── fas_rules: FasRulesConfig                         │
│       ├── fps_gears, fps_margin, pid 系数              │
│       ├── cluster_profiles (capacity_weight)           │
│       ├── per_app_profiles (per-app fps 档位)           │
│       ├── 加载检测 / 齿轮 / jank / 温度参数              │
│       └── cold_boot / app_switch 参数                   │
└───────────────────────────────────────────────────────┘
```

---

## 20. 线程模型全景

```
┌─ main ─────────────────────────────────────────────────┐
│ main.rs: 初始化 + 启动子线程                              │
│                                                         │
│  monitor_thread ───────────────────────────────────┐   │
│  ├── screen_watcher    (Netlink uevent 监听)        │   │
│  ├── config_watcher    (inotify rules.yaml 监听)    │   │
│  ├── fps_monitor_ebpf  (Tokio + mio + eBPF RingBuf)│   │
│  │   └── PID 更新 Task (tokio::spawn, 500ms 轮询)   │   │
│  ├── cpu_monitor_ebpf  (Tokio + eBPF TracePoint)   │   │
│  │   └── PID 更新 Task (tokio::spawn, 500ms 轮询)   │   │
│  └── app_detection_loop (阻塞主循环: Cgroup 读取)    │   │
│                                                     │   │
│  scheduler_thread ────────────────────────────────┐ │   │
│  ├── config_watcher (inotify config/ 目录监听)     │ │   │
│  └── scheduler_ipc (事件循环: FAS / CLG / sysfs)  │ │   │
│                                                    │   │
│  ←─ mpsc channel (DaemonEvent) ────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

---

*本文档基于 yumi v2.0.2 源码生成，覆盖核心架构与关键实现细节。*
