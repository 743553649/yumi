# Task 6: 阶段六 — 配置文件与文档

## 目标
完善配置文件，更新文档。

## 依赖
- Task 5 已完成：所有功能已实现

## 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `module/config/idle_dive.yaml` | 确认配置文件存在且正确 |
| `module/config/touch_boost.yaml` | 确认配置文件存在且正确 |
| `module/config/config.yaml` | 添加完整配置示例 |
| `README.md` | 文档化新功能 |
| `AGENTS.md` | 新增「功能计划文档保存到 docs/」规则 |

## 实现细节

### 6.1 确认配置文件

确认 `idle_dive.yaml` 和 `touch_boost.yaml` 已在 Task 3 和 Task 4 中创建。

### 6.2 config.yaml 完整配置

确认 config.yaml 包含：
```yaml
function:
  SchedulerTuning: true
  # ... 原有 ...

StillDive:
  enabled: true
  enter_threshold: 0.08
  enter_ticks: 10
  exit_threshold: 0.20
  exit_boost_ticks: 5
  perf_ceil: 0.30
  smoothing_up: 0.05
```

### 6.3 README.md 更新

在 README.md 中添加新功能说明：
- StillDive：亮屏静止下潜
- IdleDive：CPU 空闲下潜
- TouchBoost：触摸提频
- 三层联动架构

### 6.4 AGENTS.md 更新

在 AGENTS.md 中添加规则：
```
功能计划文档保存到 docs/ 目录
```

## 验证点
- 所有配置文件存在且格式正确
- README.md 包含新功能说明
- AGENTS.md 包含新规则

## 代码风格要求
- 遵循现有文档风格
- 使用中文
