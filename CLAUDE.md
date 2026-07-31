# yumi 项目 Claude 规则

## 语言规范

- **始终使用中文回复**：所有解释、注释、交流均使用中文
- **代码注释使用中文**：Rust 源码中的注释使用中文编写
- **技术术语保留英文**：如 PID、EMA、FAS、CLG、CPU、GPU 等缩写保持原样
- **变量名/函数名保持英文**：遵循 Rust 命名规范

## 项目持久化记忆

### 项目基本信息
- **项目名称**: yumi
- **项目类型**: Android CPU 调度器（Rust 实现）
- **主要功能**: 帧感知调度（FAS）、CPU 负载调速器（CLG）、省电优化
- **源码位置**: `/mnt/sdcard/yumi/`

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

### 重要配置文件
- `src/scheduler/config.rs` - 调度器配置
- `src/monitor/config.rs` - 监控配置
- `config/config.yaml` - 运行时配置

### 关键参数说明
- **perf**: 性能指标，范围 [0, 1]，值越高频率越高
- **target_fps**: 目标帧率
- **util**: CPU 利用率
- **smoothing_up/down**: 升频/降频平滑系数
- **perf_floor/ceil**: 性能地板/天花板

## 代码风格

### Rust 编码规范
- 使用 4 空格缩进
- 函数命名使用 snake_case
- 结构体命名使用 PascalCase
- 常量命名使用 SCREAMING_SNAKE_CASE
- 每行不超过 100 字符（灵活处理）

### 注释规范
- 模块级注释使用 `// ════════════════════════════════════════════════════════════════`
- 函数注释使用 `///` 文档注释
- 行内注释使用 `//`
- 复杂逻辑必须添加注释说明

### 日志规范
- 使用 `log::info!`, `log::debug!`, `log::warn!`, `log::error!`
- 日志内容使用中文
- 使用 `t()` 和 `t_with_args()` 进行国际化

## 工作流程

### 修改代码前
1. 阅读相关文档（如 `powersave_optimization.md`）
2. 理解现有架构和参数含义
3. 确认修改影响范围

### 修改代码时
1. 保持代码风格一致
2. 添加必要的注释
3. 遵循现有命名规范

### 修改代码后
1. 更新 `worklog.md` 记录修改
2. 如有配置变更，更新相关文档
3. 确保日志输出清晰可读

## 测试与验证

### 编译测试
```bash
cargo build
cargo test
```

### 功能验证
- 日用场景：社交、阅读、浏览
- 游戏场景：60fps/120fps 游戏
- 息屏场景：待机、后台同步

### 监控指标
- CPU 频率变化
- 帧率稳定性
- 功耗变化
- 温度变化

## 重要文件索引

| 文件 | 用途 |
|:---|:---|
| `powersave_optimization.md` | 省电优化方案文档 |
| `worklog.md` | 修改工作日志（记录省电优化实施状态） |
| `PROJECT_MAP.md` | 项目结构地图 |
| `README.md` | 项目说明文档 |
| `src/scheduler/config.rs` | 调度器配置定义 |
| `src/scheduler/mod.rs` | 调度器状态机与事件处理 |
| `src/scheduler/cpu_load_governor.rs` | CLG 负载调速器实现 |
| `src/scheduler/fas/pid.rs` | FAS PID 控制器 |
| `src/scheduler/fas/controller.rs` | FAS 主控制器 |
| `src/scheduler/fas/frame_pipeline.rs` | FAS 帧流水线处理 |
| `src/scheduler/fas/gear_state.rs` | FAS 档位状态管理 |

## 注意事项

### 修改风险
- 参数修改可能影响帧率稳定性
- 叠加修改可能产生交互效应
- 需要充分测试验证

### 回滚策略
- 记录所有修改的原始值
- 准备参数回调预案
- 分阶段实施，逐步验证

### 协作规范
- 修改前说明目的和影响
- 修改后记录结果和观察
- 保持文档与代码同步更新

---

*最后更新: 2026-07-31*
*项目版本: yumi v2.0.1*
