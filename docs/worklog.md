# yumi 省电优化工作日志

> 记录 docs/powersave_optimization.md 中所有修改项的实施状态

---

## 修改状态汇总

| 优先级 | 修改项 | 文件 | 状态 | 完成日期 |
|:---:|:---|:---|:---:|:---:|
| 1 | util_cap 封顶更严格 | policy_mgmt.rs | ✅ 已完成 | 2026-07-31 |
| 2 | PID util_gain 扩大到 0.45 | pid.rs | ✅ 已完成 | 2026-07-31 |
| 3 | Doze 更深度 | mod.rs | ✅ 已完成 | 2026-07-31 |
| 4 | target_fps 偏移加大 | controller.rs | ✅ 已完成 | 2026-07-31 |
| 5 | 快速衰减更激进 | frame_pipeline.rs | ✅ 已完成 | 2026-07-31 |
| 6 | CLG 升频逻辑收紧 | cpu_load_governor.rs | ✅ 已完成 | 2026-07-31 |
| 7 | 升档门槛提高 | gear_state.rs | ✅ 已完成 | 2026-07-31 |
| 8 | EMA 升频平滑降低 | frame_pipeline.rs | ✅ 已完成 | 2026-07-31 |

---

## 详细修改记录

### 1. CLG 负载调速器优化

**文件**: `src/scheduler/cpu_load_governor.rs`

#### 1.1 提高 significant_jump 阈值
- **位置**: `update_perf()` 函数第 248 行
- **原值**: `0.35`
- **新值**: `0.50`
- **目的**: 减少日用场景因负载小幅波动导致的快速升频

#### 1.2 提高低负载降频加速阈值
- **位置**: `update_perf()` 函数第 260-261 行
- **原值**: `util < 0.10`, `smoothing_down * 2.5`
- **新值**: `util < 0.15`, `smoothing_down * 3.0`
- **目的**: 轻负载场景降频更快，节省电量

---

### 2. FAS 帧感知调度优化

#### 2.1 PID 利用率感知范围扩大
**文件**: `src/scheduler/fas/pid.rs`
- **位置**: `compute()` 函数第 96 行
- **原值**: `fg_util < 0.30`, 系数 `2.3`
- **新值**: `fg_util < 0.45`, 系数 `1.56`
- **目的**: 日用轻中负载场景减少无效拉频

#### 2.2 target_fps 偏移更激进
**文件**: `src/scheduler/fas/controller.rs`
- **位置**: `adjust_target_for_util()` 函数第 268-272 行
- **修改内容**:
  - 触发阈值: `0.55/0.65` → `0.50/0.60`
  - 偏移步长: `-0.1` → `-0.2`
  - 最大偏移: `-3.0` → `-5.0`
  - 恢复速度: `0.1` → `0.15`
- **目的**: GPU bound 场景更快降低 target_fps，节省功耗

#### 2.3 快速衰减更激进
**文件**: `src/scheduler/fas/frame_pipeline.rs`

**2.3.1 放宽高刷衰减限制**
- **位置**: `update_decay()` 第 165 行
- **原值**: `0.6`
- **新值**: `0.75`
- **目的**: 高刷下帧率稳定时允许更快衰减

**2.3.2 降低高刷衰减阈值增长系数**
- **位置**: `update_decay()` 第 154-155 行
- **原值**: 系数 `0.002`, 上限 `0.15`
- **新值**: 系数 `0.001`, 上限 `0.08`
- **目的**: 降低高刷下快速衰减的触发门槛

#### 2.4 EMA 升频平滑系数降低
**文件**: `src/scheduler/fas/frame_pipeline.rs`
- **位置**: `update_ema()` 第 135 行
- **原值**: `0.15 * fps_factor`, clamp `(0.10, 0.35)`
- **新值**: `0.10 * fps_factor`, clamp `(0.08, 0.25)`
- **目的**: 单帧卡顿不会导致频率飙升

#### 2.5 升档门槛提高
**文件**: `src/scheduler/fas/gear_state.rs`
- **位置**: `check_upgrade()` 第 90-94 行
- **修改内容**:
  - overshoot: `1.35` → `1.50`
  - fps_window.count(): `15` → `20`
  - recent30: `1.2` → `1.25`
  - perf_index: `0.45` → `0.40`
- **目的**: 减少因瞬时负载波动导致的误升档

---

### 3. Doze 息屏模式优化

**文件**: `src/scheduler/mod.rs`
- **位置**: `enter_doze()` 逻辑第 224-230 行
- **修改内容**:
  - perf_ceil: `0.40` → `0.30`
  - smoothing_up: `0.10` → `0.05`
  - up_rate_limit_ticks: `3` → `5`
- **目的**: 息屏待机功耗显著降低

---

## 测试验收标准

### 日用场景
- 功耗降低 ≥ 8%
- jank 次数增加 ≤ 10%

### 游戏场景
- jank 次数增加 ≤ 15%
- 120fps 游戏帧率稳定性无明显下降

### 息屏场景
- 功耗降低 ≥ 15%

---

## 参数回调预案

如测试中发现问题，按以下优先级调优：

### 日用场景卡顿
1. 回调 util_cap 上限: `0.90` → `0.95`
2. 回调 PID util_gain 系数: `1.56` → `1.8`
3. 回调 target_fps 最大偏移: `-5.0` → `-4.0`

### 游戏场景帧率不稳
1. 回调快速衰减阈值: `0.76` → `0.80`
2. 回调 decay_scale: `0.75` → `0.65`
3. 回调 target_fps 最大偏移: `-5.0` → `-3.0`

### 息屏后唤醒慢
1. 回调 perf_ceil: `0.30` → `0.35`
2. 回调 smoothing_up: `0.05` → `0.08`
3. 回调 up_rate_limit_ticks: `5` → `4`

---

## 版本历史

| 版本 | 日期 | 修改内容 |
|:---|:---|:---|
| v1.0 | 2026-07-31 | 完成全部 8 项省电优化修改 |
| v1.1 | 2026-07-31 | 创建 KernelSU 模块打包脚本，更新 module.prop 描述 |

---

## 打包记录

### 2026-07-31 KernelSU 模块打包

**输出文件**: `output/yumi-v2.0.1-0-20260731-1316.zip`

**打包内容**:
- `core/bin/yumi` - 核心二进制文件 (1.5M)
- `webroot/` - WebUI 界面文件 (4 文件)
- `config/` - 配置文件 (config.yaml, i18n/)
- `scripts/` - 脚本文件 (disable_boost.sh)
- `META-INF/` - 模块元数据
- `module.prop` - 模块属性
- `rules.yaml` - 调度规则
- `service.sh` - 启动脚本
- `customize.sh` - 安装脚本
- `uninstall.sh` - 卸载脚本

**打包脚本**: `pack_module.sh`

**注意**: 由于当前环境编译限制（sdcard 文件系统无法执行编译生成的二进制文件），需要在有完整编译环境的机器上重新编译。

**编译指南**: 参见 `docs/BUILD_INSTRUCTIONS.md`

**编译脚本**: `build_and_pack.sh`

---

*最后更新: 2026-07-31*
*基于 docs/powersave_optimization.md v1.0*
