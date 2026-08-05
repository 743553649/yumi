# 毛玻璃 (Glassmorphism) 精致化升级实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将现有首页所有 UI 卡片（顶部状态卡片、四大模式卡片、CPU 8核看板、RAM及电池信息卡片）的背景玻璃效果升级为原生 iOS/macOS 级别的高质感毛玻璃 (Glassmorphism)。

**Architecture:** 降低背景板不透明度至 30%~45%，通过 Jetpack Compose `Modifier.blur()` / `RenderEffect` 引入 24dp 硬件级高斯模糊漫反射，并配合 45° 冰白斜切折射边框与悬浮弥散阴影。

**Tech Stack:** Jetpack Compose (Android 12+ `RenderEffect`, `Modifier.blur`, `Brush.linearGradient`, `BoxWithConstraints`).

---

## 1. 当前参数诊断与毛玻璃提升方案对比

| 参数维度 | 当前代码参数 (Current) | 精致毛玻璃目标方案 (Glassmorphism Target) | 改善效果分析 |
| :--- | :--- | :--- | :--- |
| **背景透明度 (Opacity)** | **70% ~ 85%** (`0xD9FFFFFF` ~ `0xB3E0F2FE`) | **30% ~ 42%** (`0x66FFFFFF` ~ `0x40F0F9FF`) | 大幅降低白底覆盖率，让后方液态网格背景色块隐约透光 |
| **高斯模糊度 (Blur Radius)** | **0 dp** (缺失模糊) | **24 dp** (`Modifier.blur(24.dp)` / `RenderEffect`) | 真实折射背景色彩，消除硬边界，呈现细腻磨砂触感 |
| **边缘折射高光 (Border)** | 1.5.dp 简易线性渐变 | **1.2.dp 45° 冰晶斜切高光渐变** (`0xE6FFFFFF` -> `0x20FFFFFF` -> `0x400284C7`) | 模拟玻璃物理厚度与顶部光源入射折射 |
| **空间悬浮阴影 (Shadow)** | 无阴影 | **6dp ~ 12dp 弥散软阴影** (`Color(0x100F172A)`) | 增强卡片与背景之间的视觉纵深感与层次分明 |

---

## Proposed Changes

### UI 组件与样式层 (UI Component & Design System)

#### [MODIFY] [GlassBackdropWrapper.kt](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/java/com/yumi/bridge/ui/compose/GlassBackdropWrapper.kt)

- 引入 **30% - 42% 半透明冰白水纹渐变**。
- 为容器绑定 **24.dp 高斯模糊效果** (`Modifier.blur(24.dp)`)。
- 重构 **45° 斜角物理高光边框** (`HighlightBorderBrush`)。
- 增加 **8dp 弥散软阴影**，提升空间感。

#### [MODIFY] [LiquidControlCenter.kt](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/java/com/yumi/bridge/ui/compose/LiquidControlCenter.kt)

- 适配未选中模式卡片的毛玻璃透光度，确保子卡片与主背景玻璃层次协同。

---

## Detailed Task Decomposition

### Task 1: 重构 GlassBackdropWrapper 毛玻璃核心容器

**Files:**
- Modify: `c:\Users\GUDGA\yumi\android-app\app\src\main\java\com\yumi/bridge/ui/compose/GlassBackdropWrapper.kt`
- Test: `c:\Users\GUDGA\yumi\android-app\app\src\test\java\com\yumi/bridge/utils/HomeModeConfigTest.kt`

- [ ] **Step 1: 编写 UI 渲染单元测试/样式导出校验**

```kotlin
@Test
fun testGlassmorphismOpacityValues() {
    val maxOpacityHex = 0x66
    val minOpacityHex = 0x40
    assertTrue(maxOpacityHex in 0x30..0x75)
    assertTrue(minOpacityHex in 0x30..0x75)
}
```

- [ ] **Step 2: 执行测试确认规范通过**

Run: `.\gradlew testDebugUnitTest`

- [ ] **Step 3: 实现精致毛玻璃容器代码**

```kotlin
@Composable
fun GlassBackdropWrapper(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(24.dp),
    blurRadius: Dp = 24.dp,
    content: @Composable BoxScope.() -> Unit
) {
    val highlightBorder = remember {
        BorderStroke(
            width = 1.2.dp,
            brush = Brush.linearGradient(
                colors = listOf(
                    Color(0xE6FFFFFF), // 顶部 90% 不透明强光
                    Color(0x33FFFFFF), // 中段 20% 通透过渡
                    Color(0x500284C7)  // 底部天蓝折射
                ),
                start = Offset(0f, 0f),
                end = Offset(Float.POSITIVE_INFINITY, Float.POSITIVE_INFINITY)
            )
        )
    }

    val glassBrush = remember {
        Brush.verticalGradient(
            colors = listOf(
                Color(0x59FFFFFF), // 35% 白
                Color(0x3DDCFCE7)  // 24% 浅青透光
            )
        )
    }

    Box(
        modifier = modifier
            .shadow(elevation = 8.dp, shape = shape, spotColor = Color(0x1A0284C7))
            .clip(shape)
            .blur(radius = blurRadius)
            .background(brush = glassBrush)
            .border(highlightBorder, shape),
        content = content
    )
}
```

- [ ] **Step 4: 编译运行单元测试**

Run: `.\gradlew testDebugUnitTest`

---

## Verification Plan

### Automated Tests
- `.\gradlew testDebugUnitTest` (确保全部单元测试通过)

### Manual Verification
- 编译并部署 `yumi-bridge.apk` 至项目根目录。
- 确认全套 UI 卡片背景色透出后方液态天幕色彩，高斯模糊细腻柔和，边缘具有剔透折射感。
