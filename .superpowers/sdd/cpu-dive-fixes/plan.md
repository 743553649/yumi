# CPU静止下潜代码质量修复计划

## 背景
用户完成了CPU静止下潜功能的编码实现，经过代码审查发现以下问题需要修复。

## 问题清单

### 问题 1：IdleDive 的 dive_timer 逻辑 bug
**文件**: `src/idle_dive/controller.rs`
**问题**: `dive_timer` 在进入 Diving 状态后没有重置，导致后续可能立即再次进入下潜
**修复**: 在 `avg_util >= exit_threshold` 时重置 `dive_timer`

### 问题 2：TouchBoost 衰减频率未取整到有效值
**文件**: `src/touch_boost/controller.rs`
**问题**: 衰减后的频率可能不在 `scaling_available_frequencies` 列表里，写入 sysfs 可能被内核拒绝
**修复**: 用 `find_nearest_freq()` 找最近的有效频率

### 问题 3：PM-QoS 错误被静默忽略
**文件**: `src/idle_dive/controller.rs`
**问题**: PM-QoS 写入失败时用 `let _` 忽略错误，用户不知道
**修复**: 至少打个 warn 日志

### 问题 4：检查冗余代码
**涉及文件**: 所有新增文件
**任务**: 检查是否有未使用的导入、变量、函数，是否有重复逻辑

### 问题 5：添加编译验证
**任务**: 确保代码能通过 `cargo check`

## 全局约束
- 使用通俗中文注释（如果需要）
- 代码变量名保持英文
- 不添加不必要的注释
- 遵循现有代码风格
- 每个修复完成后运行 `cargo check` 验证

## 任务列表

### Task 1: 修复 IdleDive timer 逻辑
**文件**: `src/idle_dive/controller.rs`
**修改内容**:
1. 在 `update()` 方法中，当 `avg_util >= exit_threshold` 时重置 `dive_timer`
2. 确保状态转换逻辑正确

### Task 2: 修复 TouchBoost 频率取整
**文件**: `src/touch_boost/controller.rs`
**修改内容**:
1. 在 `update()` 方法中，衰减后的频率需要找到最近的有效频率
2. 需要获取 `scaling_available_frequencies` 或使用 FastWriter 的频率列表

### Task 3: 修复 PM-QoS 错误处理
**文件**: `src/idle_dive/controller.rs`
**修改内容**:
1. 在 `transition_to()` 方法中，为 PM-QoS 写入失败添加 warn 日志
2. 使用 `if let Err(e)` 模式而不是 `let _`

### Task 4: 清理冗余代码
**涉及文件**: 所有新增文件
**检查内容**:
1. 未使用的导入
2. 未使用的变量
3. 重复的逻辑
4. 可以简化的代码

### Task 5: 最终验证
**任务**: 
1. 运行 `cargo check` 确保编译通过
2. 检查所有修改的一致性

## 验证标准
- 所有修改后的代码能通过 `cargo check`
- 修复了所有已知问题
- 没有引入新问题
- 代码风格与现有代码一致
