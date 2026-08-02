# TouchBoost 实现方案

> 基于骁龙 8 Elite 架构，为 yumi 调度器添加触摸提频功能
> 适用版本：yumi v2.0.1

---

## 1. 功能概述

### 1.1 什么是 TouchBoost

TouchBoost 是 Android 系统中常见的性能优化功能：当用户触摸屏幕时，临时提升 CPU 频率，确保触摸操作（点击、滑动、滚动）的响应速度。松手后，频率逐渐恢复到正常调度状态。

### 1.2 为什么需要 TouchBoost

| 场景 | 没有 TouchBoost | 有 TouchBoost |
|:---|:---|:---|
| 点击 App 图标 | CPU 从低频启动，响应延迟 50-100ms | 立即提频，响应延迟 < 20ms |
| 滑动列表 | 前几帧可能掉帧 | 流畅启动，无掉帧 |
| 游戏中移动视角 | 视角响应略慢 | 即时响应 |

### 1.3 与现有机制的关系

```
用户触摸屏幕
    ↓
TouchBoost (新增) → 立即提频，保证响应速度
    ↓
FAS (已有) → 帧感知调度，根据实际帧率调整频率
    ↓
CLG (已有) → 负载调速器，根据 CPU 利用率调整频率
```

TouchBoost 是**第一道响应**，在 FAS/CLG 还没来得及反应时就提频；FAS/CLG 随后接管，根据实际负载精细调频。

---

## 2. 硬件架构分析

### 2.1 骁龙 8 Elite CPU 配置

```
┌─────────────────────────────────────────────┐
│              骁龙 8 Elite                    │
├─────────────────────────────────────────────┤
│  Policy 0 (Prime Cluster)                   │
│  ├── CPU 0, CPU 1  (超大核, Oryon)          │
│  └── 最高主频: 4.32 GHz                     │
├─────────────────────────────────────────────┤
│  Policy 2 (Performance Cluster)             │
│  ├── CPU 2-7  (大核, Oryon)                 │
│  └── 最高主频: 3.53 GHz                     │
└─────────────────────────────────────────────┘
```

### 2.2 触摸事件路径

```
触摸屏硬件
    ↓
Linux Input 子系统 (/dev/input/event*)
    ↓
Android InputDispatcher
    ↓
App 的 onTouchEvent()
```

TouchBoost 在 **Linux Input 子系统** 层监听，比 Android 框架层更快。

---

## 3. 技术方案

### 3.1 监听方式选择

| 方案 | 优点 | 缺点 |
|:---|:---|:---|
| **方案 A: epoll + /dev/input/event*** | 最快，内核直达；无需 root 额外权限 | 需要解析 evdev 协议 |
| 方案 B: inotify + /sys/class/input | 简单 | 延迟高，不适合实时性要求 |
| 方案 C: Android InputChannel | Android 原生 | 需要 Java/JNI，与 Rust 架构不符 |

**选择方案 A**：使用 `epoll` 监听 `/dev/input/event*` 设备，直接读取内核触摸事件。

### 3.2 事件处理流程

```
┌─────────────────────────────────────────────────────────────┐
│                    TouchBoost 处理流程                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ 监听线程     │    │ Boost 控制器 │    │ 频率写入器   │  │
│  │ (epoll)      │───→│ (TouchBoost) │───→│ (sysfs)      │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│         │                    │                    │         │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ 解析 evdev   │    │ 状态机管理   │    │ 恢复原频率   │  │
│  │ ABS_MT_*     │    │ IDLE→TOUCH   │  │ (衰减恢复)   │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 状态机设计

```
                    触摸按下 (ABS_MT_TRACKING_ID != -1)
    ┌──────────────────────────────────────────────────────┐
    │                                                      │
    │                                                      ▼
┌───┴────┐      触摸按下       ┌─────────────┐      超时或松手     ┌──────────────┐
│  IDLE  │ ──────────────────→ │   TOUCH     │ ──────────────────→ │  RECOVERING  │
│(空闲)  │                     │ (触摸中)    │                     │ (恢复中)     │
└────────┘ ←────────────────── └─────────────┘ ←────────────────── └──────────────┘
    ▲          恢复完成                          新的触摸按下
    │          (频率恢复)                         (重新 boost)
    └───────────────────────────────────────────────────────────────────────────────┘
```

### 3.4 频率提升策略

#### 策略：分集群 boost

```rust
// 触摸时：所有集群提升到目标频率
Prime Cluster (Policy 0):  → boost_freq (如 2.5 GHz)
Performance Cluster (Policy 2):  → boost_freq (如 2.0 GHz)

// 松手后：逐步恢复到 FAS/CLG 控制的频率
Prime Cluster:  → 当前 FAS/CLG 频率 (衰减过渡)
Performance Cluster:  → 当前 FAS/CLG 频率 (衰减过渡)
```

#### 为什么分集群？

- **超大核**：负责主线程、UI 渲染，需要高频保证响应
- **大核**：负责后台任务、辅助线程，适度提频即可

---

## 4. 实现细节

### 4.1 新增文件结构

```
src/
├── touch_boost/
│   ├── mod.rs              # 模块入口
│   ├── listener.rs         # 触摸事件监听器 (epoll + evdev)
│   ├── controller.rs       # TouchBoost 控制器 (状态机)
│   └── config.rs           # TouchBoost 配置
```

### 4.2 核心数据结构

```rust
// src/touch_boost/config.rs
/// TouchBoost 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchBoostConfig {
    /// 是否启用 TouchBoost
    pub enabled: bool,
    /// 各集群的 boost 目标频率 (kHz)，按 policy id 索引
    /// 例如: [2500000, 2000000] 表示 Policy 0 boost 到 2.5GHz，Policy 2 boost 到 2.0GHz
    pub boost_freqs: Vec<u32>,
    /// 松手后恢复延迟 (ms)，防止快速点击时频繁切换
    pub release_delay_ms: u64,
    /// 恢复阶段的衰减步长 (每次 tick 降低的比例)
    pub recover_decay: f32,
    /// 最小 boost 持续时间 (ms)，防止误触
    pub min_boost_duration_ms: u64,
    /// 触摸设备路径，空则自动检测
    pub input_device: String,
}

