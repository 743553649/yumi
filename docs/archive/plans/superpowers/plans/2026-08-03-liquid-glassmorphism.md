# AndroidLiquidGlass (Backdrop) 集成与编译适配计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 解决 Kyant0/backdrop 库与当前 App 的 Kotlin 1.9.24 编译器版本兼容冲突，在 App 上完美重现官方 Liquid Glass (流体玻璃) 效果。

---

## 1. 编译冲突诊断 (Compilation Conflict Analysis)

在集成测试中，Kotlin 编译器抛出以下错误：
> `Module was compiled with an incompatible version of Kotlin. The binary version of its metadata is 2.2.0, expected version is 1.9.0.`

**原因分析**：Kyant0/backdrop 库（包括 1.0.0 到 2.0.0 版本）全部基于 **Kotlin 2.x (Kotlin Multiplatform)** 编译。其导出的 `.kotlin_module` 元数据版本为 `2.2.0+`，而我们当前项目的 Kotlin 编译器版本为 `1.9.24`（仅能读取 <= 2.0.0 的元数据）。

为了达成与官方 Demo 一致的视觉效果，我们有两个实施选项：

---

## 2. 备选实施方案对比 (Implementation Options)

### 选项 A：将项目编译器升级至 Kotlin 2.0.21 (推荐)
通过将项目整体重构升级为 Kotlin 2.x，直接引入 `io.github.kyant0:backdrop` 库：
1. 升级 Kotlin 版本至 `2.0.21`。
2. 废弃 `kotlinCompilerExtensionVersion = "1.5.14"`，引入 Compose 2.0 官方编译器插件 `id("org.jetbrains.kotlin.plugin.compose")`。
3. 完美调用 `Modifier.drawBackdrop` 渲染物理像素折射。

### 选项 B：原生 Compose 像素渲染模拟方案 (轻量无侵入)
不引入第三方库，使用 Android 12+ 原生 `RenderEffect` 与 `GraphicsLayer` 实现等同的液态高斯漫反射：
1. 利用 `graphicsLayer { renderEffect = RenderEffect.createBlurEffect(...) }` 捕获下层视图。
2. 使用自定义的 `DrawModifier` 模拟液态镜头（Lens）的光学边缘折射与反射。

---

## Proposed Changes (Based on Option A)

### Root Gradle 配置

#### [MODIFY] [build.gradle.kts](file:///c:/Users/GUDGA/yumi/android-app/build.gradle.kts)
- 升级 Kotlin 插件版本到 `2.0.21`。
- 引入 Compose Compiler 插件。

### App Module Gradle 配置

#### [MODIFY] [build.gradle.kts](file:///c:/Users/GUDGA/yumi/android-app/app/build.gradle.kts)
- 移出旧的 `composeOptions` 块。
- 启用 `org.jetbrains.kotlin.plugin.compose` 插件。
- 引入 `io.github.kyant0:backdrop:2.0.0` 依赖。

### UI 渲染层

#### [MODIFY] [ComposeHomeBridge.kt](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/java/com/yumi/bridge/ui/compose/ComposeHomeBridge.kt)
- 初始化 `rememberLayerBackdrop()` 全局捕获状态。

#### [MODIFY] [GlassBackdropWrapper.kt](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/java/com/yumi/bridge/ui/compose/GlassBackdropWrapper.kt)
- 升级为 `drawBackdrop` GPU 实时模糊渲染。

---

## 详细开发步骤 (Detailed Execution Steps - Option A)

### Task 1: 升级项目至 Kotlin 2.0.21 并同步

**Files:**
- Modify: `c:\Users\GUDGA\yumi\android-app\build.gradle.kts`
- Modify: `c:\Users\GUDGA\yumi\android-app\app\build.gradle.kts`

- [ ] **Step 1: 升级根目录 kotlin 插件版本**

```kotlin
plugins {
    id("com.android.application") version "8.2.2" apply false
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.0.21" apply false
}
```

- [ ] **Step 2: 在 app/build.gradle.kts 启用新 compose 插件，删除旧 composeOptions**

```kotlin
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}
// 删除 composeOptions { kotlinCompilerExtensionVersion = "1.5.14" }
```

- [ ] **Step 3: 同步并执行测试编译**

Run: `.\gradlew compileDebugKotlin`
Expected: BUILD SUCCESSFUL (无 Kotlin 元数据版本冲突错误)

---

### Task 2: 绑定 layerBackdrop 采样源与 drawBackdrop 渲染

- [ ] **Step 1: 在 ComposeHomeBridge.kt 对底色天幕应用采样**
- [ ] **Step 2: 在 GlassBackdropWrapper.kt 实现 drawBackdrop 物理折射**

---

## 验证计划 (Verification Plan)
- 运行 `.\gradlew testDebugUnitTest` 校验所有核心测试。
- 编译生成最终安装包并测试滑动，验证液态高斯漫反射流畅度。
