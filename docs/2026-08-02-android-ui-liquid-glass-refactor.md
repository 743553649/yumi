# Android App 顶级流体玻璃 (Liquid Glass) UI 重构实施计划书 (v2.0 终极版)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 `io.github.kyant0:backdrop` (AndroidLiquidGlass) 与 Jetpack Compose，对 `android-app` 进行全方位的流体玻璃 (Liquid Glass) 顶级 UI 重构。彻底修复主线程 IO 阻塞 (ANR 风险) 与内存抖动，通过新建 `ComposeHomeBridge.kt` 封装响应式 `HomeUiState` 状态，打造完全媲美 [Kyant0/AndroidLiquidGlass](https://github.com/Kyant0/AndroidLiquidGlass) 展示效果的动态极光光斑天幕背景、物理折射边缘光泽、2x2 交互卡片与 8 核 CPU 实时仪表盘。

**Architecture:** 采用“后台异步 IO 采样 ($\text{Executors.newSingleThreadExecutor()}$) $\rightarrow$ 响应式 Compose 状态树 ($\text{HomeUiState}$) $\rightarrow$ 动态流体天幕背景 (LiquidMeshBackground) $\rightarrow$ 实时图层捕捉 (LayerBackdrop)”的高性能 4 层物理架构。使用 `ComposeHomeBridge.kt` 桥接 Java `MainActivity`，解决 Java 编译兼容与频繁 GC 内存抖动问题。

**Tech Stack:** Kotlin 1.9.24+, Jetpack Compose BOM 2024.06.00, Material3, `io.github.kyant0:backdrop:1.0.1`, Android SDK 34 (minSdk 26).

---

## Global Constraints & Critical Security Rules

- **主线程非阻塞 (ANR 防御)**：绝对禁止在主线程 (`Looper.getMainLooper()`) 中同步调用 `Runtime.getRuntime().exec("su", ...)` 或 `p.waitFor()`。所有 CPU `/proc/stat` 采样与 Shell 命令必须在后台子线程完成，仅通过 Handler 提交状态增量。
- **状态维护与 GC 零抖动**：绝对禁止在 1 秒轮询中重复调用 `composeView.setContent` 或频繁创建临时 `long[]` 数组。使用 `ComposeHomeBridge` 维持单例 `HomeUiState`。
- **视觉还原标准**：完全达到 VisionOS / Kyant0 Demo 展示效果。背景必须拥有平滑流动的动态弥散光斑（Mesh Gradient Blobs），以使前景 `Backdrop` 玻璃卡片产生令人惊艳的实时动态透光折射与高光描边。
- **最低 SDK 与降级策略**：`minSdk = 26` (Android 8.0)。API 33+ (Android 13+) 使用硬件加速 `RenderEffect` 与 AGSL 液体 Shader；API 26-32 自动降级至 RenderNode 高斯模糊 + 85% 冰白半透明混色。

---

### Task 1: Gradle 依赖与 Compose 基础设施配置

**Files:**
- Modify: `android-app/app/build.gradle.kts`
- Modify: `android-app/build.gradle.kts`

- [ ] **Step 1: 修改 `android-app/build.gradle.kts` 注入 Kotlin 插件**

```kotlin
plugins {
    id("com.android.application") version "8.2.2" apply false
    id("org.jetbrains.kotlin.android") version "1.9.24" apply false
}
```

- [ ] **Step 2: 修改 `android-app/app/build.gradle.kts` 配置依赖**

```kotlin
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.yumi.bridge"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.yumi.bridge"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.14"
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")

    // Jetpack Compose BOM & UI
    implementation(platform("androidx.compose:compose-bom:2024.06.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.9.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.2")

    // Kyant0 AndroidLiquidGlass (Backdrop)
    implementation("io.github.kyant0:backdrop:1.0.1")

    debugImplementation("androidx.compose.ui:ui-tooling")
}
```

- [ ] **Step 3: 编译验证**

Run: `cd /storage/emulated/0/yumi/android-app && ./gradlew assembleDebug`
Expected: BUILD SUCCESSFUL

---

### Task 2: 动态彩色流体天幕与容器 (`LiquidMeshBackground.kt` & `GlassBackdropWrapper.kt`)

**Files:**
- Create: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/LiquidMeshBackground.kt`
- Create: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/GlassBackdropWrapper.kt`

- [ ] **Step 1: 创建 `LiquidMeshBackground.kt`** (全量 Compose Canvas 动态漂浮流体极光光斑代码)
- [ ] **Step 2: 创建 `GlassBackdropWrapper.kt`** (全量 Kyant0 `Backdrop` 物理折射与渐变高光卡片代码)
- [ ] **Step 3: 编译验证**

Run: `cd /storage/emulated/0/yumi/android-app && ./gradlew assembleDebug`
Expected: BUILD SUCCESSFUL

---

### Task 3: 构建 `ComposeHomeBridge.kt`（支持零 GC 内存抖动的响应式 `HomeUiState`）

**Files:**
- Modify/Create: `android-app/app/src/main/java/com/yumi/bridge/ui/compose/ComposeHomeBridge.kt`