impl Default for TouchBoostConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            boost_freqs: vec![2500000, 2000000],  // 骁龙 8 Elite 默认
            release_delay_ms: 100,
            recover_decay: 0.15,
            min_boost_duration_ms: 50,
            input_device: String::new(),  // 自动检测
        }
    }
}
```

```rust
// src/touch_boost/controller.rs
/// TouchBoost 状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum BoostState {
    /// 空闲，无触摸
    Idle,
    /// 触摸中，已 boost
    Touching,
    /// 松手后恢复中
    Recovering,
}

/// TouchBoost 控制器
pub struct TouchBoostController {
    config: TouchBoostConfig,
    state: BoostState,
    /// 各集群的频率写入器 (scaling_min_freq)
    min_freq_writers: Vec<FastWriter>,
    /// 各集群的原始 min_freq (恢复时使用)
    original_min_freqs: Vec<u32>,
    /// 各集群当前的 boost 频率
    current_boost_freqs: Vec<u32>,
    /// 触摸开始时间
    touch_start: Instant,
    /// 松手时间
    release_time: Instant,
    /// 是否已初始化
    initialized: bool,
}
```

### 4.3 触摸事件监听器

```rust
// src/touch_boost/listener.rs
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use nix::sys::epoll::{epoll_create1, epoll_ctl, EpollFlags, EpollEvent};
use nix::sys::eventfd::{eventfd, EfdFlags};

/// Linux input_event 结构
#[repr(C)]
struct InputEvent {
    tv_sec: i64,
    tv_usec: i64,
    ev_type: u16,
    ev_code: u16,
    ev_value: i32,
}

/// ABS_MT 事件类型
const EV_ABS: u16 = 3;
const ABS_MT_TRACKING_ID: u16 = 57;

/// 触摸事件监听器
pub struct TouchListener {
    epoll_fd: RawFd,
    device_fds: Vec<RawFd>,
}

impl TouchListener {
    /// 创建新的监听器，自动检测触摸设备
    pub fn new(config: &TouchBoostConfig) -> Result<Self> {
        let epoll_fd = epoll_create1(EpollFlags::EPOLL_CLOEXEC)?;
        let mut device_fds = Vec::new();

        // 自动检测触摸设备或使用配置的设备
        let devices = if config.input_device.is_empty() {
            Self::detect_touch_devices()?
        } else {
            vec![config.input_device.clone()]
        };

        for device_path in &devices {
            let fd = Self::open_device(device_path)?;
            device_fds.push(fd);
            
            let mut event = EpollEvent::new(EpollFlags::EPOLLIN, fd as u64);
            epoll_ctl(epoll_fd, EpollOp::EpollCtlAdd, fd, Some(&mut event))?;
        }

        Ok(Self { epoll_fd, device_fds })
    }

    /// 检测系统中的触摸设备
    fn detect_touch_devices() -> Result<Vec<String>> {
        let mut devices = Vec::new();
        // 扫描 /sys/class/input/ 查找触摸设备
        // 触摸设备通常有 ABS_MT_TRACKING_ID 属性
        // ...
        Ok(devices)
    }

    /// 等待触摸事件，返回是否触摸中
    pub fn wait_for_touch(&self, timeout_ms: i32) -> Result<bool> {
        let mut events = [EpollEvent::empty(); 1];
        let n = epoll_wait(self.epoll_fd, &mut events, timeout_ms)?;
        
        if n == 0 {
            return Ok(false);  // 超时，无事件
        }

        // 读取 input 事件
        for event in &events[..n] {
            let fd = event.data() as RawFd;
            let input_event = self.read_input_event(fd)?;
            
            // ABS_MT_TRACKING_ID != -1 表示有触摸点
            if input_event.ev_type == EV_ABS 
                && input_event.ev_code == ABS_MT_TRACKING_ID 
                && input_event.ev_value != -1 
            {
                return Ok(true);  // 触摸按下
            }
        }

        Ok(false)
    }

    fn read_input_event(&self, fd: RawFd) -> Result<InputEvent> {
        let mut event = InputEvent {
            tv_sec: 0, tv_usec: 0, ev_type: 0, ev_code: 0, ev_value: 0
        };
        let size = std::mem::size_of::<InputEvent>();
        let buf = unsafe {
            std::slice::from_raw_parts_mut(
                &mut event as *mut InputEvent as *mut u8, size
            )
        };
        nix::unistd::read(fd, buf)?;
        Ok(event)
    }
}
```

### 4.4 频率控制逻辑

```rust
// src/touch_boost/controller.rs

impl TouchBoostController {
    /// 初始化控制器，获取各集群的频率写入器
    pub fn init(&mut self, policies: &[CpuPolicy]) -> Result<()> {
        for policy in policies {
            let min_path = format!(
                "/sys/devices/system/cpu/cpufreq/policy{}/scaling_min_freq",
                policy.id
            );
            let writer = FastWriter::new(&min_path)?;
            
            // 读取当前 min_freq 作为恢复目标
            let current_min = Self::read_current_min_freq(policy.id)?;
            
            self.min_freq_writers.push(writer);
            self.original_min_freqs.push(current_min);
            self.current_boost_freqs.push(0);
        }
        self.initialized = true;
        Ok(())
    }

