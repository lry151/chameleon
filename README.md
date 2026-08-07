# 变色龙 (chameleon)

> 面向国内手工测试团队的 Chrome 会话隔离管理工具。每个测试角色一个独立 Chrome 数据目录，色块一眼可辨，支持会话接力、快照、沙箱。纯本地、离线、无谷歌账号依赖。

## 为什么

手工测试同一系统常需同时以多个角色（管理员 / 审计员 / 普通用户…）登录验证权限差异。浏览器只有一份会话，切换角色要反复登出登录；靠 bat 脚本开多实例不可控、不直观，还有误操作波及日常 Chrome 配置的风险。变色龙把每个角色关进独立数据目录，CDP 受控、色块区分、一键批量、接力传 URL。

## 功能

- **角色隔离**：每角色独立 `--user-data-dir` + CDP 端口，Cookie/LocalStorage/缓存完全隔离；中文名 + 色块标识
- **系统分组**：角色可选归属一个系统（被测应用），启动组批量拉起该系统全部角色；系统级常用 URL 预设组内共享
- **会话接力**：把角色 A 当前激活标签页 URL 传到角色 B 新标签页。并行模式保留 A 双窗口对比；接力模式 CDP 优雅关闭 A
- **常用 URL 预设**：角色/系统级预设，点击即开；角色级预设可标记「启动时自动打开」
- **登录辅助**：角色配登录页 URL + 用户名 + 输入框选择器，点「登录」自动打开登录页并填用户名，密码手输（不存储密码）
- **一键启动 / 关闭 / 启动组**：批量拉起全部角色 / 启动某系统角色 / CDP 优雅关闭全部测试窗口，绝不触碰日常 Chrome 配置
- **窗口位置记忆**：移动/缩放后重启回到上次位置
- **会话快照 (v2)**：保存所有角色标签页 URL + 窗口位置为 JSON，一键恢复
- **临时沙箱 (v2)**：用完即毁的一次性隔离窗口，进程退出自动删数据目录；崩溃残留下次启动清理
- **数据目录清理 (v2)**：一键清理测试临时数据目录
- **浏览器检测**：自动检测 Chrome/Edge（含 `%LOCALAPPDATA%` 按用户安装），失败可手动选；多浏览器并列选择
- **配置导出/导入**：明文 JSON，新人入职直接导入；冲突拒绝且不破坏现有配置
- **安全边界**：数据目录指向 Chrome/Edge 默认配置目录一律拒绝；单实例锁；所有错误走中文文案层

术语见 [CONTEXT.md](./CONTEXT.md)，架构决策见 [docs/adr/](./docs/adr/)。

## 安装

到 [Releases](https://github.com/lry151/chameleon/releases) 下载最新版：

- **`chameleon_<ver>_x64-setup.exe`** — NSIS 安装器。Win10 未装 WebView2 运行时时安装器自动下载安装（需联网）。装到当前用户目录，无需管理员。
- **`chameleon_<ver>_x64-portable.zip`** — 便携版，解压双击 `chameleon.exe` 即用。依赖系统已装 WebView2 运行时（Win11 自带；Win10 多数已装，精简镜像若无请先装 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)）。

运行后 exe 同目录生成 `config.json` / `data` / `snapshots`，整个文件夹可搬运。

## 从源码构建

### 前置依赖

- Rust（msvc on Windows，stable on Linux）—— `rustup`
- Tauri CLI：`cargo install tauri-cli`
- Node.js 18+、pnpm
- Linux (WSL2)：`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`
- 测试需要 Chrome/Chromium 可执行文件在 PATH 中

### 开发模式

```bash
# 启动 Tauri + 新 UI（Vue 3 + Naive UI），前端在 ui/ 热重载
cargo tauri dev
```

前端源码在 `ui/`（Vite + Vue 3 + TS + Naive UI）。修改 `ui/src/**` 即时热重载，无需重启 Rust。
单独启动前端 dev server：`pnpm --dir ui dev`（默认 `http://localhost:1420`）。
单独构建前端：`pnpm --dir ui build`，产物输出到 `src-tauri/www/`。

### 构建发布

```bash
cargo tauri build   # 自动构建前端 + 打包 NSIS 安装器
```

打 tag 自动出包（GitHub Actions，windows runner + msvc）：

```bash
git tag v0.2.0 && git push origin v0.2.0
```

## 配置

`config.json` 为唯一配置源，明文 JSON 人工可改。角色列表 + 浏览器路径 + 数据根目录 + 系统分组。

## 技术栈

Tauri 2（Web 前端 + Rust 后端）+ chromiumoxide（CDP）。核心领域逻辑在 `crates/core`，Tauri 外壳薄壳透传。前端 Vue 3 + TypeScript + Naive UI + Fluent Design tokens，Vite 构建，产物输出到 `src-tauri/www/`。视觉语言与 Hybrid 主题策略详见 [ADR-0009](./docs/adr/0009-fluent-design-hybrid-theme.md)。

## 调试

### 前端

`cargo tauri dev` 启动后 DevTools 可打开（Tauri 支持）。`console.log` 输出到启动终端。UI 改动即时热重载，无需重启。前端源码在 `ui/src/`。

### 后端

```bash
cargo tauri dev -- --debug   # 或直接用 rust-gdb / lldb
```

日志输出到 stderr，`cargo tauri dev` 终端可见。Webkit 警告（`libEGL warning` 等）在 WSL2 无 GPU 环境下正常，可忽略。

## License

MIT
