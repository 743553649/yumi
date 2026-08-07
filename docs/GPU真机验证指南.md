# GPU 控制真机验证指南

> 目的：确认 Rust 守护进程中的 GpuManager 能在骁龙 8 Elite（或其它 Adreno GPU）手机上正常工作——探测 kgsl sysfs、切换模式、息屏联动、保活抗覆盖。

---

## 📦 准备工作

### 1. 获取刷机包

**方式 A：GitHub Actions 自动构建**
1. 把当前代码 push 到 GitHub main 分支
2. 进入 Actions 页面 → `Build Yumi` workflow → 等它跑完
3. 在 run 的底部 Artifacts 区下载 `.zip` 刷机包

**方式 B：本地交叉编译（需要 Linux/macOS + NDK）**
```bash
# 在 Linux/macOS 上
export YUMI_SKIP_EBPF=1  # 如果没配 eBPF 工具链
cargo ndk --platform 26 -t arm64-v8a build -r
```

### 2. 刷入模块

```bash
# KernelSU / APatch
adb push yumi-xxx.zip /sdcard/
# 在 KernelSU 管理器中刷入，或：
adb shell su -c 'ksud module install /sdcard/yumi-xxx.zip'

# Magisk
adb push yumi-xxx.zip /sdcard/
adb shell su -c 'magisk --install-module /sdcard/yumi-xxx.zip'
```

### 3. 启用 GPU 控制

刷完后重启，编辑 `/data/adb/modules/yumi/config/config.yaml`：

```yaml
function:
  GPUControl: true   # 改成 true
```

然后重启 yumi 守护进程：
```bash
adb shell su -c 'killall yumi && sleep 1 && /data/adb/modules/yumi/core/bin/yumi'
```

或者重启手机让 `service.sh` 自动拉起。

---

## 🧪 测试项

### 测试 1：启动日志——确认 GPU 模块被加载

```bash
adb shell su -c 'cat /data/adb/modules/yumi/logs/daemon.log | grep "\\[GPU\\]"'
```

**预期输出（类似）：**
```
[GPU] Detected Adreno (830) | freqs=8 governors=3
[GPU] GPU manager initialized
[GPU] Mode switch: →balance latency=0ms freq=553000000Hz
```

**如果看到：**
- `[GPU] KGSL device path not found, GPU control unavailable` → ❌ 该设备可能没有 kgsl sysfs
- `[GPU] Insufficient GPU frequencies (0), GPU control unavailable` → ❌ 频率节点不可读

---

### 测试 2：sysfs 写入确认——看看 GPU 频率限制是否生效

切换 modesave 模式：

```bash
# 切换到省电模式
adb shell su -c 'echo "powersave" > /data/adb/modules/yumi/current_mode.txt'
# 等待几秒让调度器处理
sleep 3
# 查看 GPU 当前频率
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/gpuclk'
# 查看 governor
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/devfreq/governor'
# 查看 max_gpuclk 限制
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/max_gpuclk'
```

**省电模式预期：**
- `gpuclk` → 最低档（如 300000000 Hz ≈ 300 MHz）
- `governor` → `powersave`
- `max_gpuclk` → 最低档

切换到极速模式：
```bash
adb shell su -c 'echo "fast" > /data/adb/modules/yumi/current_mode.txt'
sleep 3
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/devfreq/governor'
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/max_gpuclk'
```

**极速模式预期：**
- `governor` → `msm-adreno-tz`
- `max_gpuclk` → 最高档（如 1100000000 Hz ≈ 1.1 GHz）

---

### 测试 3：息屏/亮屏联动——确认 GPU 在息屏时进入 doze

```bash
# 先设为 balance 模式
adb shell su -c 'echo "balance" > /data/adb/modules/yumi/current_mode.txt'

# 息屏（按电源键），等 5 秒
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/devfreq/governor'
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/gpuclk'
```

**息屏预期：**
- `governor` → `powersave`
- `gpuclk` → 最低档

```bash
# 亮屏，等 3 秒
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/devfreq/governor'
```

**亮屏预期：**
- `governor` → `simple_ondemand`（balance 模式）

---

### 测试 4：保活线程——确认第三方 app 覆盖后能被恢复

模拟第三方 app 覆盖：
```bash
# 当前在 balance 模式，governor 应为 simple_ondemand
adb shell su -c 'echo "performance" > /sys/class/kgsl/kgsl-3d0/devfreq/governor'
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/devfreq/governor'
# 应该看到：performance（被覆盖了）

# 等 5 秒（保活间隔）
sleep 6
adb shell su -c 'cat /sys/class/kgsl/kgsl-3d0/devfreq/governor'
```

**预期：** 5 秒后 governor 自动恢复为 `simple_ondemand`（balance 模式配置的 governor）

---

### 测试 5：WebUI 确认——GPU 状态卡片是否显示

1. 打开 KernelSU Manager → 模块 → yumi → WebUI
2. 看主页顶部状态卡片区

**预期：** 看到第三张卡片，显示 GPU 型号（如 "Adreno 830"）和当前频率

**如果没显示：**
- 确认 `GPUControl: true` 已设置
- 确认 yumi 守护进程在运行
- 打开 WebUI 日志页，搜 `[GPU]` 日志

---

### 测试 6：日志全览

```bash
adb shell su -c 'cat /data/adb/modules/yumi/logs/daemon.log | grep "\\[GPU\\]"'
```

**完整的正常日志链：**
```
[INFO] [GPU] Detected Adreno (830) | freqs=8 governors=3
[INFO] [GPU] GPU manager initialized
[INFO] [GPU] Mode switch: →balance latency=0ms freq=553000000Hz
[INFO] [GPU] Keepalive thread started (interval 5s)
[INFO] [GPU] Entering screen-off GPU power-saving mode    ← 息屏时
[INFO] [GPU] Mode switch: →doze latency=0ms freq=300000000Hz
[INFO] [GPU] Exiting screen-off GPU power-saving mode     ← 亮屏时
[INFO] [GPU] Mode switch: →performance latency=0ms freq=900000000Hz  ← 切性能模式
```

---

## ⚠️ 异常排查

| 现象 | 原因 | 解决 |
|---|---|---|
| 日志无 `[GPU]` | `GPUControl: false` | 在 `config.yaml` 中改为 `true` |
| `KGSL path not found` | 设备没有 kgsl sysfs（非高通/内核没开） | 不支持，保持禁用 |
| `Insufficient frequencies` | 频率节点不存在或格式不对 | 检查设备 `/sys/class/kgsl/kgsl-3d0/` |
| governor 没变 | 节点只读/selinux 限制 | `su` 后检查 `ls -l /sys/class/kgsl/kgsl-3d0/devfreq/governor` |
| WebUI 没 GPU 卡片 | 同上 / 守护进程没运行 | 先测日志确认 GPU 模块正常 |
| 切换模式后频率不变 | 温控限频覆盖了设置 | 正常行为，保活线程不做温控对抗 |

---

## ✅ 验收标准

- [ ] 启动日志看到 `[GPU] Detected` 和初始化完成
- [ ] 省电模式：governor 为 `powersave`，频率为最低档
- [ ] 极速模式：governor 为 `msm-adreno-tz`，频率放开
- [ ] 息屏：自动切到 `powersave` + 最低频率
- [ ] 亮屏：恢复为当前模式的 governor
- [ ] 保活：第三方覆盖后 5 秒内恢复
- [ ] WebUI 主页显示 GPU 型号 + 频率卡片
- [ ] 切回 `GPUControl: false` 后重启，无 GPU 日志（功能关闭正常）