    /// 处理触摸事件
    pub fn on_touch_event(&mut self, touching: bool) {
        if !self.config.enabled || !self.initialized {
            return;
        }

        let now = Instant::now();

        match (self.state, touching) {
            // IDLE → TOUCHING: 触摸按下
            (BoostState::Idle, true) => {
                self.state = BoostState::Touching;
                self.touch_start = now;
                self.apply_boost();
                log::debug!("TouchBoost: 触摸开始，应用 boost");
            }
            // TOUCHING → RECOVERING: 松手
            (BoostState::Touching, false) => {
                // 检查最小 boost 持续时间
                if now.duration_since(self.touch_start).as_millis() 
                    >= self.config.min_boost_duration_ms as u128 
                {
                    self.state = BoostState::Recovering;
                    self.release_time = now;
                    log::debug!("TouchBoost: 松手，开始恢复");
                }
            }
            // RECOVERING → IDLE: 恢复完成
            (BoostState::Recovering, false) => {
                if self.should_recover(now) {
                    self.recover();
                    self.state = BoostState::Idle;
                    log::debug!("TouchBoost: 恢复完成");
                }
            }
            // RECOVERING → TOUCHING: 恢复中再次触摸
            (BoostState::Recovering, true) => {
                self.state = BoostState::Touching;
                self.touch_start = now;
                self.apply_boost();
                log::debug!("TouchBoost: 恢复中再次触摸，重新 boost");
            }
            _ => {}
        }
    }

    /// 应用 boost 频率
    fn apply_boost(&mut self) {
        for (i, writer) in self.min_freq_writers.iter().enumerate() {
            if let Some(&boost_freq) = self.config.boost_freqs.get(i) {
                if boost_freq > 0 {
                    writer.write_value_force(boost_freq);
                    self.current_boost_freqs[i] = boost_freq;
                }
            }
        }
    }

    /// 恢复原始频率
    fn recover(&mut self) {
        for (i, writer) in self.min_freq_writers.iter().enumerate() {
            let target_freq = self.original_min_freqs[i];
            writer.write_value_force(target_freq);
            self.current_boost_freqs[i] = 0;
        }
    }

    /// 判断是否应该恢复
    fn should_recover(&self, now: Instant) -> bool {
        let elapsed = now.duration_since(self.release_time).as_millis() as u64;
        elapsed >= self.config.release_delay_ms
    }

    /// 定时 tick，用于恢复阶段的衰减
    pub fn tick(&mut self) {
        if self.state == BoostState::Recovering {
            let now = Instant::now();
            if self.should_recover(now) {
                // 衰减 boost 频率
                for (i, writer) in self.min_freq_writers.iter().enumerate() {
                    let current = self.current_boost_freqs[i];
                    let target = self.original_min_freqs[i];
                    
                    if current > target {
                        let step = ((current - target) as f32 
                            * self.config.recover_decay) as u32;
                        let new_freq = (current - step).max(target);
                        
                        writer.write_value_force(new_freq);
                        self.current_boost_freqs[i] = new_freq;
                    }
                }
            }
        }
    }
}
```

### 4.5 与主调度器集成

```rust
// src/scheduler/mod.rs (新增)

/// 启动 TouchBoost 监听线程
fn start_touch_boost_thread(
    config: TouchBoostConfig,
    policies: Vec<CpuPolicy>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let listener = match TouchListener::new(&config) {
            Ok(l) => l,
            Err(e) => {
                log::error!("TouchBoost 监听器初始化失败: {}", e);
                return;
            }
        };
        
        let mut controller = TouchBoostController::new(config);
        if let Err(e) = controller.init(&policies) {
            log::error!("TouchBoost 控制器初始化失败: {}", e);
            return;
        }

        log::info!("TouchBoost 线程启动");

        loop {
            // 等待触摸事件，超时 16ms (约 60fps 的 tick 率)
            match listener.wait_for_touch(16) {
                Ok(touching) => {
                    controller.on_touch_event(touching);
                }
                Err(e) => {
                    log::warn!("TouchBoost 事件读取错误: {}", e);
                }
            }
            
            // 定时 tick，处理恢复衰减
            controller.tick();
        }
    })
}
```

---

## 5. 配置参数说明

### 5.1 配置文件位置

```
module/config/touch_boost.yaml
```

### 5.2 配置示例

```yaml
# TouchBoost 配置
touch_boost:
  enabled: true
  
  # 各集群 boost 目标频率 (kHz)
  # 骁龙 8 Elite: Policy 0 (超大核), Policy 2 (大核)
  # 按 policy id 顺序填写，空缺填 0
  boost_freqs:
    - 2500000   # Policy 0: 超大核 boost 到 2.5GHz
    - 0         # Policy 1: 不存在，填 0
    - 2000000   # Policy 2: 大核 boost 到 2.0GHz
  
  # 松手后恢复延迟 (ms)
  # 防止快速点击时频繁切换
  release_delay_ms: 100
  
  # 恢复阶段的衰减步长 (0.0 - 1.0)
  # 每次 tick 降低当前 boost 频率的此比例
  recover_decay: 0.15
  
  # 最小 boost 持续时间 (ms)
  # 防止误触导致的短暂 boost
  min_boost_duration_ms: 50
  
  # 触摸设备路径，留空则自动检测
  input_device: ""
```

### 5.3 参数调优建议

| 参数 | 推荐值 | 说明 |
|:---|:---|:---|
| `boost_freqs` | 根据设备调整 | 查看 `scaling_available_frequencies` 选择合适的频率 |
| `release_delay_ms` | 50-200 | 值越大，松手后 boost 持续越久 |
| `recover_decay` | 0.1-0.3 | 值越大，恢复越快；值越小，过渡越平滑 |
| `min_boost_duration_ms` | 30-100 | 值越大，防误触效果越好 |

---

## 6. 测试方案

### 6.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boost_state_machine() {
        let config = TouchBoostConfig::default();
        let mut controller = TouchBoostController::new(config);
        
        // 初始状态: Idle
        assert_eq!(controller.state, BoostState::Idle);
        
        // 触摸按下 → Touching
        controller.on_touch_event(true);
        assert_eq!(controller.state, BoostState::Touching);
        
        // 松手 → Recovering
        controller.on_touch_event(false);
        assert_eq!(controller.state, BoostState::Recovering);
    }

    #[test]
    fn test_boost_recover() {
        let config = TouchBoostConfig {
            release_delay_ms: 0,  // 立即恢复
            ..Default::default()
        };
        let mut controller = TouchBoostController::new(config);
        
        // 模拟 boost
        controller.on_touch_event(true);
        assert!(controller.current_boost_freqs.iter().any(|&f| f > 0));
        
        // 松手并 tick
        controller.on_touch_event(false);
        controller.tick();
        
        // 频率应该恢复
        assert!(controller.current_boost_freqs.iter().all(|&f| f == 0));
    }
}
```

