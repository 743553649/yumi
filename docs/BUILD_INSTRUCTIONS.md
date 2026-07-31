# yumi 编译与打包指南

## 环境要求

### 1. 操作系统
- Linux (推荐 Ubuntu 20.04+)
- macOS (需要额外配置)
- Windows (需要 WSL2)

### 2. Rust 工具链
```bash
# 安装 rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 nightly 工具链
rustup toolchain install nightly
rustup default nightly

# 添加 Android 编译目标
rustup target add aarch64-linux-android
```

### 3. cargo-ndk
```bash
cargo install cargo-ndk
```

### 4. Android NDK
```bash
# 下载 Android NDK (推荐 r25c 或更高版本)
# https://developer.android.com/ndk/downloads

# 设置环境变量
export ANDROID_NDK_HOME=/path/to/android-ndk
export PATH=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH
```

### 5. Node.js 和 npm (用于编译 WebUI)
```bash
# 安装 Node.js 18+ (推荐使用 nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18
```

## 编译步骤

### 方式一：使用编译脚本 (推荐)
```bash
# 给脚本添加执行权限
chmod +x build_and_pack.sh

# 运行编译并打包脚本
./build_and_pack.sh
```

### 方式二：手动编译

#### 1. 编译 Rust 核心
```bash
# 设置编译标志
export RUSTFLAGS="-C default-linker-libraries"

# 编译 (release 模式)
cargo +nightly ndk --platform 26 -t arm64-v8a build -Z build-std -r
```

#### 2. 编译 WebUI
```bash
cd webui

# 安装依赖
npm install

# 编译
npm run build

cd ..
```

#### 3. 打包 KernelSU 模块
```bash
# 运行打包脚本
chmod +x pack_module.sh
./pack_module.sh
```

## 输出文件

编译完成后，会在 `output` 目录生成以下文件：

```
output/
├── yumi-v2.0.1-<git_count>-<date>.zip  # KernelSU 模块
└── .temp/                               # 临时目录 (自动清理)
```

## 模块内容

打包的 KernelSU 模块包含以下文件：

```
yumi-*.zip
├── META-INF/
│   └── com/google/android/
│       ├── update-binary              # 模块安装脚本
│       └── updater-script             # Magisk 安装标记
├── config/
│   ├── config.yaml                    # 主配置文件
│   └── i18n/
│       ├── en.ftl                     # 英文语言包
│       └── zh.ftl                     # 中文语言包
├── core/
│   └── bin/
│       └── yumi                       # 核心二进制文件 (新编译)
├── scripts/
│   └── disable_boost.sh               # 禁用 boost 脚本
├── webroot/
│   ├── index.html                     # WebUI 入口
│   ├── assets/
│   │   ├── index-*.js                 # JavaScript 文件
│   │   └── index-*.css                # CSS 文件
│   └── favicon.ico                    # 网站图标
├── module.prop                        # 模块属性
├── rules.yaml                         # 调度规则配置
├── service.sh                         # 启动脚本
├── customize.sh                       # 安装脚本
└── uninstall.sh                       # 卸载脚本
```

## 省电优化修改

新编译的二进制文件包含以下 8 项省电优化：

### CLG 负载调速器优化
1. **significant_jump 阈值**: 0.35 → 0.50
2. **低负载降频阈值**: 0.10 → 0.15
3. **低负载降频倍数**: 2.5 → 3.0

### FAS 帧感知调度优化
4. **PID util_gain 范围**: 0.30 → 0.45
5. **target_fps 偏移**: 更激进的偏移策略
6. **快速衰减**: 高刷下更激进的衰减
7. **EMA 升频平滑**: 降低升频敏感度
8. **升档门槛**: 提高升档触发条件

### Doze 息屏模式优化
9. **perf_ceil**: 0.40 → 0.30
10. **smoothing_up**: 0.10 → 0.05
11. **up_rate_limit_ticks**: 3 → 5

## 常见问题

### Q1: 编译失败，提示权限错误
**A**: 确保在本地文件系统（如 /home）中编译，不要在 sdcard 或网络挂载的目录中编译。

### Q2: 找不到 Android NDK
**A**: 下载 Android NDK 并设置 ANDROID_NDK_HOME 环境变量：
```bash
export ANDROID_NDK_HOME=/path/to/android-ndk-r25c
```

### Q3: nightly 工具链安装失败
**A**: 尝试使用国内镜像：
```bash
export RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static
export RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
rustup toolchain install nightly
```

### Q4: cargo-ndk 安装失败
**A**: 确保安装了必要的系统依赖：
```bash
# Ubuntu/Debian
sudo apt-get install -y build-essential pkg-config libssl-dev

# 安装 cargo-ndk
cargo install cargo-ndk
```

### Q5: WebUI 编译失败
**A**: 确保 Node.js 版本 >= 18：
```bash
node --version
npm --version

# 如果版本过低，使用 nvm 升级
nvm install 18
nvm use 18
```

## 安装方法

1. 将 `output/yumi-*.zip` 传输到手机
2. 在 KernelSU 管理器中选择「从本地安装」
3. 选择该 zip 文件并安装
4. 重启后自动生效

## 验证安装

安装后，可以通过以下方式验证：

1. **检查模块是否激活**
   - 打开 KernelSU 管理器
   - 查看模块列表中是否有 yumi

2. **检查服务是否运行**
   ```bash
   adb shell ps -A | grep yumi
   ```

3. **查看日志**
   ```bash
   adb shell cat /data/adb/modules/yumi/logs/service.log
   ```

4. **访问 WebUI**
   - 在浏览器中访问: `http://localhost:9090` (需要端口转发)
   - 或者使用 KernelSU 的 WebUI 功能

## 技术支持

如有问题，请提供以下信息：
- 设备型号和 Android 版本
- KernelSU 版本
- 编译环境信息 (`rustc --version`, `cargo --version`)
- 错误日志

---

*最后更新: 2026-07-31*
*yumi v2.0.1*
