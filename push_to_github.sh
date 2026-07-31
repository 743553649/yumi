#!/bin/bash
#
# yumi 推送到 GitHub 脚本
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
echo -e "${GREEN}║           yumi 推送到 GitHub 脚本                         ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# 检查是否已配置 Git
if [ -z "$(git config --global user.name)" ] || [ -z "$(git config --global user.email)" ]; then
    echo -e "${YELLOW}[配置] 请先配置 Git 用户信息${NC}"
    echo ""
    read -p "请输入你的名字: " GIT_NAME
    read -p "请输入你的邮箱: " GIT_EMAIL

    git config --global user.name "$GIT_NAME"
    git config --global user.email "$GIT_EMAIL"

    echo -e "${GREEN}✓ Git 用户信息已配置${NC}"
    echo ""
fi

# 显示当前配置
echo -e "${BLUE}[信息] 当前 Git 配置:${NC}"
echo -e "  用户名: $(git config --global user.name)"
echo -e "  邮箱: $(git config --global user.email)"
echo ""

# 询问 GitHub 用户名
read -p "请输入你的 GitHub 用户名: " GITHUB_USERNAME

if [ -z "$GITHUB_USERNAME" ]; then
    echo -e "${RED}错误: GitHub 用户名不能为空${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}[步骤 1] 初始化 Git 仓库...${NC}"

# 检查是否已初始化
if [ ! -d ".git" ]; then
    git init
    echo -e "${GREEN}  ✓ Git 仓库已初始化${NC}"
else
    echo -e "${GREEN}  ✓ Git 仓库已存在${NC}"
fi

echo ""
echo -e "${BLUE}[步骤 2] 添加文件...${NC}"

# 添加所有文件
git add .

# 显示将要提交的文件数量
FILE_COUNT=$(git status --short | wc -l)
echo -e "${GREEN}  ✓ 已添加 ${FILE_COUNT} 个文件${NC}"

echo ""
echo -e "${BLUE}[步骤 3] 提交修改...${NC}"

# 提交
git commit -m "feat: 省电优化 + KernelSU 模块打包工具

- CLG 负载调速器优化 (significant_jump, 低负载降频)
- FAS 帧感知调度优化 (PID util_gain, target_fps 偏移等)
- Doze 息屏模式优化 (perf_ceil, smoothing_up)
- 添加 KernelSU 模块打包脚本
- 添加 GitHub Actions 自动编译配置"

echo -e "${GREEN}  ✓ 已提交${NC}"

echo ""
echo -e "${BLUE}[步骤 4] 添加远程仓库...${NC}"

# 检查是否已添加远程仓库
if git remote get-url origin &>/dev/null; then
    echo -e "${YELLOW}  ! 远程仓库已存在，更新中...${NC}"
    git remote set-url origin "https://github.com/${GITHUB_USERNAME}/yumi.git"
else
    git remote add origin "https://github.com/${GITHUB_USERNAME}/yumi.git"
fi

echo -e "${GREEN}  ✓ 远程仓库: https://github.com/${GITHUB_USERNAME}/yumi.git${NC}"

echo ""
echo -e "${BLUE}[步骤 5] 推送到 GitHub...${NC}"
echo ""
echo -e "${YELLOW}当提示输入密码时，请输入你的 Personal Access Token${NC}"
echo -e "${YELLOW}（不是 GitHub 密码，生成方法见 PUSH_TO_GITHUB.md）${NC}"
echo ""

# 推送
git push -u origin main

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    推送完成！                              ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}下一步:${NC}"
echo -e "  1. 访问 https://github.com/${GITHUB_USERNAME}/yumi"
echo -e "  2. 点击 'Actions' 标签"
echo -e "  3. 启用 workflows"
echo -e "  4. 点击 'Build Yumi' -> 'Run workflow'"
echo -e "  5. 等待编译完成"
echo -e "  6. 下载 Artifacts 中的 zip 文件"
echo ""
echo -e "${GREEN}=== 完成 ===${NC}"
