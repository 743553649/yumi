# 浅蓝色+纯白色 (Light Ice Blue & White) 极简毛玻璃 UI 统一重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 `android-app` 重塑统一的“浅蓝冰粹毛玻璃”设计语言。以柔和天蓝与纯冰白为基调，搭配动态漂浮的浅蓝/青色天幕光斑、85% 冰白半透明毛玻璃卡片、高对比深色石墨文字与清爽 5 大模式 Accent 标签，全面消除割裂感。

**Architecture:** 全局统一使用 Light Ice Glass Design Token：
1. 背景: 柔和浅蓝/银白渐变底色 (`#F8FAFC` -> `#E0F2FE`) + 浅天蓝/极光青动态流动天幕 (`LiquidMeshBackground.kt`)。
2. 卡片: 85% 冰白微透填充 (`0xD9FFFFFF` ~ `0xF0FFFFFF`) + 纯白高光描边 (`#FFFFFF`) + 柔和蓝色浮空阴影 (`0x200284C7`)。
3. 文字: 深石墨色 (`#0F172A` 主标题)、暗板岩色 (`#334155` 正文)、银灰色 (`#64748B` 辅助文案)。
4. 5大模式标签:
   - 省电 (Powersave): 浅薄荷绿背景 `#E6F0FDF4` + 翡翠绿文字 `#16A34A`
   - 均衡 (Balance): 浅天蓝背景 `#E6E0F2FE` + 海洋蓝文字 `#0284C7`
   - 性能 (Performance): 浅暖琥珀背景 `#E6FFF7ED` + 暖橙文字 `#EA580C`
   - 极速 (Fast): 浅玫瑰红背景 `#E6FEF2F2` + 珊瑚红文字 `#DC2626`
   - FAS (帧感知): 浅薰衣草紫背景 `#E6F3E8FF` + 皇家紫文字 `#9333EA`
   - 默认 (Default): 浅石墨背景 `#E6F1F5F9` + 石墨色文字 `#475569`

**Tech Stack:** Jetpack Compose, Material3, GlassCardView, XML Layouts & Drawables.

## Global Constraints

- 全局主色调: 浅天蓝 (`#0284C7` / `#38BDF8` / `#E0F2FE`) 与 纯冰白 (`#FFFFFF`)。
- 主标题与图标文本颜色必须保持 `#0F172A` 高对比石墨黑，严禁浅色字在浅色背景上无法看清。
- 模式卡片与应用 Badge 必须完全统一使用上述 5 大 Mode Accent 颜色，不允许旧版配色混用。

---

### Task 1: 重塑 Light Mesh Background 与设计 Token (`LiquidMeshBackground.kt` & `colors.xml`)

**Files:**
- Modify: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/LiquidMeshBackground.kt`
- Modify: `android-app/app/src/main/res/values/colors.xml`

**Interfaces:**
- Consumes: None
- Produces: 浅蓝色调 Liquid Mesh 极光天幕与全局 XML 色彩 Token

- [ ] **Step 1: 修改 `LiquidMeshBackground.kt` 调色盘**
  - 底色修改为 `Color(0xFFF8FAFC)`
  - Blob1 (Soft Sky Blue): `Color(0xFF0284C7).copy(alpha = 0.25f)`
  - Blob2 (Soft Ice Cyan): `Color(0xFF38BDF8).copy(alpha = 0.28f)`
  - Blob3 (Soft Electric Blue): `Color(0xFF60A5FA).copy(alpha = 0.22f)`
  - Blob4 (Soft Pastel Turquoise): `Color(0xFF2DD4BF).copy(alpha = 0.20f)`
  - Scrim: 浅天蓝混光渐变 `listOf(Color(0x05FFFFFF), Color(0x10E0F2FE), Color(0x20BAE6FD))`

- [ ] **Step 2: 修改 `colors.xml` Token**
  - `ios_bg_dark`: `#F8FAFC`
  - `ios_glass_card_bg`: `#D9FFFFFF` (85% 冰白半透明)
  - `ios_glass_card_bg_selected`: `#F0F0F9FF`
  - `ios_glass_stroke_default`: `#FFFFFF`
  - `ios_glass_stroke_focused`: `#0284C7`
  - `ios_glass_input_bg`: `#80F1F5F9`
  - `ios_text_primary`: `#0F172A`
  - `ios_text_secondary`: `#334155`
  - `ios_text_muted`: `#64748B`
  - `ios_terminal_bg`: `#F8FAFC`
  - `ios_terminal_text`: `#0F172A`

