# chameleon（变色龙）— Chrome 会话隔离管理工具

面向国内手工测试团队的桌面工具：每个测试角色运行在独立的 Chrome 用户数据目录中，通过 CDP 受工具控制，支持会话接力、快照与沙箱。纯本地、离线、无谷歌账号依赖。

## 领域文档

单上下文。碰代码前先读：

- `CONTEXT.md` — 领域术语表（系统 / 角色 / 数据目录 / 登录辅助 / 接力 / 会话快照 / 临时沙箱 / Quick Links 等）。命名一律用术语表词汇，不漂移到 `_Avoid_` 里的同义词。
- `docs/adr/` — 读触及你工作区域的 ADR。若输出与某 ADR 冲突，显式提出，不静默覆盖。

## 分支与协作模型（主干开发 + feature/fix）

- 主干：`master`（受保护；CI 绿后才合入）。
- 功能分支：`feature/<简述>` — 新功能，基于 `master`。
- 修复分支：`fix/<简述>` — bug 修复，基于 `master`。
- **不直接向 `master` 提交**；改动先开分支、走 PR 合并。
- 一次性 / 探索产物（原型、research 结果）用一次性命名分支（`prototype/<name>`、`research/<name>`），**不进主干**；main 只保留验证过的决定。

## 工作流：规划先行，禁止直接改码

本仓库的工程师态技能**默认只做规划，不直接修改代码**，产出的是规划工件而非代码改动：

- `/grill-me`（内部跑 `/grilling` + `/domain-modeling`）— 研磨一个计划或设计，产出决议、术语条目、ADR。
- `/grill-with-docs` — 研磨 + 边研磨边落 ADR / 术语表。
- `/wayfinder` — 把大块工作化为 `wayfinder:map` 决策地图 + 工单，一次解析一个；**plan not do**。
- `/to-spec`、`/to-tickets` — 把决议 / 规格化为 spec 与可认领工单（`ready-for-agent`）。
- `/triage` — 分诊工单。

这些技能产出决议、术语、ADR、spec、工单、地图——**不 push 产品代码改动**。代码实现由实现会话按 `ready-for-agent` 工单执行，走 `feature/` / `fix/` 分支 + PR。

- 分支/文档基点破解（如 AGENTS.md、CONTEXT.md 等规则文件）属于仓库元工作，可提交；产品代码始终走分支。

## Agent skills

### Issue tracker

Issues 与 spec 存在于 GitHub Issues（gh CLI）。见 `docs/agents/issue-tracker.md`。

### Triage labels

五个规范角色：needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix。见 `docs/agents/triage-labels.md`。

### Domain docs

单上下文：仓库根部一个 CONTEXT.md + docs/adr/。见 `docs/agents/domain.md`。


## 开发环境与构建

### 前置依赖

- Rust (msvc on Windows, stable on Linux) — `rustup`
- Tauri CLI: `cargo install tauri-cli`
- Linux (WSL2): `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`
- 测试需要 Chrome/Chromium 可执行文件在 PATH 中

### 开发模式

```bash
cargo tauri dev    # 编译 + 启动，热重载前端
```

### 构建发布

```bash
cargo tauri build  # 生产构建，产出安装器
```

## 测试

### 单元测试 + 集成测试

```bash
# 全部测试（需要 Chrome）
cargo test -p chameleon-core

# 集成测试带环境变量（headless + no-sandbox 用于 CI）
CHAMELEON_HEADLESS=1 CHAMELEON_NO_SANDBOX=1 cargo test -p chameleon-core
```

集成测试在 `crates/core/tests/integration.rs`，对真实 Chrome 运行：启动→CDP 连接→开标签→读激活标签→关窗→沙箱清理→快照恢复。无浏览器时自动跳过。

### 快速语法检查

```bash
cargo check          # 类型检查，比 build 快
cargo check -p chameleon-core
```

## 调试

### 前端调试

- `cargo tauri dev` 启动后，DevTools 可打开（Tauri 支持）
- 前端日志：`console.log` 输出到终端（Tauri 转发）
- UI 改动即时热重载，无需重启

### 后端调试

```bash
# 附加调试器（Rust）
cargo tauri dev -- --debug   # 或直接用 rust-gdb/lldb
```

### 日志与错误排查

- 应用日志输出到 stderr，`cargo tauri dev` 终端可见
- Webkit 警告（`libEGL warning` 等）在 WSL2 无 GPU 环境下正常，可忽略
- 配置错误 → 检查 `~/.config/chameleon/config.json`（Linux）或 `%APPDATA%/chameleon/`（Windows）


## 本地验证 Windows exe（WSL interop）

