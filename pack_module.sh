#!/bin/bash
#
# yumi KernelSU 模块打包脚本
# 用于将编译好的文件打包成 KernelSU 模块
#
# 使用方法:
#   ./pack_module.sh          # 使用现有编译产物打包
#   ./pack_module.sh --build  # 先编译再打包
#

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_ROOT"

echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║           yumi KernelSU 模块打包脚本                      ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# 解析参数
BUILD_FIRST=false
for arg in "$@"; do
    case $arg in
        --build)
            BUILD_FIRST=true
            shift
            ;;
        --help|-h)
            echo "使用方法:"
            echo "  ./pack_module.sh          # 使用现有编译产物打包"
            echo "  ./pack_module.sh --build  # 先编译再打包"
            echo "  ./pack_module.sh --help   # 显示帮助"
            exit 0
            ;;
    esac
done

# 如果需要先编译
if [ "$BUILD_FIRST" = true ]; then
    echo -e "${YELLOW}[1/3] 编译 Rust 核心...${NC}"
    RUSTFLAGS="-C default-linker-libraries" cargo +nightly ndk --platform 26 -t arm64-v8a build -Z build-std -r

    echo -e "${YELLOW}[2/3] 编译 WebUI...${NC}"
    cd webui && npm run build && cd ..
fi

# 检查必要文件
echo -e "${BLUE}[检查] 验证必要文件...${NC}"

# 检查二进制文件
BINARY_PATH=""
if [ -f "target/aarch64-linux-android/release/yumi" ]; then
    BINARY_PATH="target/aarch64-linux-android/release/yumi"
    echo -e "${GREEN}  ✓ 找到编译产物: $BINARY_PATH${NC}"
elif [ -f "output/yumi-glassmorphism.zip" ]; then
    echo -e "${YELLOW}  ! 未找到编译产物，将从现有 zip 中提取二进制文件${NC}"
    TEMP_EXTRACT="output/.temp_extract"
    rm -rf "$TEMP_EXTRACT"
    mkdir -p "$TEMP_EXTRACT"
    unzip -q "output/yumi-glassmorphism.zip" "core/bin/yumi" -d "$TEMP_EXTRACT"
    BINARY_PATH="$TEMP_EXTRACT/core/bin/yumi"
    echo -e "${GREEN}  ✓ 从现有 zip 提取二进制文件${NC}"
else
    echo -e "${RED}  ✗ 错误: 未找到二进制文件${NC}"
    echo -e "${YELLOW}    请先编译项目: cargo +nightly ndk --platform 26 -t arm64-v8a build -Z build-std -r${NC}"
    echo -e "${YELLOW}    或者使用 ./pack_module.sh --build 自动编译打包${NC}"
    exit 1
fi

# 检查 WebUI 文件
if [ -d "webui/dist" ] && [ -f "webui/dist/index.html" ]; then
    echo -e "${GREEN}  ✓ 找到 WebUI 构建目录${NC}"
    USE_WEBUI_DIST=true
elif [ -f "output/yumi-glassmorphism.zip" ]; then
    echo -e "${YELLOW}  ! 未找到 WebUI 构建目录，将从现有 zip 中提取${NC}"
    USE_WEBUI_DIST=false
else
    echo -e "${RED}  ✗ 错误: 未找到 WebUI 文件${NC}"
    echo -e "${YELLOW}    请先编译 WebUI: cd webui && npm run build${NC}"
    exit 1
fi

# 检查 module 目录
if [ ! -d "module" ]; then
    echo -e "${RED}  ✗ 错误: 未找到 module 目录${NC}"
    exit 1
fi
echo -e "${GREEN}  ✓ 找到模块配置目录${NC}"

echo ""
echo -e "${BLUE}[打包] 开始组装模块...${NC}"

# 创建临时目录
TEMP_DIR="output/.temp"
rm -rf "$TEMP_DIR"
mkdir -p "$TEMP_DIR"

# 1. 拷贝 module 目录内容
echo -e "${YELLOW}  [1/5] 拷贝模块配置文件...${NC}"
cp -r module/* "$TEMP_DIR/"
rm -f "$TEMP_DIR/.gitignore"

# 2. 创建 core/bin 目录并拷贝二进制文件
echo -e "${YELLOW}  [2/5] 拷贝二进制文件...${NC}"
mkdir -p "$TEMP_DIR/core/bin"
cp "$BINARY_PATH" "$TEMP_DIR/core/bin/yumi"
chmod 755 "$TEMP_DIR/core/bin/yumi"
echo -e "${GREEN}        -> core/bin/yumi ($(du -h "$TEMP_DIR/core/bin/yumi" | cut -f1))${NC}"

# 3. 拷贝 WebUI 文件
echo -e "${YELLOW}  [3/5] 拷贝 WebUI 文件...${NC}"
mkdir -p "$TEMP_DIR/webroot"
if [ "$USE_WEBUI_DIST" = true ]; then
    cp -r webui/dist/* "$TEMP_DIR/webroot/"
else
    unzip -q "output/yumi-glassmorphism.zip" "webroot/*" -d "$TEMP_EXTRACT"
    cp -r "$TEMP_EXTRACT/webroot/"* "$TEMP_DIR/webroot/"
fi
echo -e "${GREEN}        -> webroot/ ($(find "$TEMP_DIR/webroot" -type f | wc -l) 文件)${NC}"

# 4. 读取版本信息
VERSION=$(grep "^version=" module/module.prop | cut -d'=' -f2)
GIT_COUNT=$(git rev-list --count HEAD 2>/dev/null || echo "0")
DATE=$(date +%Y%m%d-%H%M)

# 5. 创建 zip 文件
ZIP_NAME="yumi-${VERSION}-${GIT_COUNT}-${DATE}.zip"
ZIP_PATH="output/$ZIP_NAME"

echo -e "${YELLOW}  [4/5] 打包 ZIP 文件...${NC}"
cd "$TEMP_DIR"
zip -r "../../$ZIP_PATH" . -x "*.gitignore" > /dev/null
cd "$PROJECT_ROOT"
echo -e "${GREEN}        -> $ZIP_PATH ($(du -h "$ZIP_PATH" | cut -f1))${NC}"

# 6. 清理临时目录
echo -e "${YELLOW}  [5/5] 清理临时文件...${NC}"
rm -rf "$TEMP_DIR"
rm -rf "$TEMP_EXTRACT"

# 7. 显示结果
echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    打包完成！                              ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}输出文件:${NC} $ZIP_PATH"
echo -e "${BLUE}文件大小:${NC} $(du -h "$ZIP_PATH" | cut -f1)"
echo ""
echo -e "${BLUE}模块内容:${NC}"
unzip -l "$ZIP_PATH" | grep -E "^  Length|----|\.sh$|\.prop$|\.yaml$|\.html$|\.js$|\.css$|yumi$" | head -20
echo ""
echo -e "${BLUE}安装方法:${NC}"
echo -e "  1. 将 ${ZIP_NAME} 传输到手机"
echo -e "  2. 在 KernelSU 管理器中选择「从本地安装」"
echo -e "  3. 选择该 zip 文件并安装"
echo -e "  4. 重启后自动生效"
echo ""
echo -e "${GREEN}=== 完成 ===${NC}"
