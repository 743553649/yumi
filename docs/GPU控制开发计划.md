# GPU 控制集成开发计划

> 基于 `.trae/documents/gpu-control-plan.md` 方案，遵循 yumi 既有模块化架构（参考 CPUSet、IdleDive、TouchBoost 集成模式）分阶段实施。

---

## 阶段概览

| 阶段 | 内容 | 预估文件数 | 依赖关系 |
|:---:|:---|:---:|:---:|
| 1 | GpuManager 核心模块（Rust 模块 + 配置结构 + 兼容性探测） | 3 新增 | 无 |
| 2 | 配置文件（gpu.yaml）+ config.yaml 开关 + i18n | 3 新增/修改 | 阶段 1 |
| 3 | 集成到调度器（runner.rs 主循环 + 保活线程） | 2 修改 | 阶段 1、2 |
| 4 | WebUI 侧 GPU 状态显示 | 3 新增/修改 | 阶段 3 |
| 5 | 文档同步与编译验证 | 2 修改 | 全部完成 |

---

## 阶段 1：GpuManager 核心模块

**说明**：与 `cpuset_manager`、`idle_dive`、`touch_boost` 模式一致，创建独立 Rust 模块，包含主控制器、配置结构体和兼容性探测逻辑。

### 1.1 创建 `src/gpu_manager/mod.rs` — GpuManager 主控制器

- `GpuManager` 结构体（字段：`enabled`、`compat`、各 sysfs 写入器 `FastWriter`、熔断器 `WriteCircuitBreaker`、看门狗 `GpuWatchdog`、当前模式缓存）
- 核心接口：
  - `new(config: &GpuConfig)` — 根据配置创建实例
  - `init(&mut self)` — 启动时探测兼容性，应用 balance 模式
  - `apply_mode(&mut self, mode: &str)` — 模式切换主入口（含参数解析、钳位、写入、回滚）
  - `enter_doze(&mut self)` — 息屏进入 doze 模式
  - `exit_doze(&mut self, restore_mode: &str)` — 亮屏恢复指定模式
  - `release(&mut self)` — 退出时恢复默认（释放控制权）
- 内部辅助函数：
  - `resolve_mode_config(&self, mode: &str)` — 从 GpuConfig 解析并计算实际参数（0 值为自动档）
  - `clamp_max_gpuclk(&self, target: u32)` — 将目标频率钳位到可用频率表
  - `validate_governor(&self, requested: &str)` — 校验 governor 并降级
  - `write_max_gpuclk(&mut self, freq: u32)` — 写入 max_gpuclk（含重试）
  - `write_governor(&mut self, gov: &str)` — 写入 governor（含回读确认）
  - `write_force_no_nap(&mut self, val: u32)` — 写入 force_no_nap（含钳位）
- 频率自动计算逻辑（`max_gpuclk: 0` 时）：
  - powersave/doze → `min(frequencies)`
  - balance → `freqs[floor(len × 0.4)]`
  - performance → `freqs[floor(len × 0.85)]`
  - fast → `max(frequencies)`
- 采用 `#[derive(Debug)]` + `log::info/warn/error` 日志模式（与现有模块一致）

### 1.2 创建 `src/gpu_manager/config.rs` — 配置结构体

- `GpuConfig`（顶层：`enabled: bool`、`modes: GpuModeConfigs`、`keepalive_interval_s: u64`）
- `GpuModeConfigs`（包含 `powersave`/`balance`/`performance`/`fast`/`doze` 五个子配置 + `get(&self, mode: &str)` 方法）
- `GpuModeConfig`（字段：`max_gpuclk: u32`、`governor: String`、`force_no_nap: u32`）
- 全部使用 `#[derive(Debug, Deserialize)]` + `#[serde(default)]`，默认值策略：
  - `enabled: true`（由 function.GPUControl 总闸门控）
  - `governor: "msm-adreno-tz"`
  - `max_gpuclk: 0`（自动选择）
  - `force_no_nap: 0`
- **配置扁平化**（吸取 cpuset.yaml C1 教训）：gpu.yaml 顶层直接为 `enabled:`/`modes:` 键，无包装键

### 1.3 创建 `src/gpu_manager/compat.rs` — 兼容性探测