### 6.2 集成测试

```bash
# 1. 编译
cargo build

# 2. 部署到设备
adb push target/aarch64-linux-android/debug/yumi /data/local/tmp/

# 3. 运行并观察日志
adb shell "su -c /data/local/tmp/yumi" 2>&1 | grep -i touch

# 4. 触摸屏幕，观察日志输出
# 预期看到:
# TouchBoost: 触摸开始，应用 boost
# TouchBoost: 松手，开始恢复
# TouchBoost: 恢复完成

# 5. 监控 CPU 频率变化
adb shell "while true; do cat /sys/devices/system/cpu/cpufreq/policy0/scaling_cur_freq; sleep 0.1; done"
```

### 6.3 性能测试

| 测试场景 | 测试方法 | 预期结果 |
|:---|:---|:---|
| 触摸响应延迟 | 使用高速摄像机记录触摸到屏幕响应的时间 | < 20ms |
| 功耗影响 | 连续触摸操作 30 分钟，对比功耗 | 增加 < 3% |
| 频率稳定性 | 松手后观察频率恢复曲线 | 平滑衰减，无跳变 |
| 并发安全 | 快速连续点击 | 无 panic，频率正常切换 |

---

## 7. 风险评估与回滚

### 7.1 风险点

| 风险 | 影响 | 缓解措施 |
|:---|:---|:---|
| 触摸设备检测失败 | TouchBoost 不工作 | 提供手动配置 `input_device` 选项 |
| 频率写入失败 | 无 boost 效果 | 检查 sysfs 权限，记录错误日志 |
| 与 FAS/CLG 冲突 | 频率跳变 | TouchBoost 只修改 `min_freq`，不修改 `max_freq` |
| 功耗增加 | 续航缩短 | 可通过配置关闭或调整 boost 频率 |

### 7.2 回滚方案

1. **配置关闭**：在 `touch_boost.yaml` 中设置 `enabled: false`
2. **代码回滚**：删除 `src/touch_boost/` 目录和相关集成代码
3. **紧急关闭**：运行时通过 IPC 发送关闭命令

---

## 8. 实施步骤

### 阶段 1：基础框架 (1-2 天)
- [ ] 创建 `src/touch_boost/` 模块结构
- [ ] 实现 `TouchBoostConfig` 配置结构
- [ ] 实现 `TouchListener` 触摸事件监听器
- [ ] 实现 `TouchBoostController` 基本状态机

### 阶段 2：频率控制 (1 天)
- [ ] 实现 `apply_boost()` 频率提升
- [ ] 实现 `recover()` 频率恢复
- [ ] 实现衰减恢复逻辑

### 阶段 3：集成与测试 (1 天)
- [ ] 集成到主调度器线程
- [ ] 添加配置文件
- [ ] 编写单元测试
- [ ] 设备实测

### 阶段 4：优化与文档 (1 天)
- [ ] 根据实测结果调整参数
- [ ] 更新工作日志
- [ ] 完善文档

---

# CPUSet 管理实现方案

> 动态调整进程的 CPU 核心绑定，优化大小核调度

---

## 1. 功能概述

### 1.1 什么是 CPUSet

CPUSet 是 Linux 内核的 cpuset 子系统，通过 `/dev/cpuset/` 或 `/sys/fs/cgroup/cpuset/` 控制进程可以使用哪些 CPU 核心。Android 系统使用 cpuset 来管理前台/后台进程的核心分配。

### 1.2 为什么需要 CPUSet 管理

| 场景 | 系统默认行为 | yumi 优化后 |
|:---|:---|:---|
| 前台游戏 | 可能分配到小核 | 强制绑定大核，保证帧率 |
| 后台同步 | 可能占用大核 | 限制到小核，省电 |
| 息屏待机 | 后台进程分散运行 | 集中到效率核，深度省电 |

### 1.3 现有 Android CPUSet 结构

```
/dev/cpuset/
├── top-app/          # 当前前台应用 (最高优先级)
├── foreground/       # 前台服务
├── background/       # 后台应用
├── system-background/ # 系统后台服务
├── restricted/       # 受限进程
└── root/             # 根组 (默认)
```

---

## 2. 硬件架构适配

### 2.1 骁龙 8 Elite 核心映射

```
┌─────────────────────────────────────────────────────┐
│                 骁龙 8 Elite                         │
├─────────────────────────────────────────────────────┤
│  Policy 0: CPU 0-1  (Prime 超大核, 4.32GHz)         │
│  ├── 高性能，适合游戏主线程                          │
│  └── 功耗较高                                       │
├─────────────────────────────────────────────────────┤
│  Policy 2: CPU 2-7  (Performance 大核, 3.53GHz)     │
│  ├── 中等性能，适合日常使用                          │
│  └── 功耗适中                                       │
└─────────────────────────────────────────────────────┘

CPUSet 分配策略:
┌─────────────────────────────────────────────────────┐
│  游戏模式:                                           │
│  ├── top-app: CPU 0-7 (全部核心)                    │
│  ├── background: CPU 4-7 (大核部分)                 │
│  └── system-background: CPU 6-7 (大核末端)          │
├─────────────────────────────────────────────────────┤
│  日常模式:                                           │
│  ├── top-app: CPU 0-7 (全部核心)                    │
│  ├── background: CPU 2-5 (大核中部)                 │
│  └── system-background: CPU 6-7 (大核末端)          │
├─────────────────────────────────────────────────────┤
│  省电模式:                                           │
│  ├── top-app: CPU 0-3 (超大核+部分大核)             │
│  ├── background: CPU 4-7 (大核)                     │
│  └── system-background: CPU 6-7 (大核末端)          │
├─────────────────────────────────────────────────────┤
│  息屏模式:                                           │
│  ├── top-app: CPU 2-3 (大核)                        │
│  ├── background: CPU 4-7 (大核)                     │
│  └── system-background: CPU 6-7 (大核末端)          │
└─────────────────────────────────────────────────────┘
```

---

