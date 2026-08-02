# yumi Bridge 底部导航栏与多界面重构设计方案 (Implementation Plan)

## Goal Description
为 yumi Bridge Android 控制端 App 重构界面，增加底部 **iOS 冰粹毛玻璃导航栏**，支持【首页】、【日志】、【应用】三个独立页面 Tab：
1. **首页 (Home Tab)**: 展示 yumi 守护进程在线/离线状态、实时指标（FPS、CPU 负载、IPC 状态）以及 2x2 全局调度模式切换卡片。
2. **日志 (Logs Tab)**: 独占全屏页面，支持 5 级日志过滤（全部、调试、信息、警告、错误）、自动滚动、日志一键清空及流畅的终端滚屏。
3. **应用 (Apps Tab)**: 应用规则管理面板，支持搜索已安装应用与预置游戏列表（如王者荣耀、原神、崩坏：星穹铁道、鸣潮等），并为每个独立 App 分配特定调度模式。

---

## User Review Required

> [!IMPORTANT]
> **确认的 6 种应用调度模式选项 (Confirmed App Mode Options)**：
> 每个独立 App 支持精准分配以下 6 种模式选项之一：
> 1. **【跟随全局 (Default)】**: 恢复随全局 `global_mode` 自动切换（移除该包名的独立规则）
> 2. **【省电 (Powersave)】**: 强制限制 CPU 频率上限，优先续航
> 3. **【均衡 (Balance)】**: 动态跟随负载，兼顾流畅与功耗
> 4. **【性能 (Performance)】**: 激进升频与 TouchBoost 响应
> 5. **【极速 (Fast)】**: 全核解锁频，延迟最低
> 6. **【FAS 帧感知 (FAS)】**: 启用动态 30/60/90/120/144 FPS 帧感知智能 PID 算法

> [!IMPORTANT]
> **底部导航栏样式与交互模式**：
> 底部导航栏采用高对比度 iOS 冰粹毛玻璃底座。Tab 切换采用多容器切换装载，确保在 3 个 Tab 切换时流畅过渡、无卡顿。

> [!IMPORTANT]
> **应用规则 (App Rules) 数据的持久化与 IPC 机制**：
> 1. 应用规则直接与 `/storage/emulated/0/yumi/module/rules.yaml` 中的 `app_modes` 属性同步写回。
> 2. Android App 通过标准 File/IPC 发送 `set_app_mode <package> <mode>`，确保即时生效且重启后持久保存。

---

## Proposed Changes

### Android App (`android-app`)

#### [MODIFY] `app/src/main/res/layout/activity_main.xml`
- 将原有的单页结构改造为**多 Tab 页面容器 + 底部毛玻璃导航栏**。
- 引入 3 个 Tab 页面容器：
  - `tabHomeContainer`: 包含守护进程 Header 卡片 + 2x2 模式选择卡片网格。
  - `tabLogsContainer`: 独占全屏终端日志面板与 5 级筛选控件。
  - `tabAppsContainer`: 包含搜索框、已安装应用列表项与 6 模式下拉选择按钮。
- 底部添加 `bottomNavLayout` (高度 64dp，内置【首页】、【日志】、【应用】3 个 Icon+Title 导航项)。

#### [MODIFY] `app/src/main/res/values/strings.xml`
- 添加 Tab 标题文案及应用规则管理相关字符串 (`nav_home`, `nav_logs`, `nav_apps`, `app_rule_title`, `app_rule_subtitle`, `search_app_hint` 等)。

#### [NEW] `app/src/main/res/drawable/bg_ios_bottom_nav.xml`
- 底部导航栏专属毛玻璃胶囊 Drawable（高对比白/黑亮色渐变边框 + 轻微投影）。

#### [NEW] `app/src/main/res/layout/item_app_rule.xml`
- 应用规则列表单项 Layout，包含应用图标/占位图、应用包名、应用中文名、当前生效模式 Badge，以及 6 模式弹出选择菜单。

#### [MODIFY] `app/src/main/java/com/yumi/bridge/MainActivity.java`
- 引入 Tab 切换状态机 (`currentTab = TAB_HOME | TAB_LOGS | TAB_APPS`)。
- 绑定底部导航栏点击事件，平滑显示/隐藏对应 Tab 容器。
- 实现【应用规则管理】逻辑：
  - 读取与解析 `rules.yaml` 中的 `app_modes` 规则。
  - 动态扫描与展示应用列表（支持搜索框过滤）。
  - 处理应用专属模式修改并写回 `rules.yaml` / 发送 IPC 命令。

---

### Rust 守护进程 IPC 服务 (`src/ipc_server.rs`)

#### [MODIFY] `src/ipc_server.rs`
- 扩展 IPC 命令：
  - `get_app_modes`: 返回 `rules.yaml` 中已配置的 `app_modes` 映射清单。
  - `set_app_mode <package> <mode>`: 更新 `rules.yaml` 中的 `app_modes` 属性并即时生效。

---

## Verification Plan

### Automated Tests
1. **Rust 核心与 IPC 单元测试**:
   ```bash
   cargo test --lib
   ```
2. **Android APK 构建与签名验证**:
   ```bash
   bash /storage/emulated/0/yumi/android-app/build_apk.sh
   ```

### Manual Verification
1. 打开生成的 `yumi-bridge.apk`，验证底部导航栏【首页】、【日志】、【应用】3 个按钮切换正常。
2. 切换至【日志】Tab，验证日志独占全屏，5 级筛选（全部/调试/信息/警告/错误）与自动滚动正常。
3. 切换至【应用】Tab，搜索 `com.tencent.tmgp.sgame`（王者荣耀）或 `com.miHoYo.GenshinImpact`（原神），修改其模式为 `FAS` / `Performance` / `Powersave` / `Balance` / `Fast` / `Default`，验证设置生效并成功写入规则。