- `GpuCompatInfo` 结构体（字段：`available: bool`、`kgsl_path: PathBuf`、`frequencies: Vec<u32>`、`governors: Vec<String>`、`gpu_model: String`、`has_governor_control: bool`、`has_freq_control: bool`）
- `probe_compat() -> GpuCompatInfo` 函数：
  - 探测 kgsl sysfs 路径（主候选：`/sys/class/kgsl/kgsl-3d0/`）
  - 读取 `available_frequencies`，排序去重
  - 读取 `available_governors`
  - 读取 `gpu_model` 芯片型号
  - 写入 `Disabled` 兼容信息时的兜底
- `GpuCompatInfo::disabled()` 静态方法

### 1.4 创建 `src/gpu_manager/watchdog.rs` — GPU 健康监控与熔断器

- `GpuWatchdog` 结构体（检测频率锁死 + 自动恢复三阶段流程）
- `WriteCircuitBreaker` 结构体（连续 3 次写入失败 → 30s 熔断冷却）
- 检测与恢复流程（参考方案第 6 节，三阶段：释放限频 → 切换 governor → force_no_nap 脉冲唤醒）

### 1.5 在 `src/main.rs` 注册模块

- 增加 `pub mod gpu_manager;`（与其他模块并列，按字母序排在 `ebpf_monitor` 后）

### 测试要求（中量级）

- 单元测试：`GpuModeConfigs::get()` 各模式返回正确
- 单元测试：`clamp_gpuclk()` 边界情况（空列表、精确命中、向下取整、0 值处理）
- 单元测试：`validate_governor()` 正常/降级/完全不可用
- 单元测试：`WriteCircuitBreaker` 熔断/冷却/重置
- 单测方法：`#[cfg(test)] mod tests { ... }` inline 测试（与 idle_dive/touch_boost 模式一致）
- **不写集成测试**（依赖真实 sysfs）

---

## 阶段 2：配置文件与国际化

### 2.1 创建 `module/config/gpu.yaml`

```yaml
# ── GPU 控制配置 ──
# 注意：顶层扁平结构，无包装键（与 idle_dive.yaml / touch_boost.yaml 一致）
enabled: true

modes:
  powersave:
    max_gpuclk: 0        # 0 = 自动选择最低档
    governor: "powersave"
    force_no_nap: 0

  balance:
    max_gpuclk: 0        # 0 = 自动选择 40% 分位频率
    governor: "simple_ondemand"
    force_no_nap: 0

  performance:
    max_gpuclk: 0        # 0 = 自动选择 85% 分位频率
    governor: "msm-adreno-tz"
    force_no_nap: 0

  fast:
    max_gpuclk: 0        # 0 = 自动选择最高档
    governor: "msm-adreno-tz"
    force_no_nap: 1

  doze:
    max_gpuclk: 0        # 0 = 自动选择最低档
    governor: "powersave"
    force_no_nap: 0

keepalive_interval_s: 5
```

### 2.2 修改 `module/config/config.yaml`

- 在 `function:` 块中增加 `GPUControl: false`（默认关闭，由用户启用）
- 格式对齐：`GPUControl: true/false`

### 2.3 更新 `module/config/i18n/zh.ftl` + `en.ftl`

新增以下 i18n 键（中英文双语）：