**Interfaces:**
- Consumes: `LiquidControlCenter`, `LiquidCpuDashboard`, `ComposeView`
- Produces: 响应式 UI 状态管理与 `updateHomeScreenState` API

- [ ] **Step 1: 创建 `ComposeHomeBridge.kt`**

```kotlin
package com.yumi.bridge.ui.compose

import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.unit.dp
import com.yumi.bridge.ui.theme.YumiTheme

class HomeUiState {
    var currentMode by mutableStateOf("balance")
    var cpuFreqs by mutableStateOf(LongArray(8))
    var cpuUsages by mutableStateOf(IntArray(8))
    var ramPercent by mutableStateOf(0)
    var ramDetailText by mutableStateOf("已用 0.0G / 0.0G")
    var uptimeText by mutableStateOf("00:00:00")
}

private val globalHomeState = HomeUiState()

@Composable
fun HomeScreen(
    state: HomeUiState,
    onModeSelected: (String) -> Unit
) {
    YumiTheme {
        LiquidMeshBackground {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp)
            ) {
                LiquidControlCenter(
                    currentMode = state.currentMode,
                    onModeSelected = onModeSelected
                )
                Spacer(modifier = Modifier.height(16.dp))
                LiquidCpuDashboard(
                    cpuFreqs = state.cpuFreqs,
                    cpuUsages = state.cpuUsages,
                    ramPercent = state.ramPercent,
                    ramDetailText = state.ramDetailText,
                    uptimeText = state.uptimeText
                )
            }
        }
    }
}

fun attachHomeScreen(
    composeView: ComposeView,
    onModeSelectedListener: OnModeSelectedListener
) {
    composeView.setContent {
        HomeScreen(
            state = globalHomeState,
            onModeSelected = { mode -> onModeSelectedListener.onModeSelected(mode) }
        )
    }
}

fun updateHomeScreenState(
    currentMode: String,
    cpuFreqs: LongArray,
    cpuUsages: IntArray,
    ramPercent: Int,
    ramDetailText: String,
    uptimeText: String
) {
    globalHomeState.currentMode = currentMode
    globalHomeState.cpuFreqs = cpuFreqs
    globalHomeState.cpuUsages = cpuUsages
    globalHomeState.ramPercent = ramPercent
    globalHomeState.ramDetailText = ramDetailText
    globalHomeState.uptimeText = uptimeText
}

fun interface OnModeSelectedListener {
    fun onModeSelected(mode: String)
}
```

- [ ] **Step 2: 编译验证**

Run: `cd /storage/emulated/0/yumi/android-app && ./gradlew assembleDebug`
Expected: BUILD SUCCESSFUL

---

### Task 4: 修改 `MainActivity.java` 修复主线程 ANR 阻塞与 Compose 状态更新

**Files:**
- Modify: `android-app/app/src/main/res/layout/activity_main.xml`
- Modify: `android-app/app/src/main/java/com/yumi/bridge/MainActivity.java`

- [ ] **Step 1: 修改 `activity_main.xml` 嵌套结构**

取消外层 `ScrollView` 对 `ComposeView` 的包裹，防止手势滑动冲突。

- [ ] **Step 2: 修改 `MainActivity.java` 剥离 IO 至后台线程并接入响应式 `updateHomeScreenState`**

```java
package com.yumi.bridge;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import androidx.compose.ui.platform.ComposeView;
import com.yumi.bridge.ui.compose.ComposeHomeBridgeKt;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class MainActivity extends Activity {
    private ComposeView composeHomeHost;

    // 线程池：专门处理 su /proc/stat 后台 IO 采样，彻底防御主线程 ANR 卡死
    private final ExecutorService backgroundIoExecutor = Executors.newSingleThreadExecutor();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    private final Runnable logPollRunnable = new Runnable() {
        @Override
        public void run() {
            // 在后台子线程中执行 su 和 IO
            backgroundIoExecutor.execute(() -> {
                int[] usagePercents = new int[8];
                long[] curFreqs = new long[8];
                readCpuStatsAndFreqs(usagePercents, curFreqs);

                // 采样完毕后，回到主线程更新 Compose 响应式 State
                mainHandler.post(() -> {
                    updateSystemDashboardInfo();
                    ComposeHomeBridgeKt.updateHomeScreenState(
                        currentMode,
                        curFreqs,
                        usagePercents,
                        currentRamPercent,
                        currentRamDetailText,
                        currentUptimeText
                    );
                });
            });

            mainHandler.postDelayed(this, 1000);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        composeHomeHost = findViewById(R.id.composeHomeHost);
        if (composeHomeHost != null) {
            // 初始化 Compose 视图结构（仅调用一次！）
            ComposeHomeBridgeKt.attachHomeScreen(composeHomeHost, this::setGlobalMode);
        }

        mainHandler.postDelayed(logPollRunnable, 1000);
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        backgroundIoExecutor.shutdown();
    }
}
```

- [ ] **Step 3: 执行全量打包验证**

Run: `cd /storage/emulated/0/yumi/android-app && ./gradlew clean assembleRelease`
Expected: BUILD SUCCESSFUL with `app-release-unsigned.apk` generated.