## 3. 技术方案

### 3.1 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    CPUSet 管理器架构                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ 事件监听器   │    │ 策略控制器   │    │ CPUSet 写入器│  │
│  │ (模式变更)   │───→│ (CpuSetMgr)  │───→│ (cgroup)     │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│         │                    │                    │         │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ AppDetect    │    │ 模式配置表   │    │ 权限检查     │  │
│  │ 屏幕状态     │    │ (4种模式)    │    │ SELinux      │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 核心数据结构

```rust
// src/cpuset_manager/mod.rs
/// CPUSet 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSetConfig {
    /// 是否启用 CPUSet 管理
    pub enabled: bool,
    /// 各模式下的 CPUSet 分配
    pub modes: CpuSetModes,
}

/// 各模式的 CPUSet 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSetModes {
    pub powersave: CpuSetPolicy,
    pub balance: CpuSetPolicy,
    pub performance: CpuSetPolicy,
    pub fast: CpuSetPolicy,
    pub doze: CpuSetPolicy,  // 息屏模式
}

/// 单个模式的 CPUSet 策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSetPolicy {
    /// top-app 组的 CPU 列表
    pub top_app: Vec<u32>,
    /// foreground 组的 CPU 列表
    pub foreground: Vec<u32>,
    /// background 组的 CPU 列表
    pub background: Vec<u32>,
    /// system-background 组的 CPU 列表
    pub system_background: Vec<u32>,
    /// restricted 组的 CPU 列表
    pub restricted: Vec<u32>,
}

impl Default for CpuSetPolicy {
    fn default() -> Self {
        Self {
            top_app: vec![0, 1, 2, 3, 4, 5, 6, 7],
            foreground: vec![0, 1, 2, 3, 4, 5, 6, 7],
            background: vec![2, 3, 4, 5, 6, 7],
            system_background: vec![4, 5, 6, 7],
            restricted: vec![6, 7],
        }
    }
}
```

```rust
// src/cpuset_manager/controller.rs
/// CPUSet 管理器
pub struct CpuSetManager {
    config: CpuSetConfig,
    /// 当前模式
    current_mode: String,
    /// cgroup 路径
    cpuset_root: PathBuf,
    /// 各组的 cpus 文件写入器
    writers: HashMap<String, FastWriter>,
    /// 是否已初始化
    initialized: bool,
}
```

### 3.3 实现逻辑

```rust
// src/cpuset_manager/controller.rs

impl CpuSetManager {
    /// 创建新的 CPUSet 管理器
    pub fn new(config: CpuSetConfig) -> Result<Self> {
        let cpuset_root = Self::detect_cpuset_root()?;
        Ok(Self {
            config,
            current_mode: String::new(),
            cpuset_root,
            writers: HashMap::new(),
            initialized: false,
        })
    }

    /// 检测 cpuset 根路径
    fn detect_cpuset_root() -> Result<PathBuf> {
        let candidates = vec![
            "/dev/cpuset",
            "/sys/fs/cgroup/cpuset",
        ];
        
        for path in candidates {
            if Path::new(path).exists() {
                return Ok(PathBuf::from(path));
            }
        }
        
        anyhow::bail!("未找到 cpuset 挂载点")
    }

    /// 初始化管理器
    pub fn init(&mut self) -> Result<()> {
        let groups = vec![
            "top-app",
            "foreground",
            "background",
            "system-background",
            "restricted",
        ];
        
        for group in groups {
            let cpus_path = self.cpuset_root.join(group).join("cpus");
            if cpus_path.exists() {
                let writer = FastWriter::new(&cpus_path)?;
                self.writers.insert(group.to_string(), writer);
            }
        }
        
        self.initialized = true;
        log::info!("CPUSet 管理器初始化完成，根路径: {:?}", self.cpuset_root);
        Ok(())
    }

    /// 应用指定模式的 CPUSet 配置
    pub fn apply_mode(&mut self, mode: &str) -> Result<()> {
        if !self.config.enabled || !self.initialized {
            return Ok(());
        }
        
        let policy = match mode {
            "powersave" => &self.config.modes.powersave,
            "balance" => &self.config.modes.balance,
            "performance" => &self.config.modes.performance,
            "fast" => &self.config.modes.fast,
            "doze" => &self.config.modes.doze,
            _ => {
                log::warn!("未知模式: {}，使用 balance 配置", mode);
                &self.config.modes.balance
            }
        };
        
        self.apply_policy(policy)?;
        self.current_mode = mode.to_string();
        
        log::debug!("CPUSet 已切换到 {} 模式", mode);
        Ok(())
    }

    /// 应用 CPUSet 策略
    fn apply_policy(&self, policy: &CpuSetPolicy) -> Result<()> {
        let groups = vec![
            ("top-app", &policy.top_app),
            ("foreground", &policy.foreground),
            ("background", &policy.background),
            ("system-background", &policy.system_background),
            ("restricted", &policy.restricted),
        ];
        
        for (group, cpus) in groups {
            if let Some(writer) = self.writers.get(group) {
                let cpus_str = Self::cpus_to_string(cpus);
                writer.write_value_force_str(&cpus_str);
            }
        }
        
        Ok(())
    }

    /// 将 CPU 列表转换为 cpuset 格式的字符串
    /// 例如: [0, 1, 2, 3] -> "0-3"
    /// 例如: [0, 2, 4, 6] -> "0,2,4,6"
    /// 例如: [0, 1, 4, 5, 6, 7] -> "0-1,4-7"
    fn cpus_to_string(cpus: &[u32]) -> String {
        if cpus.is_empty() {
            return String::new();
        }
        
        let mut sorted = cpus.to_vec();
        sorted.sort();
        sorted.dedup();
        
        let mut result = Vec::new();
        let mut start = sorted[0];
        let mut end = sorted[0];
        
        for &cpu in &sorted[1..] {
            if cpu == end + 1 {
                end = cpu;
            } else {
                if start == end {
                    result.push(start.to_string());
                } else {
                    result.push(format!("{}-{}", start, end));
                }
                start = cpu;
                end = cpu;
            }
        }
        
        if start == end {
            result.push(start.to_string());
        } else {
            result.push(format!("{}-{}", start, end));
        }
        
        result.join(",")
    }

    /// 处理模式变更事件
    pub fn on_mode_change(&mut self, new_mode: &str) {
        if new_mode != self.current_mode {
            if let Err(e) = self.apply_mode(new_mode) {
                log::error!("CPUSet 模式切换失败: {}", e);
            }
        }
    }

    /// 处理息屏事件
    pub fn on_screen_off(&mut self) {
        if let Err(e) = self.apply_mode("doze") {
            log::error!("CPUSet 息屏模式切换失败: {}", e);
        }
    }

    /// 处理亮屏事件
    pub fn on_screen_on(&mut self, mode: &str) {
        if let Err(e) = self.apply_mode(mode) {
            log::error!("CPUSet 亮屏模式切换失败: {}", e);
        }
    }
}
```

