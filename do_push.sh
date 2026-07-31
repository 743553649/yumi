#!/data/data/com.termux/files/usr/bin/bash
#
# 一键推送到 GitHub
#

cd /mnt/sdcard/yumi

echo "=== 推送到 GitHub ==="
echo ""
echo "请输入你的 Personal Access Token:"
echo "(在 https://github.com/settings/tokens 生成)"
echo ""

# 读取 token
read TOKEN

if [ -z "$TOKEN" ]; then
    echo "错误: Token 不能为空"
    exit 1
fi

echo ""
echo "正在推送..."

# 设置带 token 的远程 URL
git remote set-url origin "https://743553649:${TOKEN}@github.com/743553649/yumi.git"

# 推送
git push -u origin main

if [ $? -eq 0 ]; then
    echo ""
    echo "✓ 推送成功！"
    echo ""
    echo "下一步:"
    echo "1. 访问 https://github.com/743553649/yumi"
    echo "2. 点击 Actions 标签"
    echo "3. 运行 Build Yumi workflow"
    echo "4. 下载编译好的 zip 文件"
else
    echo ""
    echo "✗ 推送失败，请检查 Token 是否正确"
fi
