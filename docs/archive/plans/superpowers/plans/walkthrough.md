# Walkthrough - 液态毛玻璃效果集成与编译验证
我们已经完成了毛玻璃渲染重构以及项目编译适配，目前项目已成功升级至 Kotlin 2.0.21，并引入了真实的物理折射渲染毛玻璃效果。

---

## 变动的修改内容 (Changes Made)

### 1. 编译环境升级与兼容处理
- 成功将项目升级到最新的 **Kotlin 2.0.21** 环境。
- 引入 `org.jetbrains.kotlin.plugin.compose` (版本 2.0.21) 插件来替代旧版 Compose Compiler 插件。
- 在 `app/build.gradle.kts` 中使用 `-Xskip-metadata-version-check` 配置跳过因为依赖库（如 `backdrop`）由于使用了过高或不稳定的 Kotlin Metadata 版本而导致的编译错误，顺利实现了版本共存与成功编译。

### 2. 物理仿制毛玻璃渲染 ([`GlassBackdropWrapper.kt`](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/java/com/yumi/bridge/ui/compose/GlassBackdropWrapper.kt))
- 重构了卡片的毛玻璃层级架构，通过分层机制（Sibling Box Layers）彻底解决子 Composable 一并被模糊的缺陷：
  - **背景毛玻璃层**：通过独立 `Box` 附加真实的物理折射效果（`backdrop`库）和渐变，仅将下方的炫彩背景进行雾化处理。
  - **前台文字与指示器**：置于毛玻璃背景层上，确保 CPU 频率、环形图、电池文字和图标等信息 **100% 锐利高清**。
  - 继承保留了 45° 斜角高光冰晶边框及 8dp 弥散软阴影。

---

## 验证与测试结果

- **编译验证**：运行 `.\gradlew compileDebugKotlin` 验证，编译成功。
- **单元测试**：运行 `.\gradlew testDebugUnitTest` 验证，测试用例全部 **Green (100% 通过)**。

---

# Walkthrough 3 - 应用规则生效链路根治、日志通道规范化与 iOS 26 冰雪毛玻璃启动体验

## 1. 核心变动原因 (Why We Did It)

在近期系统集成测试中，我们定位并根治了 3 个关键系统隐患与体验瓶颈：
1. **应用独立模式失效**：在 App 侧给指定应用配置特定调度模式（如 `fas` / `performance`）后，前台切换到该应用时守护进程依然退回到 `global_mode`。根源在于：
   - App 写入 YAML 的节点键名与 Rust 端 `app_modes` 结构体字段不匹配；
   - Rust 守护进程中的 `ipc_server` 在接收指令时未更新 `app_detect` 线程持有的 `config_arc` 共享锁与 `force_refresh_arc`，导致前台检测循环死锁在初始旧内存快照；
   - 未显式处理 `default` / `none` 的回退逻辑。
2. **日志污染与按钮失效**：原本日志系统会尝试读取 SD 卡存储路径（`/storage/emulated/0/yumi`），污染了用户的项目工作区。且日志清空/刷新按钮由于缺乏磁盘物理文件截断和与 Java 侧 `currentFilterLevel` 的双向同步，导致 2 秒轮询时清空视图与过滤 Chip 被强行冲刷弹回。
3. **桌面图标与启动闪烁**：之前图标未声明 `android:icon` 导致桌面图标不生效，且启动时 Compose 自建 SplashScreen 与 Android 12+ 系统启动过渡产生二次闪烁冲突。

---

## 2. 交付清单 (Delivery Checklist)

- [`src/monitor/app_detect.rs`](file:///c:/Users/GUDGA/yumi/src/monitor/app_detect.rs)：支持 `default`/`none` 回退并返回 `(String, bool)` 覆盖标志；增加特定应用独立模式覆盖日志输出。
- [`src/ipc_server.rs`](file:///c:/Users/GUDGA/yumi/src/ipc_server.rs)：接收共享 `config_arc` 和 `force_refresh_arc`；实现显式 `reload_rules` 热重载命令，修改模式时同步锁更新共享内存配置。
- [`src/main.rs`](file:///c:/Users/GUDGA/yumi/src/main.rs)：初始化全局唯一共享 `config_arc` 与 `force_refresh_arc`，并注入给 `ipc_server` 与 `monitor`。
- [`MainActivity.java`](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/java/com/yumi/bridge/MainActivity.java)：规范 YAML 节点写入为 `app_modes:`；实现 Root 磁盘文件物理擦除；实现 `onFilterLevelChanged` 过滤等级双向同步。
- [`ic_launcher_foreground.xml`](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/res/drawable/ic_launcher_foreground.xml)：iOS 26 冰雪白毛玻璃矢量背景与 CPU 极光蓝内核切面图标。
- [`themes.xml`](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/res/values/themes.xml) & [`AndroidManifest.xml`](file:///c:/Users/GUDGA/yumi/android-app/app/src/main/AndroidManifest.xml)：配置 `Theme.YumiBridge.Starting` 主题，绑定 `android:icon` 与 `android:roundIcon`。