---

## 4. 配置文件

### 4.1 配置文件位置

```
module/config/cpuset.yaml
```

### 4.2 配置示例

```yaml
# CPUSet 管理配置
enabled: true

modes:
  # 省电模式：限制后台到大核，前台可用全部
  powersave:
    top_app: "0-7"
    foreground: "0-7"
    background: "4-7"
    system_background: "6-7"
    restricted: "6-7"
  
  # 均衡模式：后台可用更多核心
  balance:
    top_app: "0-7"
    foreground: "0-7"
    background: "2-7"
    system_background: "4-7"
    restricted: "6-7"
  
  # 性能模式：所有组都可用全部核心
  performance:
    top_app: "0-7"
    foreground: "0-7"
    background: "0-7"
    system_background: "2-7"
    restricted: "4-7"
  
  # 极速模式：全部核心开放
  fast:
    top_app: "0-7"
    foreground: "0-7"
    background: "0-7"
    system_background: "0-7"
    restricted: "0-7"
  
  # 息屏模式：极致省电，后台限制到大核末端
  doze:
    top_app: "2-3"
    foreground: "2-5"
    background: "4-7"
    system_background: "6-7"
    restricted: "7"
```

---

## 5. 与主调度器集成

```rust
// src/scheduler/mod.rs (新增)

/// 启动 CPUSet 管理线程
fn start_cpuset_manager_thread(
    config: CpuSetConfig,
    mode_receiver: mpsc::Receiver<String>,
    screen_receiver: mpsc::Receiver<bool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut manager = match CpuSetManager::new(config) {
            Ok(m) => m,
            Err(e) => {
                log::error!("CPUSet 管理器创建失败: {}", e);
                return;
            }
        };
        
        if let Err(e) = manager.init() {
            log::error!("CPUSet 管理器初始化失败: {}", e);
            return;
        }

        log::info!("CPUSet 管理线程启动");

        loop {
            // 检查模式变更
            if let Ok(mode) = mode_receiver.try_recv() {
                manager.on_mode_change(&mode);
            }
            
            // 检查屏幕状态
            if let Ok(screen_on) = screen_receiver.try_recv() {
                if screen_on {
                    // 亮屏时恢复当前模式
                    let current_mode = manager.current_mode().to_string();
                    manager.on_screen_on(¤t_mode);
                } else {
                    manager.on_screen_off();
                }
            }
            
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    })
}
```

---

## 6. 风险与回滚

### 6.1 风险点

| 风险 | 影响 | 缓解措施 |
|:---|:---|:---|
| SELinux 权限不足 | 无法写入 cpuset | 需要 Magisk/KernelSU 的 root 权限 |
| cgroup 路径不存在 | 初始化失败 | 自动检测多个候选路径 |
| 配置错误导致卡顿 | 用户体验下降 | 提供合理的默认配置 |

### 6.2 回滚方案

1. **配置关闭**：在 `cpuset.yaml` 中设置 `enabled: false`
2. **恢复默认**：删除配置文件，使用系统默认 cpuset
3. **手动恢复**：`echo 0-7 > /dev/cpuset/top-app/cpus`

---

# CPU 静止下潜实现方案

> 通过 idle injection 机制，在特定场景下主动让 CPU 进入深度空闲状态

---

## 1. 功能概述

### 1.1 什么是 CPU 静止下潜

CPU 静止下潜（CPU Idle Injection）是一种主动省电技术：在 CPU 负载较低时，人为注入空闲周期，让 CPU 进入更深的 C-state（空闲状态），从而降低功耗。

### 1.2 与现有机制的关系

```
┌─────────────────────────────────────────────────────────────┐
│                    省电机制对比                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Doze 模式 (已有)         CPU 静止下潜 (新增)               │
│  ├── 触发: 息屏           ├── 触发: 低负载                  │
│  ├── 方式: 限制最高频率   ├── 方式: 注入空闲周期            │
│  ├── 效果: 限制性能上限   ├── 效果: 强制进入深度 C-state    │
│  └── 适用: 息屏待机       └── 适用: 日用轻负载              │
│                                                             │
│  CLG 降频 (已有)          CPU 静止下潜 (新增)               │
│  ├── 触发: 负载降低       ├── 触发: 负载极低                │
│  ├── 方式: 降低频率       ├── 方式: 强制 idle               │
│  ├── 效果: 降低功耗       ├── 效果: 更深度省电              │
│  └── 适用: 所有场景       └── 适用: 极低负载场景            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 C-state 说明

```
┌─────────────────────────────────────────────────────────────┐
│                    CPU C-state 层级                         │
├─────────────────────────────────────────────────────────────┤
│  C0: 活跃状态 (正在执行指令)                                │
│  C1: 空闲状态 (时钟门控，唤醒延迟 ~1μs)                    │
│  C2: 深度空闲 (电源门控，唤醒延迟 ~10μs)                   │
│  C3: 深度睡眠 (更多电源门控，唤醒延迟 ~100μs)              │
│  C4: 更深睡眠 (完全电源门控，唤醒延迟 ~500μs)              │
└─────────────────────────────────────────────────────────────┘

