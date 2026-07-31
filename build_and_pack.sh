#!/bin/bash
#
# yumi 编译并打包脚本
# 需要完整的 Rust 编译环境和 Android NDK
#

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_ROOT"

echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║           yumi 编译并打包 KernelSU 模块                   ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# 检查编译环境
echo -e "${BLUE}[检查] 验证编译环境...${NC}"

# 检查 Rust
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}错误: 未安装 Rust${NC}"
    echo -e "${YELLOW}请先安装: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    exit 1
fi
echo -e "${GREEN}  ✓ Rust $(rustc --version)${NC}"

# 检查 nightly 工具链
if ! rustup toolchain list | grep -q "nightly"; then
    echo -e "${YELLOW}  ! 未安装 nightly 工具链，正在安装...${NC}"
    rustup toolchain install nightly
fi
echo -e "${GREEN}  ✓ nightly 工具链${NC}"

# 检查 cargo-ndk
if ! command -v cargo-ndk &> /dev/null; then
    echo -e "${YELLOW}  ! 未安装 cargo-ndk，正在安装...${NC}"
    cargo install cargo-ndk
fi
echo -e "${GREEN}  ✓ cargo-ndk${NC}"

# 检查 Android NDK
if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$ANDROID_NDK_ROOT" ]; then
    echo -e "${YELLOW}  ! 未设置 ANDROID_NDK_HOME 或 ANDROID_NDK_ROOT${NC}"
    echo -e "${YELLOW}    请设置 Android NDK 路径，例如:${NC}"
    echo -e "${YELLOW}    export ANDROID_NDK_HOME=/path/to/android-ndk${NC}"
    echo -e "${YELLOW}    或者安装 Android NDK 并设置环境变量${NC}"
fi
echo -e "${GREEN}  ✓ Android NDK 环境${NC}"

echo ""
echo -e "${BLUE}[编译] 开始编译项目...${NC}"

# 1. 编译 Rust 核心 (eBPF + 主程序)
echo -e "${YELLOW}  [1/2] 编译 Rust 核心...${NC}"
RUSTFLAGS="-C default-linker-libraries" cargo +nightly ndk --platform 26 -t arm64-v8a build -Z build-std -r

if [ $? -ne 0 ]; then
    echo -e "${RED}错误: Rust 核心编译失败${NC}"
    exit 1
fi
echo -e "${GREEN}        ✓ 编译成功${NC}"

# 2. 编译 WebUI
echo -e "${YELLOW}  [2/2] 编译 WebUI...${NC}"
if [ -d "webui" ] && [ -f "webui/package.json" ]; then
    cd webui
    npm install
    npm run build
    cd ..
    echo -e "${GREEN}        ✓ 编译成功${NC}"
else
    echo -e "${YELLOW}        ! 未找到 WebUI 源码，跳过编译${NC}"
fi

echo ""
echo -e "${BLUE}[打包] 开始打包模块...${NC}"

# 运行打包脚本
bash pack_module.sh

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    编译打包完成！                          ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}新编译的二进制文件已包含所有省电优化修改:${NC}"
echo -e "  - CLG 负载调速器优化 (significant_jump, 低负载降频)"
echo -e "  - FAS 帧感知调度优化 (PID util_gain, target_fps 偏移等)"
echo -e "  - Doze 息屏模式优化 (perf_ceil, smoothing_up)"
echo ""
echo -e "${GREEN}=== 完成 ===${NC}"