| 键 | 中文 | 英文 |
|:---|:---|:---|
| `gpu-init` | [GPU] GPU 控制器初始化完成 | [GPU] GPU controller initialized |
| `gpu-init-failed` | [GPU] 初始化失败: { $error } | [GPU] Initialization failed: { $error } |
| `gpu-unavailable` | [GPU] kgsl sysfs 不可用，GPU 控制已禁用 | [GPU] kgsl sysfs unavailable, GPU control disabled |
| `gpu-insufficient-freqs` | [GPU] 可用频率不足 ({ $count })，GPU 控制已禁用 | [GPU] Insufficient frequencies ({ $count }), GPU control disabled |
| `gpu-mode-switch` | [GPU] 模式切换: →{ $mode } 延迟={ $ms }ms 频率={ $freq }Hz | [GPU] Mode switch: →{ $mode } latency={ $ms }ms freq={ $freq }Hz |
| `gpu-enter-doze` | [GPU] 进入息屏深度省电模式 | [GPU] Entering doze power-saving mode |
| `gpu-exit-doze` | [GPU] 退出息屏深度省电模式 | [GPU] Exiting doze power-saving mode |
| `gpu-release` | [GPU] 已释放控制权，恢复默认设置 | [GPU] Released control, restored to defaults |
| `gpu-write-failed` | [GPU] 写入 { $node } 失败: { $error } | [GPU] Write to { $node } failed: { $error } |
| `gpu-circuit-breaker` | [GPU] 写入熔断器触发，冷却 { $secs }s | [GPU] Write circuit breaker tripped, cooling { $secs }s |
| `gpu-watchdog-stalled` | [GPU] 看门狗检测到 GPU 频率卡死 | [GPU] Watchdog detected GPU frequency stall |
| `gpu-watchdog-recovered` | [GPU] 看门狗恢复成功 | [GPU] Watchdog recovered successfully |
| `gpu-watchdog-hung` | [GPU] GPU 无响应，放弃控制权 | [GPU] GPU unresponsive, relinquishing control |
| `gpu-keepalive-started` | [GPU] 保活线程已启动 (间隔 { $secs }s) | [GPU] Keepalive thread started (interval { $secs }s) |

### 2.4 配置热重载支持

- 在 `runner.rs` config_watcher 中增加 `gpu.yaml` 变更检测分支（与 cpuset/idle_dive/touch_boost 模式一致）
- `GpuConfig` 通过 `Arc<RwLock<>>` 共享引用实现热重载

---

## 阶段 3：集成到调度器主循环

### 3.1 修改 `src/scheduler/runner.rs`

**初始化部分**（与 idle_dive、touch_boost 并列）：

```rust
// GPU 共享配置（支持热重载）
let gpu_path = config_dir.join("gpu.yaml");
let shared_gpu_config = Arc::new(std::sync::RwLock::new(
    crate::utils::read_config::<crate::gpu_manager::GpuConfig, _>(&gpu_path).unwrap_or_default()
));
let gpu_config_watcher = shared_gpu_config.clone();
```

**config_watcher 线程新增分支**：

```rust
// GPU 配置变更
if changed_file == "gpu.yaml" || changed_file.is_empty() {
    let new_gpu = crate::utils::read_config::<crate::gpu_manager::GpuConfig, _>(&gpu_path).unwrap_or_default();
    *gpu_config_watcher.write().unwrap_or_else(|e| e.into_inner()) = new_gpu;
    log::info!("{}", t("gpu-config-reloaded"));
}
```

**IPC 线程内集成**（与 cpuset_manager、idle_dive 并列初始化）：

```rust
// GPU 控制器
let gpu_config = shared_gpu_config.clone();
let mut gpu_manager = crate::gpu_manager::GpuManager::new(&gpu_config.read().unwrap_or_else(|e| e.into_inner()));
if let Err(e) = gpu_manager.init() {
    log::error!("{}", t_with_args("gpu-init-failed", &fluent_args!("error" => e.to_string())));
}
```

**`ScreenStateChange` 事件处理**：

```rust
DaemonEvent::ScreenStateChange(screen_on) => {
    // ... 现有代码 ...
    if !is_screen_on {
        // ... 现有代码 ...
        gpu_manager.enter_doze(); // 息屏 GPU 深度省电
    } else {
        // ... 现有代码 ...
        let current_mode = mode_clone.lock().unwrap_or_else(|e| e.into_inner()).clone();
        gpu_manager.exit_doze(&current_mode); // 亮屏恢复
    }
}
```

**`ModeChange` 事件处理**：

```rust
DaemonEvent::ModeChange { mode, .. } => {
    // ... 现有代码 ... 在模式切换时：
    if is_screen_on {
        gpu_manager.apply_mode(&mode).ok();
    }
    // 息屏时不切换（GPU 保持在 doze 模式）
}
```

**保活线程**（与现有模块分离，独立线程）：

