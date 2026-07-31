# yumi 编译与打包快速指南

## 快速开始

### 1. 克隆项目
```bash
git clone https://github.com/imacte/yumi.git
cd yumi
```

### 2. 一键编译并打包
```bash
chmod +x build_and_pack.sh
./build_and_pack.sh
```

### 3. 获取模块
编译完成后，在 `output` 目录中找到生成的 zip 文件：
```bash
ls output/yumi-*.zip
```

## 手动编译步骤

如果一键脚本失败，可以手动执行以下步骤：

### 步骤 1: 安装依赖
```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 nightly 工具链
rustup toolchain install nightly
rustup default nightly

# 添加 Android 目标
rustup target add aarch64-linux-android

# 安装 cargo-ndk
cargo install cargo-ndk

# 设置 Android NDK 环境变量
export ANDROID_NDK_HOME=/path/to/android-ndk
```

### 步骤 2: 编译 Rust 核心
```bash
export RUSTFLAGS="-C default-linker-libraries"
cargo +nightly ndk --platform 26 -t arm64-v8a build -Z build-std -r
```

### 步骤 3: 编译 WebUI
```bash
cd webui
npm install
npm run build
cd ..
```

### 步骤 4: 打包模块
```bash
chmod +x pack_module.sh
./pack_module.sh
```

## 安装到手机

1. 将 `output/yumi-*.zip` 传输到手机
2. 打开 KernelSU 管理器
3. 选择「从本地安装」
4. 选择 zip 文件
5. 等待安装完成
6. 重启手机

## 验证安装

```bash
# 检查模块是否激活
adb shell ls /data/adb/modules/yumi

# 检查服务是否运行
adb shell ps -A | grep yumi

# 查看日志
adb shell cat /data/adb/modules/yumi/logs/service.log
```

## 常见问题

**Q: 编译失败怎么办？**
A: 确保在本地文件系统（如 /home）中编译，不要在 sdcard 或网络挂载目录中编译。

**Q: 找不到 Android NDK 怎么办？**
A: 从 https://developer.android.com/ndk/downloads 下载并设置 ANDROID_NDK_HOME 环境变量。

**Q: 安装后服务没有启动怎么办？**
A: 检查日志文件 `/data/adb/modules/yumi/logs/service.log`，查看具体错误信息。

## 文件说明

- `worklog.md` - 省电优化修改记录
- `CLAUDE.md` - 项目规则和持久化记忆
- `pack_module.sh` - 打包脚本
- `build_and_pack.sh` - 编译并打包脚本
- `BUILD_INSTRUCTIONS.md` - 详细编译说明
- `powersave_optimization.md` - 省电优化方案文档

---

*最后更新: 2026-07-31*