---

### Task 2: 重构 Compose 首页 2x2 模式选择与 CPU 仪表盘 (`GlassBackdropWrapper.kt`, `LiquidControlCenter.kt`, `LiquidCpuDashboard.kt`)

**Files:**
- Modify: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/GlassBackdropWrapper.kt`
- Modify: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/LiquidControlCenter.kt`
- Modify: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/LiquidCpuDashboard.kt`

**Interfaces:**
- Consumes: Task 1 Design Tokens
- Produces: 浅色冰蓝毛玻璃首页视图组件

- [ ] **Step 1: 修改 `GlassBackdropWrapper.kt` 材质**
  - 85% 冰白半透明渐变：`listOf(Color(0xD9FFFFFF), Color(0xB3E0F2FE))`
  - 纯白与浅蓝高光描边：`listOf(Color(0xFFFFFFFF), Color(0x80FFFFFF), Color(0x400284C7))`

- [ ] **Step 2: 重构 `LiquidControlCenter.kt` 2x2 模式选择卡片**
  - 选中状态: 海洋蓝高光描边 `Color(0xFF0284C7)`，浅蓝填充 `Color(0x30E0F2FE)`
  - 标题与描述文本使用高对比 `#0F172A` 与 `#334155`

- [ ] **Step 3: 重构 `LiquidCpuDashboard.kt` 8核 CPU 仪表盘**
  - CPU 核心进度条与内存利用率使用海洋蓝 `Color(0xFF0284C7)` / `Color(0xFF38BDF8)`
  - 卡片文字统一使用 `#0F172A` 与 `#475569`

---

### Task 3: 统一 XML 日志终端、应用规则与底部导航栏 (`activity_main.xml`, `item_app_rule.xml`, `bg_ios_bottom_nav.xml`, `GlassCardView.java`)

**Files:**
- Modify: `android-app/app/src/main/res/layout/activity_main.xml`
- Modify: `android-app/app/src/main/res/layout/item_app_rule.xml`
- Modify: `android-app/app/src/main/res/drawable/bg_ios_bottom_nav.xml`
- Modify: `android-app/app/src/main/res/drawable/bg_ios_btn_secondary.xml`
- Modify: `android-app/app/src/main/java/com/yumi/bridge/ui/GlassCardView.java`

**Interfaces:**
- Consumes: Task 1 & Task 2 Design Tokens
- Produces: 统一浅色冰蓝 XML 控制台与列表组件

- [ ] **Step 1: 修改 `GlassCardView.java` 默认参数**
  - `customTint = 0xD9FFFFFF` (85% 冰白)
  - `customStrokeStart = 0xFFFFFFFF` (纯白高光)
  - `customStrokeEnd = 0x800284C7` (浅蓝底光)

- [ ] **Step 2: 修改 `bg_ios_bottom_nav.xml` 胶囊导航栏**
  - 填充: `#E8FFFFFF`，描边: `#FFFFFF`

- [ ] **Step 3: 修改 `item_app_rule.xml` & `activity_main.xml`**
  - 搜索框与日志框使用 `#80F1F5F9` 浅蓝灰透明底色，文字使用 `#0F172A`

---

### Task 4: 统一模式 Badge 按钮与主控制逻辑 (`MainActivity.java`)

**Files:**
- Modify: `android-app/app/src/main/java/com/yumi/bridge/MainActivity.java`

**Interfaces:**
- Consumes: All UI Components
- Produces: 完全和谐统一的浅蓝色+纯白 App

- [ ] **Step 1: 修改 `MainActivity.java` 中的 `updateAppModeBtnText` 与 `setLogLevelFilter`**
  - 模式 Badge 文本与背景色严格映射浅色 Token (Powersave `#16A34A`, Balance `#0284C7`, Performance `#EA580C`, Fast `#DC2626`, FAS `#9333EA`, Default `#475569`)
  - Log 筛选按钮选中高亮 `#0284C7`

- [ ] **Step 2: 编译与功能全量验证**
  - 运行: `cargo check --target aarch64-linux-android` 确保后端完美编译。
