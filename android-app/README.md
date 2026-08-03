# yumi Bridge Android 控制端 App

**yumi Bridge** 是 `yumi` CPU 调度器守护进程的独立 Android 沉浸式控制中心应用，采用极简 **iOS 26 冰雪白毛玻璃 (Light Glassmorphism)** 视觉规范开发。

---

## 🌟 核心特性与架构

### 1. 双渲染引擎混合架构 (Hybrid Dual-Engine UI)
- **Jetpack Compose 流体 UI**：包含 `LiquidControlCenter.kt`、`LiquidCpuDashboard.kt`（CPU 仪表盘）与 `LiquidMeshBackground.kt`（流体渐变背景）。
- **原生 Java/XML 沉浸视图**：底层由 `MainActivity.java` (ComponentActivity) 管理，配合 `GlassCardView`（支持 API 31+ RenderEffect 高斯模糊与硬件抗锯齿）与 `CpuCircleProgressView`。

### 2. 3-Tab 沉浸导航与功能控制
- **TAB 0 - 首页控制台**：显示守护进程 TCP 连接状态、多核 CPU 利用率/频率实时仪表盘、极简毛玻璃卡片与 5 种模式动态切换。
- **TAB 1 - 运行日志终端**：5 级中文日志实时分级过滤（全部、调试、信息、警告、错误），支持物理 Root `su -c` 通道实时读取 `daemon.log` 流。
- **TAB 2 - 应用规则管理**：支持已安装应用模糊搜索，实时读取与回写 `module/rules.yaml`，为特定应用配置 `powersave` / `balance` / `performance` / `fast` / `fas` 专属调度模式。

---

## 🏗️ 代码结构

```
android-app/app/src/main/
├── java/com/yumi/bridge/
│   ├── MainActivity.java                # 主控 Activity (3-Tab/IPC/应用规则/日志过滤)
│   ├── ui/
│   │   ├── GlassCardView.java           # 真实浅色毛玻璃容器卡片 (硬件模糊与抗锯齿)
│   │   ├── CpuCircleProgressView.java   # 多核 CPU 图表与百分比环形 View
│   │   └── compose/
│   │       ├── LiquidControlCenter.kt   # Compose 流体控制中心
│   │       ├── LiquidCpuDashboard.kt    # Compose CPU 仪表盘
│   │       ├── LiquidMeshBackground.kt   # Compose 流体背景渐变
│   │       └── GlassBackdropWrapper.kt  # Compose 毛玻璃容器封装
│   └── theme/
│       └── YumiTheme.kt                 # Compose 主题 Token 映射
└── res/
    ├── layout/activity_main.xml         # 主界面布局
    ├── values/colors.xml                # 浅色毛玻璃视觉 Token 定义
    └── values/themes.xml                # Edge-to-Edge 全屏沉浸主题
```

---

## 🛠️ 编译与打包指南

由于开发环境包含电脑端与手机端，提供不同的构建途径：

### 1. 💻 电脑端编译（推荐）

在电脑端拥有标准 Java 与 Android SDK 环境：

#### 命令行 (Gradle)
进入 `android-app` 目录运行：
```powershell
# Windows
.\gradlew.bat assembleDebug

# Linux / macOS
./gradlew assembleDebug
```
产物位置：`app/build/outputs/apk/debug/app-debug.apk`

#### IDE (Android Studio)
直接在 Android Studio 中打开 `android-app` 文件夹，点击 **Run (Shift+F10)** 运行至真机或模拟器。

---

### 2. 📱 手机端 (Termux) 编译

手机端脚本为 Termux 环境专属定制：

#### ① 全功能 Compose 版 (Python 自动化)
运行 `build_compose_apk.py`，脚本将自动从 Gradle 缓存中提取 Compose/Kotlin AAR 与 JAR 依赖，合并资源并调用 `aapt2`/`d8` 打包全功能 APK：
```bash
python3 android-app/build_compose_apk.py
```

#### ② 基础 Java 原生版 (Shell)
仅编译纯 Java 原生逻辑的轻量级构建脚本：
```bash
bash android-app/build_apk.sh
```

---

## 📚 详细设计规范

有关浅色毛玻璃 UI 架构设计、Token 映射与抗锯齿物理管道的详细规范，请参阅：
- [iOS 26 极简冰雪白毛玻璃重构设计书](../docs/2026-08-02-android-ui-liquid-glass-refactor.md)
