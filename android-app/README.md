# yumi-bridge (Android TCP 控制端)

对应 `yumi` 调度器守护进程的独立 Android 测试 App。

## 方案说明
- 基于纯标准库 `java.net.Socket` 实现 TCP Loopback 通信（`127.0.0.1:14567`）
- 界面包含 Ping 测试、当前模式查询与 4 种性能模式（powersave/balance/performance/fast）实时切换

## 编译与运行
直接在 Android Studio 中打开 `android-app` 目录，连接设备或模拟器点击 Run 即可运行。