WSL2 能直接执行 Windows `.exe` 并把 stdout/stderr 转发到 WSL 终端——
不必装 Windows / 盲发 CI 就能冒烟启动、抓 Rust panic。发版前必做，避免又一轮坏 release。

### 跑 exe 抓启动 panic

```bash
# exe 必须在 Windows 文件系统路径（/mnt/c/...）；Linux fs（/tmp/...）会 permission denied
cp <exe> /mnt/c/Users/Public/chm.exe
RUST_BACKTRACE=1 timeout 12 /mnt/c/Users/Public/chm.exe   # panic 打到 stderr，<1s 出
```

v0.8.0 的启动闪退（`tokio::spawn` 在非 runtime 线程 panic，`src-tauri/src/lib.rs:712`）
就是这法子抓的——CI 构建一个 Windows 轮次 ~10min，本地 WSL 跑 exe 秒级出 panic。

### 本地 msvc 交叉构建（全本地闭环）

**前提**（一次性安装）：
```bash
# 1. cargo-xwin（拉 MSVC SDK + 用 lld-link）
cargo install cargo-xwin --locked

# 2. llvm-rc（tauri-winres 嵌 manifest/图标用）
sudo apt-get install -y llvm && sudo ln -sf "$(ls /usr/bin/llvm-rc-* | sort -V | tail -1)" /usr/local/bin/llvm-rc

# 3. clang-cl（cc-rs 编译 C 代码用）
# 方案 A：x pixi（无 sudo）
. ~/.x-cmd.root/X && x pixi use clang
# 方案 B：apt（需 sudo）
sudo apt-get install -y clang && sudo ln -sf "$(which clang)" /usr/local/bin/clang-cl
```

**构建命令**：
```bash
cd /home/kuuga/projects/chameleon
PATH="$HOME/.pixi/bin:$PATH" cargo xwin build --release --target x86_64-pc-windows-msvc -p chameleon-app
# 产物：target/x86_64-pc-windows-msvc/release/chameleon-app.exe（~15MB，静态链接 WebView2Loader）
```

**验证**：
```bash
cp target/x86_64-pc-windows-msvc/release/chameleon-app.exe /mnt/c/Users/Public/chm.exe
RUST_BACKTRACE=1 timeout 15 /mnt/c/Users/Public/chm.exe
# 无 panic + 窗口正常弹出 = GREEN
```

### exe 来源对比

| 来源 | WebView2 | 启动验证 | 推荐场景 |
|---|---|---|---|
| **本地 xwin msvc** | 静态链接 ✅ | ✅ 能到 Rust | 发版前本地验证 |
| **CI msvc portable** | 静态链接 ✅ | ✅ 能到 Rust | 下载 release 产物验证 |
| **gnu 交叉构建** | 动态链接 ❌ | ❌ DLL 缺失 | 仅验证 Rust 逻辑编译 |

### 踩坑记录

1. **`tokio::spawn` 在 Tauri setup 里 panic**（v0.8.0）
   - 根因：setup 回调在事件循环线程执行，Windows 上该线程未 enter tokio runtime
   - 修复：改用 `tauri::async_runtime::spawn`（走全局 runtime 句柄，不依赖 thread-local）
   - 模式：与 `lib.rs:690` quit 菜单回调同款

2. **release exe 弹 Edge localhost 报错**（v0.8.1）
   - 根因：`tauri.conf.json` 的 `devUrl: "http://localhost:1420"` 被嵌入 release exe，webview 加载 `http://localhost:1420/www` 而非内嵌 `www/`
   - 修复：分离配置文件——`tauri.conf.json`（生产）只放 `frontendDist: "www"`；`tauri.dev.conf.json`（开发）放 `devUrl` + `beforeDevCommand`
   - 原理：`cargo tauri dev` 合并两文件（devUrl 生效）；`cargo tauri build` 只读基础配置（devUrl 不嵌入）

## 验证清单

改动完成后，按以下顺序验证：

1. **编译通过**: `cargo check -p chameleon-core` — 零错误零警告
2. **测试通过**: `cargo test -p chameleon-core` — 全部绿
3. **UI 可运行**: `cargo tauri dev` — 窗口正常打开，无控制台报错
4. **功能验证**: 手动走一遍改动涉及的用户流程（如：新建角色→启动→登录辅助→关闭）
5. **边界情况**: 空状态、重复操作、错误输入不崩溃

### 发布前

- `cargo tauri build` 成功产出安装器
- GitHub Actions CI 绿（push 后自动触发）
- 版本号同步：`Cargo.toml` + `src-tauri/tauri.conf.json` 一致