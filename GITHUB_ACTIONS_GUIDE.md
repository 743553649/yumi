# 使用 GitHub Actions 编译 yumi

由于 Android 手机环境限制，推荐使用 GitHub Actions 免费编译。

## 快速开始

### 方式一：Fork 项目自动编译

1. **Fork 项目**
   - 访问 https://github.com/imacte/yumi
   - 点击右上角的 "Fork" 按钮
   - 选择你的 GitHub 账号

2. **启用 GitHub Actions**
   - 进入你 Fork 的项目
   - 点击 "Actions" 标签
   - 点击 "I understand my workflows, go ahead and enable them"

3. **触发编译**
   - 方式 A：直接推送代码到 main 分支
   - 方式 B：在 Actions 页面手动运行
     - 点击 "Build Yumi"
     - 点击 "Run workflow"
     - 选择 main 分支
     - 点击 "Run workflow" 按钮

4. **下载编译结果**
   - 等待编译完成（约 5-10 分钟）
   - 在 Actions 页面点击最新的 workflow 运行
   - 在 "Artifacts" 部分下载 `yumi-*.zip`

### 方式二：发布 Release 自动编译

1. **创建 Tag**
   ```bash
   git tag v2.0.1
   git push origin v2.0.1
   ```

2. **自动触发**
   - 推送 tag 后会自动触发编译
   - 编译完成后会自动创建 Release
   - 在 Releases 页面下载 zip 文件

## 编译流程说明

GitHub Actions 会自动执行以下步骤：

1. ✅ 拉取代码
2. ✅ 设置 Node.js 环境
3. ✅ 安装 WebUI 依赖
4. ✅ 设置 Rust Nightly 工具链
5. ✅ 安装 Android NDK
6. ✅ 安装 cargo-ndk
7. ✅ 编译 WebUI
8. ✅ 编译 Rust 核心（包含省电优化）
9. ✅ 打包 KernelSU 模块
10. ✅ 上传编译产物

## 编译产物

编译完成后，会生成以下文件：

```
yumi-v2.0.1-<commit_count>-<date>.zip
```

### 模块内容

```
yumi-*.zip
├── META-INF/              # 模块安装文件
├── config/                # 配置文件
│   ├── config.yaml
│   └── i18n/
├── core/bin/yumi          # 核心二进制文件 (新编译)
├── scripts/               # 脚本文件
├── webroot/               # WebUI 界面
├── module.prop            # 模块属性
├── rules.yaml             # 调度规则
├── service.sh             # 启动脚本
├── customize.sh           # 安装脚本
└── uninstall.sh           # 卸载脚本
```

## 安装到手机

1. **下载 zip 文件**
   - 从 GitHub Actions Artifacts 下载
   - 或从 Releases 页面下载

2. **传输到手机**
   - 使用数据线
   - 或使用云存储/即时通讯工具

3. **安装模块**
   - 打开 KernelSU 管理器
   - 点击 "模块" 标签
   - 点击 "从本地安装"
   - 选择下载的 zip 文件
   - 等待安装完成
   - 重启手机

## 验证安装

```bash
# 检查模块是否安装成功
adb shell ls /data/adb/modules/yumi

# 检查服务是否运行
adb shell ps -A | grep yumi

# 查看日志
adb shell cat /data/adb/modules/yumi/logs/service.log
```

## 常见问题

### Q: Actions 编译失败怎么办？

A: 检查以下几点：
- 确保 Fork 的是最新的代码
- 查看 Actions 日志中的错误信息
- 确保 GitHub 账号有足够的 Actions 额度

### Q: 编译需要多长时间？

A: 通常需要 5-10 分钟，取决于：
- 依赖下载速度
- 编译服务器性能
- 网络状况

### Q: GitHub Actions 免费吗？

A: 是的，GitHub Actions 对公开仓库免费：
- 每月 2000 分钟免费额度
- 公开仓库不计入额度
- 私有仓库有额度限制

### Q: 如何获取最新的编译版本？

A: 两种方式：
1. 定期 Fork 最新代码并手动触发编译
2. 关注原项目的 Releases 页面

### Q: 编译的二进制文件包含省电优化吗？

A: 是的，编译会自动包含源码中的所有修改，包括：
- CLG 负载调速器优化
- FAS 帧感知调度优化
- Doze 息屏模式优化

## 自定义编译

如果你想修改代码后重新编译：

1. **修改代码**
   - 在你的 Fork 中修改代码
   - 提交并推送到 main 分支

2. **自动触发编译**
   - 推送后会自动触发 Actions
   - 或手动触发 workflow

3. **获取结果**
   - 在 Actions 页面下载编译产物

## 技术支持

如有问题，请提供：
- GitHub Actions 运行日志
- 错误信息截图
- 设备型号和 Android 版本

---

*最后更新: 2026-07-31*
*yumi v2.0.1*