```rust
// GPU 保活线程：定期重新写入当前模式配置，防第三方覆盖
let gpu_keepalive_config = shared_gpu_config.clone();
let gpu_keepalive_mode = mode_clone.clone();
thread::Builder::new()
    .name("gpu_keepalive".to_string())
    .spawn(move || {
        let interval = Duration::from_secs(
            gpu_keepalive_config.read().unwrap_or_else(|e| e.into_inner()).keepalive_interval_s
        );
        loop {
            thread::sleep(interval);
            let mode = gpu_keepalive_mode.lock().unwrap_or_else(|e| e.into_inner()).clone();
            // 保活逻辑通过 GpuManager 的公开方法或直接读写共享状态完成
            // 保活仅重写当前模式已知的 GPU 节点
        }
    })?;
```

**退出时释放**（runner.rs 目前没有显式 cleanup，保持对齐，暂不添加 release 调用）

### 3.2 修改 `src/main.rs`

- 无需额外修改（阶段 1.5 已注册模块）

---

## 阶段 4：WebUI 侧 GPU 状态显示

**说明**：放弃 Android App 侧修改（`SystemDashboardMonitor.java` 等 Java/(已移除) 文件），改为在 WebUI 前端增加 GPU 状态卡片。WebUI 通过 IPC 协议直接从 daemon 读取 GPU 状态，无需修改任何 Android App 代码。

### 4.1 Rust IPC 新增 `get_gpu_state` 协议

- 在 `src/(已移除).rs` 的 `process_command` 函数中新增命令分支：

```rust
"get_gpu_state" => {
    // 读取 GPU 当前频率 + 模型 + 利用率
    let gpuclk_path = "/sys/class/kgsl/kgsl-3d0/gpuclk";
    let model_path = "/sys/class/kgsl/kgsl-3d0/gpu_model";
    let gpuclk = std::fs::read_to_string(gpuclk_path).unwrap_or_default().trim().to_string();
    let model = std::fs::read_to_string(model_path).unwrap_or_default().trim().to_string();
    format!("gpuclk={}\nmodel={}\n---END_GPU_STATE---\n", gpuclk, model)
}
```

- 遵循既有 IPC 协议风格（文本行协议，`---END_GPU_STATE---` 终止符与 `get_log` 的 `---END_LOG---` 一致）
- **熔断说明**：`(已移除).rs` 是跨模块公共文件，实施前需确认

### 4.2 WebUI bridge.ts 新增 `getGpuState` 方法

- 在 `webui/src/utils/bridge.ts` 的 `RealBridge` 对象中增加：

```typescript
async getGpuState(): Promise<{ gpuclk: string; model: string }> {
  try {
    const raw = await this.readFile('/sys/class/kgsl/kgsl-3d0/gpuclk');
    const model = await this.readFile('/sys/class/kgsl/kgsl-3d0/gpu_model');
    return {
      gpuclk: raw.trim(),
      model: model.trim() || 'unknown'
    };
  } catch (e) {
    return { gpuclk: 'N/A', model: 'N/A' };
  }
},
```

- 对应 `MockBridge` 中增加 mock 实现：

```typescript
async getGpuState(): Promise<{ gpuclk: string; model: string }> {
  await delay(200);
  return { gpuclk: '540000000', model: 'Adreno 830' };
},
```

### 4.3 WebUI HomeView.vue 增加 GPU 状态卡片

- 在 `webui/src/views/HomeView.vue` 中，状态卡片区域增加第三个 GPU 卡片：

```vue
<div class="status-card glass-card fade-in-up" style="animation-delay: 0.15s">
  <div class="status-indicator" style="background: var(--accent-purple)"></div>
  <van-icon name="video-o" size="28" color="var(--accent-purple)" />
  <div class="info">
    <h2>{{ gpuModel }}</h2>
    <p>{{ $t('gpu_freq', { freq: gpuClk }) }}</p>
  </div>
</div>
```

- 新增响应式数据与采集逻辑（`onMounted` 中调用 `store.getGpuState()`，周期性刷新）
- 初期显示 GPU 型号 + 当前频率，利用率待后续完善

### 4.4 i18n 新增键

在 `webui/src/i18n/locales/zh.ts` 和 `en.ts` 中增加：

```typescript
// zh.ts
gpu_freq: 'GPU 频率: {freq} Hz'

// en.ts
gpu_freq: 'GPU Freq: {freq} Hz'
```

### 4.5 WebUI store.ts 新增 GPU 状态管理

- 在 `webui/src/stores/scheduler.ts` 中增加：

