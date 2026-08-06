# yumi 项目 AI Agent 协作与开发规范

> 本文档为 `yumi` (Android CPU 调度器与 App) 核心开发规范，Agent 在所有开发过程中必须严格遵守。

---

## 1. 架构设计与工程演进原则
- **不保留向后兼容**：过时的代码、废弃的 IPC 接口与配置字段直接删除，严禁兼容层、migration 或 fallback。
- **极简实现与拒绝预防性抽象**：选择满足当前需求的最简实现，绝不搞推测性抽象与多余配置层。
- **端到端先行与渐进演进**：先跑通 Rust 核心 ➔ TCP IPC ➔ App 最小端到端闭环，绝不为了未完成的复杂度拆掉可运行功能。
- **模块化与关注点分离**：FAS 帧感知、CLG 调速器、CPUSet QoS、Idle Dive、TouchBoost 与 IPC 服务保持严格解耦与强类型约束。
- **成熟库优先与深度挖掘**：优先使用成熟维系库；新增依赖或自研前先盘点 `Cargo.toml`/`build.gradle.kts` 已有依赖能力。
- **长远架构决策**：不做“先这样以后再换”的临时方案，每次修改必须做可持续的终态架构决策。
- **参考成熟模式**：借鉴 Linux 内核、Android 系统框架及成熟开源调度器的已验证模式，禁止盲目从零发明。

---

## 2. 文档同步与维护规范
- **强制同步检查**：变动（功能、逻辑、IPC 协议、配置结构、交互、模块职责）发生时，必须同步更新 README、开发规范及接口文档。
- **收尾闭环与工作记录**：文档同步属于开发收尾必要条件，宣布“完成”、跑通测试或 commit 前强制检查。**修改代码后必须更新工作记录文档**（如 `docs/工作日志.md`）。内部细节调整未影响接口/交互可标注“无需更新文档”。
- **清理与归档过期文档**：误导性旧文档必须更新或移入 `docs/` 归档，不得留在默认阅读路径。
- **优先更新当前执行入口**：优先更新 `CLAUDE.md`、`README.md` 及各子模块 README。
- **正式说明书语气**：直接将正文改写为当前最新真实规则，**严禁补丁式语气**（如“修订说明”“当前改为”）。仅归档/Changelog 保留历史。

---

## 3. 编码、测试与协作规范
- **熔断机制与精确修改**：涉及跨模块公共文件或核心配置变动必须立刻停止并请示用户。修改仅限定于需求相关代码，清理自己产生的废弃变量与 import。
- **目标驱动与 TDD 测试闭环**：修改业务前先写/改测试（观察红灯 Red），编写最小实现使测试通过（绿灯 Green），重构验证保持绿灯。连续 3 次尝试失败必须整理日志向用户求助。
- **小白友好沟通**：用户是底层小白，对话与汇报必须使用**通俗大白话与生活比喻**，严禁堆砌专业黑话。回复统一使用中文；代码变量/方法名保持英文。
- **Git Commit 规范**：遵循 Conventional Commits（例如：`feat(fas): add target fps scaling`）。

---

## 4. 项目架构与关键路径
- **项目类型**: Android CPU 调度器 (Rust 核心守护进程 + Android 14 App) / 骁龙 8 Elite
- **核心源码**:
  - FAS 帧感知: `src/scheduler/fas/` (`pid.rs`, `controller.rs`, `frame_pipeline.rs`, `gear_state.rs`)
  - CLG 调速器: `src/scheduler/cpu_load_governor.rs` | 状态机/Doze: `src/scheduler/mod.rs`
  - CPUSet: `src/cpuset_manager/` | Idle Dive: `src/idle_dive/` | TouchBoost: `src/touch_boost/`
  - IPC 服务: `src/ipc_server.rs` (TCP 127.0.0.1:14567) | App: `android-app/`
- **配置文件**: `module/config/` (`config.yaml`, `cpuset.yaml`, `idle_dive.yaml`, `touch_boost.yaml`)

---

## 5. 编译构建与打包硬性要求

```powershell
# 1. Rust 交叉编译检查: $env:YUMI_SKIP_EBPF=1; cargo check --target aarch64-linux-android
# 2. xtask 全自动打包: cmd /c "set YUMI_SKIP_EBPF=1&& cargo run --package xtask -- b"
# 3. App 编译: cd android-app; .\gradlew.bat assembleDebug
# 4. 代码格式与 Clippy: cargo fmt --check; $env:YUMI_SKIP_EBPF=1; cargo clippy --target aarch64-linux-android
```

- **打包硬性要求**：
  1. `customize.sh` 严禁覆盖设备现有的 `/data/adb/modules/yumi/rules.yaml`（保护用户应用规则）。
  2. Shell/Yaml/Prop 脚本与配置强制使用 **Unix 换行符 (LF `\n`)**，严禁 Windows CRLF (`\r\n`)。
  3. Zip 根目录禁止残留未 Strip 二进制，Release 必须剥离符号表存放在 `core/bin/yumi`（包体积控制在 15MB 左右）。

---

## 6. 品牌命名约定
- **唯一对外名 `yumi`**：守护进程、内核模块、仓库与项目的对外名称统一使用 `yumi`；不得以 `Yuki` / `YukiCtrl` 作为项目或模块品牌名。
- **新代码与新文档强制 `yumi`**：新增源码、配置、文档与前端包名一律使用 `yumi`（前端包名以 `webui/package.json` 的 `name: yumi-webui` 为准），严禁再引入 `yuki-*` 品牌变体。
- **`yuki` 作为作者署名保留**：既有版权头 `Copyright (C) 2026 yuki`、`author=yuki`、`authors = ["yuki <loyeturz@163.com>"]` 中的 `yuki` 是开发者个人 handle，属作者署名而非项目品牌，保持原样不改动。
- **仓库 URL 以实际为准**：`Cargo.toml` 的 `repository` 指向 GitHub 实际仓库地址；若在 GitHub 侧将仓库改名为 `yumi`，同步更新该字段，不在代码侧提前改为未生效的 URL。