功耗: C0 > C1 > C2 > C3 > C4
延迟: C0 < C1 < C2 < C3 < C4
```

---

## 2. 技术方案

### 2.1 实现方式选择

| 方案 | 优点 | 缺点 |
|:---|:---|:---|
| **方案 A: cpuidle governor 切换** | 简单，系统原生支持 | 效果有限，只能切换策略 |
| **方案 B: idle injection (intel_idle)** | 效果显著 | 需要内核支持，Android 可能不可用 |
| **方案 C: 用户态模拟 idle** | 兼容性好 | 实现复杂，可能影响调度 |

**选择方案 A**：通过切换 cpuidle governor 和调整参数来实现深度省电。

### 2.2 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    CPU 静止下潜架构                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ 负载检测器   │    │ 策略控制器   │    │ Governor     │  │
│  │ (CPU util)   │───→│ (IdleDive)   │───→│ 切换器       │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│         │                    │                    │         │
│         │                    │                    │         │
│         ▼                    ▼                    ▼         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐  │
│  │ CLG 数据     │    │ 状态机       │    │ 参数调整     │  │
│  │ core_utils   │    │ IDLE→DIVE    │    │ latency_us   │  │
│  └──────────────┘    └──────────────┘    └──────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 核心数据结构

```rust
// src/idle_dive/mod.rs
/// CPU 静止下潜配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleDiveConfig {
    /// 是否启用
    pub enabled: bool,
    /// 触发下潜的负载阈值 (低于此值触发)
    pub dive_threshold: f32,
    /// 退出下潜的负载阈值 (高于此值退出)
    pub exit_threshold: f32,
    /// 下潜延迟 (ms)，负载持续低于阈值多久后触发
    pub dive_delay_ms: u64,
    /// 退出延迟 (ms)，负载持续高于阈值多久后退出
    pub exit_delay_ms: u64,
    /// 各模式下的 cpuidle governor
    pub governors: IdleDiveGovernors,
    /// 各模式下的 idle 参数
    pub params: IdleDiveParams,
}

/// 各模式的 cpuidle governor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleDiveGovernors {
    pub normal: String,      // 正常状态的 governor
    pub diving: String,      // 下潜状态的 governor
    pub doze: String,        // 息屏状态的 governor
}

/// idle 参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleDiveParams {
    /// 正常状态的 latency (μs)
    pub normal_latency_us: u32,
    /// 下潜状态的 latency (μs)
    pub diving_latency_us: u32,
    /// 息屏状态的 latency (μs)
    pub doze_latency_us: u32,
}

impl Default for IdleDiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dive_threshold: 0.15,      // 负载 < 15% 时触发
            exit_threshold: 0.30,      // 负载 > 30% 时退出
            dive_delay_ms: 2000,       // 持续 2 秒低负载后触发
            exit_delay_ms: 500,        // 持续 500ms 高负载后退出
            governors: IdleDiveGovernors {
                normal: "menu".to_string(),
                diving: "menu".to_string(),
                doze: "menu".to_string(),
            },
            params: IdleDiveParams {
                normal_latency_us: 100,
                diving_latency_us: 500,
                doze_latency_us: 1000,
            },
        }
    }
}
```

```rust
// src/idle_dive/controller.rs
/// 下潜状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum DiveState {
    /// 正常状态
    Normal,
    /// 下潜中
    Diving,
    /// 息屏下潜
    DozeDiving,
}

/// CPU 静止下潜控制器
pub struct IdleDiveController {
    config: IdleDiveConfig,
    state: DiveState,
    /// 低负载开始时间
    low_load_start: Option<Instant>,
    /// 高负载开始时间
    high_load_start: Option<Instant>,
    /// cpuidle governor 写入器
    governor_writer: FastWriter,
    /// cpuidle latency 写入器
    latency_writer: FastWriter,
    /// 是否已初始化
    initialized: bool,
}
```

### 2.4 实现逻辑

```rust
// src/idle_dive/controller.rs

impl IdleDiveController {
    /// 创建新的控制器
    pub fn new(config: IdleDiveConfig) -> Result<Self> {
        let governor_path = "/sys/devices/system/cpu/cpuidle/current_governor";
        let latency_path = "/sys/devices/system/cpu/cpuidle/latency_us";
        
        Ok(Self {
            config,
            state: DiveState::Normal,
            low_load_start: None,
            high_load_start: None,
            governor_writer: FastWriter::new(governor_path)?,
            latency_writer: FastWriter::new(latency_path)?,
            initialized: false,
        })
    }

    /// 初始化
    pub fn init(&mut self) -> Result<()> {
        // 应用正常状态的配置
        self.apply_normal_config()?;
        self.initialized = true;
        log::info!("CPU 静止下潜控制器初始化完成");
        Ok(())
    }

    /// 更新状态，由 CLG 定时调用
    pub fn update(&mut self, avg_util: f32) {
        if !self.config.enabled || !self.initialized {
            return;
        }

        let now = Instant::now();

        match self.state {
            DiveState::Normal => {
                if avg_util < self.config.dive_threshold {
                    // 低负载，开始计时
                    if self.low_load_start.is_none() {
                        self.low_load_start = Some(now);
                        self.high_load_start = None;
                    }
                    
                    // 检查是否达到下潜延迟
                    if let Some(start) = self.low_load_start {
                        if now.duration_since(start).as_millis() as u64 
                            >= self.config.dive_delay_ms 
                        {
                            self.enter_dive();
                        }
                    }
                } else {
                    // 负载恢复，重置计时
                    self.low_load_start = None;
                }
            }
            DiveState::Diving => {
                if avg_util > self.config.exit_threshold {
                    // 高负载，开始计时
                    if self.high_load_start.is_none() {
                        self.high_load_start = Some(now);
                        self.low_load_start = None;
                    }
                    
                    // 检查是否达到退出延迟
                    if let Some(start) = self.high_load_start {
                        if now.duration_since(start).as_millis() as u64 
                            >= self.config.exit_delay_ms 
                        {
                            self.exit_dive();
                        }
                    }
                } else {
                    // 负载降低，重置计时
                    self.high_load_start = None;
                }
            }
            DiveState::DozeDiving => {
                // 息屏下潜状态，只在亮屏时退出
            }
        }
    }

