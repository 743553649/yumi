# 将修改后的 yumi 推送到 GitHub

## 方式一：Fork 原项目后推送修改 (推荐)

### 步骤 1: Fork 原项目

1. 访问原项目：https://github.com/imacte/yumi
2. 点击右上角的 **"Fork"** 按钮
3. 选择你的 GitHub 账号
4. 等待 Fork 完成

### 步骤 2: 生成 GitHub Personal Access Token

1. 访问：https://github.com/settings/tokens
2. 点击 **"Generate new token (classic)"**
3. 填写信息：
   - **Note**: `yumi push token`
   - **Expiration**: 选择过期时间（建议 90 天）
   - **Scopes**: 勾选 **`repo`** (完整权限)
4. 点击 **"Generate token"**
5. **复制并保存 token**（只显示一次）

### 步骤 3: 在手机上配置 Git

```bash
# 配置 Git 用户信息
git config --global user.name "你的名字"
git config --global user.email "你的邮箱@example.com"
```

### 步骤 4: 初始化 Git 仓库并推送

```bash
# 进入项目目录
cd /mnt/sdcard/yumi

# 初始化 Git 仓库（如果还没有）
git init

# 添加所有文件
git add .

# 提交修改
git commit -m "feat: 省电优化 + KernelSU 模块打包工具"

# 添加远程仓库（替换 YOUR_USERNAME 为你的 GitHub 用户名）
git remote add origin https://github.com/YOUR_USERNAME/yumi.git

# 推送到 GitHub（会要求输入用户名和 token）
git push -u origin main
```

当提示输入密码时，**粘贴你的 Personal Access Token**（不是 GitHub 密码）。

### 步骤 5: 触发 GitHub Actions 编译

1. 访问你的 Fork 仓库：`https://github.com/YOUR_USERNAME/yumi`
2. 点击 **"Actions"** 标签
3. 如果提示启用 workflows，点击 **"I understand my workflows, go ahead and enable them"**
4. 点击 **"Build Yumi"**
5. 点击 **"Run workflow"** -> **"Run workflow"**
6. 等待 5-10 分钟编译完成
7. 在 Artifacts 中下载 `yumi-*.zip`

---

## 方式二：创建新仓库直接推送

### 步骤 1: 创建新仓库

1. 访问：https://github.com/new
2. 填写信息：
   - **Repository name**: `yumi`
   - **Description**: `yumi - 智能 CPU 调度控制器 (省电优化版)`
   - **Public** 或 **Private**
   - **不要**勾选 "Add a README file"
3. 点击 **"Create repository"**

### 步骤 2: 推送代码

```bash
# 进入项目目录
cd /mnt/sdcard/yumi

# 初始化 Git 仓库
git init

# 添加所有文件
git add .

# 提交
git commit -m "feat: 省电优化 + KernelSU 模块打包工具"

# 添加远程仓库
git remote add origin https://github.com/YOUR_USERNAME/yumi.git

# 推送
git push -u origin main
```

---

## 常见问题

### Q: 推送时提示 "Permission denied"？

A: 检查以下几点：
- 确保使用的是 Personal Access Token，不是密码
- 确保 Token 有 `repo` 权限
- 确保仓库名正确

### Q: 推送时提示 "Repository not found"？

A: 确保：
- 已经在 GitHub 上创建了仓库
- 远程 URL 中的用户名和仓库名正确

### Q: 如何更新 Fork？

A: 如果原项目有更新，执行：
```bash
# 添加原项目为上游仓库
git remote add upstream https://github.com/imacte/yumi.git

# 拉取原项目更新
git fetch upstream

# 合并更新
git merge upstream/main

# 推送到你的 Fork
git push origin main
```

### Q: GitHub Actions 编译失败？

A: 检查以下几点：
- 确保 Fork 的是最新的代码
- 查看 Actions 日志中的错误信息
- 确保 GitHub 账号有足够的 Actions 额度

### Q: 如何获取编译好的模块？

A: 两种方式：
1. **Actions Artifacts**: 编译完成后，在 Actions 运行页面的 Artifacts 部分下载
2. **Releases**: 如果推送了 tag，会自动创建 Release

---

## 快速命令汇总

```bash
# 1. 配置 Git
git config --global user.name "你的名字"
git config --global user.email "你的邮箱@example.com"

# 2. 初始化并提交
cd /mnt/sdcard/yumi
git init
git add .
git commit -m "feat: 省电优化 + KernelSU 模块打包工具"

# 3. 推送到 GitHub (替换 YOUR_USERNAME)
git remote add origin https://github.com/YOUR_USERNAME/yumi.git
git push -u origin main
```

---

## 验证推送成功

1. 访问 `https://github.com/YOUR_USERNAME/yumi`
2. 确认文件已上传
3. 检查 Actions 是否自动触发编译
4. 等待编译完成并下载 zip 文件

---

*最后更新: 2026-07-31*
