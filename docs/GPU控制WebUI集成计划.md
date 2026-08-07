# GPU 控制集成 — 阶段 4（WebUI 版）

> 将 GPU 频率/状态显示集成到 WebUI 前端，数据通过 KernelSU Bridge 从 sysfs 直接读取。

---

## 任务拆解

| # | 任务 | 文件 | 说明 |
|:---:|:---|:---|:---|
| 1 | Bridge 层 - 添加 GPU 数据读取 | `webui/src/utils/bridge.ts` + `mock.ts` | 通过 Root Shell 读取 kgsl sysfs |
| 2 | Store 层 - 添加 GPU 状态管理 | `webui/src/stores/scheduler.ts` | 新增 gpu 状态字段 + 获取 action |
| 3 | UI 层 - 主页添加 GPU 状态卡片 | `webui/src/views/HomeView.vue` | 在状态卡片区增加 GPU 频率/型号显示 |
| 4 | i18n - 添加 GPU 翻译键 | `webui/src/i18n/locales/zh.ts` + `en.ts` | 中文/英文翻译 |

---

## 任务 1：Bridge 层

### `webui/src/utils/bridge.ts`

在 `RealBridge` 对象中新增方法：

```typescript
async getGpuState(): Promise<{ available: boolean; frequency: number; model: string }> {
  try {
    // 检查 kgsl 是否存在
    const testPath = '/sys/class/kgsl/kgsl-3d0/gpu_model';
    const { errno: testErrno } = await exec(`test -f "${testPath}"`);
    if (testErrno !== 0) return { available: false, frequency: 0, model: '' };

    // 读取 GPU 型号
    const { stdout: modelRaw } = await exec(`cat /sys/class/kgsl/kgsl-3d0/gpu_model 2>/dev/null || echo unknown`);
    const model = modelRaw.trim() || 'unknown';

    // 读取当前频率
    const { stdout: freqRaw } = await exec(`cat /sys/class/kgsl/kgsl-3d0/gpuclk 2>/dev/null || echo 0`);
    const frequency = parseInt(freqRaw.trim(), 10) || 0;

    return { available: true, frequency, model };
  } catch (e) {
    return { available: false, frequency: 0, model: '' };
  }
}
```

### `webui/src/utils/mock.ts`

在 `MockBridge` 中新增：

```typescript
async getGpuState(): Promise<{ available: boolean; frequency: number; model: string }> {
  await delay(200);
  return { available: true, frequency: 553000000, model: 'Adreno 830' };
}
```

---

## 任务 2：Store 层

### `webui/src/stores/scheduler.ts`

在 `state` 中新增字段：
```typescript
gpuState: { available: false, frequency: 0, model: '' }
```

在 `initData()` 中添加获取 GPU 状态：
```typescript
const gpuState = await Bridge.getGpuState();
this.gpuState = gpuState;
```

---

## 任务 3：UI 层

### `webui/src/views/HomeView.vue`

在状态卡片区（`.header-cards` 内）新增 GPU 状态卡片——放在 daemon 卡片和 mode 卡片之后：

```vue
<div class="status-card glass-card fade-in-up" style="animation-delay: 0.15s" v-if="store.gpuState.available">
  <div class="status-indicator" style="background: var(--accent-purple)"></div>
  <van-icon name="video-o" size="28" color="var(--accent-purple)" />
  <div class="info">
    <h2>{{ store.gpuState.model }}</h2>
    <p>{{ t('gpu_frequency', { freq: formatGpuFreq(store.gpuState.frequency) }) }}</p>
  </div>
</div>
```

添加频率格式化辅助函数：
```typescript
const formatGpuFreq = (hz: number): string => {
  if (hz >= 1_000_000_000) return (hz / 1_000_000_000).toFixed(2) + ' GHz';
  if (hz >= 1_000_000) return (hz / 1_000_000).toFixed(0) + ' MHz';
  if (hz >= 1_000) return (hz / 1_000).toFixed(0) + ' KHz';
  return hz + ' Hz';
};
```

---

## 任务 4：i18n

### `zh.ts`
```typescript
gpu_frequency: 'GPU 频率: {freq}',
gpu_unavailable: 'GPU 不可用',
```

### `en.ts`
```typescript
gpu_frequency: 'GPU Freq: {freq}',
gpu_unavailable: 'GPU Unavailable',
```

---

## CSS 新增（HomeView.vue style）

GPU 状态卡片与现有 `.status-card` 样式共用，只需在 `:root` 或已有变量基础上增加紫色强调色（或用现有 `--accent-orange`）。现有通用卡片样式已覆盖。

---

## 依赖关系

任务 1 → 任务 2 → 任务 3（Bridge 层先做，Store 依赖 Bridge，UI 依赖 Store）
任务 4 可独立并行

---

## 熔断节点

- `bridge.ts` / `mock.ts` / `scheduler.ts` / `HomeView.vue` / i18n 文件均属于纯 WebUI 前端，**不涉及 Rust 核心或 Android 原生代码**，无熔断风险。