    /// 进入下潜状态
    fn enter_dive(&mut self) {
        self.state = DiveState::Diving;
        self.low_load_start = None;
        
        // 切换到下潜 governor
        self.governor_writer.write_value_force_str(&self.config.governors.diving);
        
        // 设置下潜 latency
        self.latency_writer.write_value_force(self.config.params.diving_latency_us);
        
        log::debug!("CPU 静止下潜: 进入下潜状态");
    }

    /// 退出下潜状态
    fn exit_dive(&mut self) {
        self.state = DiveState::Normal;
        self.high_load_start = None;
        
        // 恢复正常 governor
        self.governor_writer.write_value_force_str(&self.config.governors.normal);
        
        // 恢复正常 latency
        self.latency_writer.write_value_force(self.config.params.normal_latency_us);
        
        log::debug!("CPU 静止下潜: 退出下潜状态");
    }

    /// 进入息屏下潜
    pub fn enter_doze(&mut self) {
        self.state = DiveState::DozeDiving;
        
        // 切换到息屏 governor
        self.governor_writer.write_value_force_str(&self.config.governors.doze);
        
        // 设置息屏 latency
        self.latency_writer.write_value_force(self.config.params.doze_latency_us);
        
        log::debug!("CPU 静止下潜: 进入息屏下潜状态");
    }

    /// 退出息屏下潜
    pub fn exit_doze(&mut self) {
        // 根据当前负载决定状态
        self.state = DiveState::Normal;
        self.apply_normal_config();
        
        log::debug!("CPU 静止下潜: 退出息屏下潜状态");
    }

    /// 应用正常配置
    fn apply_normal_config(&self) -> Result<()> {
        self.governor_writer.write_value_force_str(&self.config.governors.normal);
        self.latency_writer.write_value_force(self.config.params.normal_latency_us);
        Ok(())
    }

    /// 获取当前状态
    pub fn state(&self) -> DiveState {
        self.state
    }
}
```

---

## 3. 配置文件

### 3.1 配置文件位置

```
module/config/idle_dive.yaml
```

### 3.2 配置示例

```yaml
# CPU 静止下潜配置
# 注意: 顶层为扁平结构 (无 idle_dive: 包装键)。
# 若加包装键，serde 会对缺失字段全部套用默认值 (与 cpuset.yaml 同源问题)。
enabled: true

# 触发阈值
dive_threshold: 0.15      # 负载 < 15% 时触发
exit_threshold: 0.30      # 负载 > 30% 时退出

# 延迟设置
dive_delay_ms: 2000       # 持续 2 秒低负载后触发
exit_delay_ms: 500        # 持续 500ms 高负载后退出

# cpuidle governor
governors:
  normal: "menu"          # 正常状态
  diving: "menu"          # 下潜状态 (可通过参数调整行为)
  doze: "menu"            # 息屏状态

# idle 参数 (微秒)
params:
  normal_latency_us: 100   # 正常状态允许的 idle 延迟
  diving_latency_us: 500   # 下潜状态允许的 idle 延迟
  doze_latency_us: 1000    # 息屏状态允许的 idle 延迟
```

---

## 4. 与主调度器集成

> **实现说明**: 下述"独立线程 + mpsc 通道"为初版草图。实际实现
> (src/idle_dive/mod.rs + src/scheduler/mod.rs) 复用现有 IPC 主线程事件循环:
> - 共享配置 `Arc<RwLock<IdleDiveConfig>>` 由 config watcher 热重载 (与 cpuset_manager 同范式)
> - `ScreenStateChange` 事件 → `enter_doze()` / `exit_doze()`
> - `SystemLoadUpdate` 事件 → 计算 core_utils 平均值投喂 `update(avg_util)`
>
> 该方式消除了独立线程处理负载/屏幕事件的乱序竞态，且无需额外通道开销。

```rust
// src/scheduler/mod.rs (新增)

/// 启动 CPU 静止下潜线程
fn start_idle_dive_thread(
    config: IdleDiveConfig,
    util_receiver: mpsc::Receiver<f32>,
    screen_receiver: mpsc::Receiver<bool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut controller = match IdleDiveController::new(config) {
            Ok(c) => c,
            Err(e) => {
                log::error!("CPU 静止下潜控制器创建失败: {}", e);
                return;
            }
        };
        
        if let Err(e) = controller.init() {
            log::error!("CPU 静止下潜控制器初始化失败: {}", e);
            return;
        }

        log::info!("CPU 静止下潜线程启动");

        loop {
            // 检查负载数据
            if let Ok(util) = util_receiver.try_recv() {
                controller.update(util);
            }
            
            // 检查屏幕状态
            if let Ok(screen_on) = screen_receiver.try_recv() {
                if screen_on {
                    controller.exit_doze();
                } else {
                    controller.enter_doze();
                }
            }
            
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    })
}
```

---

## 5. 风险与回滚

### 5.1 风险点

| 风险 | 影响 | 缓解措施 |
|:---|:---|:---|
| governor 不支持 | 功能失效 | 检测 governor 是否存在 |
| latency 设置过深 | 唤醒延迟增加 | 设置合理的延迟上限 |
| 与 Doze 冲突 | 状态混乱 | 统一管理状态机 |

### 5.2 回滚方案

1. **配置关闭**：在 `idle_dive.yaml` 中设置 `enabled: false`
2. **恢复默认**：`echo menu > /sys/devices/system/cpu/cpuidle/current_governor`
3. **手动调整**：修改 latency 参数

---

## 附录：参考资料

- Linux Input 事件文档: `Documentation/input/input.rst`
- Android 触摸事件处理: `frameworks/native/services/inputflinger/`
- evdev 协议: `Documentation/input/event-codes.rst`
- Linux cpuidle 框架: `Documentation/admin-guide/pm/cpuidle.rst`
- Linux cpuset: `Documentation/admin-guide/cgroup-v1/cpusets.rst`

---

*文档生成时间：2026-08-01*
*基于 yumi v2.0.1 源码分析*
