# 骁龙 8 Elite 全大核架构 + eBPF 零耗高流畅调度迭代设计方案

- **日期**: 2026-08-04
- **项目版本**: yumi v3.1.0
- **目标芯片**: Snapdragon 8 Elite (骁龙 8 至尊版 / 2 x Oryon Prime + 6 x Oryon Performance)
- **核心目标**: 消除传统轮询开销，解决全大核架构日常发热与待机偏高痛点，打造小白友好的极速流畅与省电体验。

---

## 1. 痛点分析与隐患排查

在为小白用户设计底层系统调度时，最核心的原则是**“零安全隐患、零异常崩溃、无感优雅降级”**。

### 现有架构隐患与瓶颈：
1. **集群硬编码错位隐患**：旧代码硬编码了传统芯片的 3 集群（Policy 0/1/2），运行在仅有 2 集群（Policy 0/6）的骁龙 8 Elite 上会导致提频指令发往无效节点，引发提频失效或误提升。
2. **轮询性能隐患**：旧监控通过定时读取 `/proc` 和 `/sys` 文件获取负载与帧率，在低功耗状态下增加了额外的 CPU 唤醒与电量消耗。
3. **极速下潜卡顿隐患**：全大核架构没有传统小核，若静止下潜参数设得太深且没有“1ms 快出”机制，用户滑动屏幕的第一下会出现微小的延迟（微卡顿）。

---

## 2. 模块一：yumi-ebpf 内核级零开销监控

### 2.1 架构设计
构建运行在 Linux 内核层面的 eBPF 探针，彻底取代定时查文件的传统轮询。

* **CPU 调度探针 (`tp/sched/sched_switch`)**：直接在内核上下文捕获应用主线程与渲染线程（RenderThread）的 CPU 占用，消除 `/proc/stat` 读取开销。
* **帧绘制探针 (`uprobe/doFrame`)**：在 Android 绘图引擎（Choreographer）出口插入探针，微秒级记录每帧绘制耗时，准确捕捉微小掉帧。
* **无锁传输管道 (`Aya RingBuffer`)**：内核探针采集到数据后，通过内存共享环形缓冲区（RingBuffer）直接推给 Rust 守护进程，真正做到监控过程零 CPU 开销。

### 2.2 隐患防范与优雅降级（核心保障）
* **内核与探针兼容性双保险**：
  * 若部分第三方 ROM 剥离了 `Choreographer` 动态符号导致 `uprobe` 挂载失败，系统会自动降级尝试通用 `vsync` 追踪或 Tracepoint。
  * 若设备内核未开启 eBPF 支持，Rust 守护进程启动时会自动识别并**平滑降级（Fallback）**回到原有极低频率的系统接口读取模式，**绝对不会导致系统崩溃、掉线或卡死**。

---

## 3. 模块二：骁龙 8 Elite 全大核专属 TouchBoost 与 Idle Dive

### 3.1 硬件适配规则
* **Cluster 0 (Policy 0)**: 6 x Performance 性能大核 (600MHz - 3.53GHz)
* **Cluster 1 (Policy 6)**: 2 x Prime 超级大核 (800MHz - 4.32GHz)

### 3.2 TouchBoost (日常滑动与点击脉冲)
* **动态 Cluster 匹配**：启动时自动扫描系统的 `policy` 节点，精确绑定 8 Elite 的 Policy 0 与 Policy 6。
* **日常封印超级大核（双保险机制）**：
  * **机制 1：CPUSet 动态线程组绑核**：通过监听应用线程组 (`/proc/[tgid]/task`) 变动，确保新创建的渲染线程（RenderThread）与主线程实时绑定在 Policy 0（`cpu0-cpu5`），从源头上阻止前台线程分配至超级大核。
  * **机制 2：频率锁与 TouchBoost 门控**：日常滑动只对 Policy 0 触发 50ms 脉冲提频（1.8GHz~2.0GHz）；Policy 6（2 个超级大核 `cpu6-cpu7`）保持最低频（800MHz）且不触发 TouchBoost 提频，允许其随时深度休眠，彻底杜绝高功耗超级大核点亮发热。
  * **冷启动瞬间爆发**：仅在检测到应用冷启动（App 点击打开）的瞬间，才短暂放开限制，给予 Policy 6 100ms 超级提频，实现秒开后立刻恢复隔离。

### 3.3 Idle Dive (日常静止深层下潜)
* **300ms 极速深休眠**：
  * 当手指离开屏幕且停顿看图/看文字超过 300ms 时，自动将 Policy 0 归位至 600MHz，Policy 6 归位至 800MHz。
  * 将内核 `cpuidle latency_us` 参数放宽至 500μs ~ 1000μs（若节点不可写则自动兼容 `/dev/cpu_dma_latency` PM-QoS 接口），允许 8 个大核进入最深度的 C2/C3 休眠状态，切断闲置核心时钟。
* **1ms 快出响应机制**：
  * 一旦重新检测到触摸信号，1ms 内瞬间恢复正常 `latency_us`（100μs），确保再次滑动时第 1 帧零延迟响应，毫无卡顿感。
* **息屏 Doze 后台保护**：
  * 息屏时自动恢复默认后台 CPUSet，确保后台音乐（如网易云/QQ音乐）无卡顿流畅播放。

---

## 4. 模块三：Rust 底层守护进程工程与编码规范

1. **eBPF 与用户态数据对齐规范 (`#[repr(C, align(8))]`)**：
   * eBPF 内核层与 Rust 用户态通过 RingBuffer 传递的数据结构必须严格采用 `#[repr(C, align(8))]` 8 字节对齐，防止 ARM64 架构出现指针内存未对齐错误（Alignment Fault）。
2. **读写锁与高并发性能规范 (快照拷贝防死锁)**：
   * 严禁在持有 `RwLock` 读锁的同时执行耗时的文件 I/O 写入。必须采用“读锁快速快照拷贝”后立刻 `drop(cfg)`，避免高频 TouchBoost 触发时主线程锁死。
3. **句柄管理与防泄露 (FastWriter 安全重置)**：
   * 针对 8 Elite 动态 Policy 的节点写入器（`FastWriter`），在配置热重载或 Cluster 重新探测时执行显式 Drop，防止文件描述符泄露（Too Many Open Files）。
4. **打包与构建安全规范 (LF 换行符强制转换)**：
   * 所有 Shell 脚本（`.sh`）与新增配置文件（`.yaml`/`.prop`）强制执行 Unix LF (`\n`) 换行符检查，阻止 Windows 环境构建引入 CRLF 导致 Android 解析中断。
   * 安装与更新脚本（`customize.sh`）严格保护用户的 `rules.yaml` 自定义规则，防止覆写。

---

## 5. 验证标准与测试闭环 (TDD 与代码规范)

1. **单函数行数与单一职责**：新增及重构函数严格控制在 50 行以内，复杂逻辑必须按职责拆分模块。
2. **语法与类型检查**：跨平台交叉编译检查 `$env:YUMI_SKIP_EBPF=1; cargo check --target aarch64-linux-android` 100% 通过。
3. **状态机与并发单元测试**：针对 8 Elite 动态 policy 索引、TouchBoost 50ms 脉冲冷却、Idle Dive 300ms 深入与 1ms 快速退出编写单元测试。
4. **打包闭环验证**：执行 `cargo run --package xtask -- b` 打包，确保 Release 二进制剥离符号表（Strip），最终 Zip 包体积严格控制在 15MB 规范范围内。