```typescript
state: () => ({
  // ... 现有字段
  gpuModel: 'N/A',
  gpuClk: 'N/A',
}),
actions: {
  // ... 现有 actions
  async getGpuState() {
    const state = await Bridge.getGpuState();
    this.gpuModel = state.model;
    this.gpuClk = state.gpuclk;
  },
}
```

**设计考量**：WebUI 方案相比 Android App 方案的优势：
1. **零编译依赖** — 不需要 Android SDK/Gradle 工具链
2. **热更新** — WebUI 修改后打包即生效，无需重新安装 App
3. **与既有架构一致** — WebUI 已经是 yumi 的主管理界面（HomeView 已有两块状态卡片），增加 GPU 卡片仅扩展
4. **熔断风险低** — `HomeView.vue` 和 `bridge.ts` 是 WebUI 专用文件，不影响 Android App 公共代码

---

## 阶段 5：文档同步与编译验证

### 5.1 更新 `docs/工作日志.md`

- 新增"阶段 N：GPU 控制集成"章节（与现有阶段格式对齐，含新增文件清单、修改文件清单、验证状态）

### 5.2 更新 README（如适用）

- 在功能列表中增加 GPU 控制说明
- 在配置说明中增加 `gpu.yaml` 和 `function.GPUControl` 开关

### 5.3 编译验证

```powershell
$env:YUMI_SKIP_EBPF=1; cargo check --target aarch64-linux-android
$env:YUMI_SKIP_EBPF=1; cargo clippy --target aarch64-linux-android
cargo fmt --check
```

---

## 风险与熔断点

| 熔断节点 | 触发条件 | 动作 |
|:---|:---|:---|
| 阶段 3 集成到 runner.rs | 需要修改 IPC 线程核心事件循环 | 仅按规范增删，不改动现有逻辑流程；涉及跨模块核心文件时提前向用户确认 |
| 阶段 4 WebUI 侧 | 涉及 `bridge.ts` 新增 Root Shell 直接读取 GPU sysfs 节点 | 仅对 WebUI 特定文件修改，不影响 Rust/Android App。如需改 `(已移除).rs` 需提前确认 |
| 任何阶段连续 3 次编译失败 | `cargo check` / `clippy` 报错 | 整理错误日志 + 已尝试修复路径，向用户求助 |
| gpu.yaml 配置键冲突 | 与 config.yaml 或其他配置格式不一致 | 熔断，检查现有配置模式对齐后再继续 |

---

## 交付检查清单

- [ ] `src/gpu_manager/mod.rs` — GpuManager 主控制器
- [ ] `src/gpu_manager/config.rs` — GpuConfig 配置结构体
- [ ] `src/gpu_manager/compat.rs` — 兼容性探测
- [ ] `src/gpu_manager/watchdog.rs` — GPU 健康监控与熔断器
- [ ] `module/config/gpu.yaml` — GPU 控制配置文件
- [ ] `module/config/config.yaml` — 增加 `function.GPUControl` 开关
- [ ] `module/config/i18n/zh.ftl` + `en.ftl` — 新增 ~15 个 GPU 翻译键
- [ ] `src/main.rs` — 注册 `gpu_manager` 模块
- [ ] `src/scheduler/runner.rs` — 集成 GpuManager（初始化、模式切换、息屏/亮屏、保活线程 + 热重载）
- [ ] `webui/src/utils/bridge.ts` — 新增 `getGpuState()` 方法 (RealBridge + MockBridge)
- [ ] `webui/src/stores/scheduler.ts` — 新增 gpuModel / gpuClk 状态 + getGpuState action
- [ ] `webui/src/views/HomeView.vue` — 新增 GPU 状态卡片（频率 + 型号）
- [ ] `webui/src/i18n/locales/zh.ts` + `en.ts` — 新增 `gpu_freq` 翻译键
- [ ] 可选：`src/(已移除).rs` — 新增 `get_gpu_state` IPC 协议命令（若 WebUI 需要 daemon 代理读取）
- [ ] Rust 编译通过（`cargo check --target aarch64-linux-android`）
- [ ] Clippy 无新警告
- [ ] 代码格式化通过（`cargo fmt --check`）
- [ ] 文档：工作日志同步