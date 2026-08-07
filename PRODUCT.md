# Product

<!-- impeccable:product-schema 1 -->

## Platform

web
<!-- 实际为 Tauri 桌面应用（WebView2 渲染 Vue3 UI），Windows 为主、设计语言跨平台中性。skill 平台分类无 desktop，故记 web；原生窗口材质（Mica/Acrylic）与桌面特性记入 Operating Context。-->

## Stack

Vue 3 + TypeScript + Naive UI（Vite），WebView2 渲染；Rust/Tauri 后端，CDP 控制 Chrome/Edge。Live mode 不适用（桌面应用，非可运行 web 项目）。

## Users

国内手工测试团队的一线手工测试工程师。情景：同一被测系统需同时以多个角色（管理员 / 审计员 / 普通用户…）登录，逐一验证权限差异。工作：在浏览器之外管理多个角色隔离的 Chrome 会话，快速切换、批量起停、对比界面差异。

## Product Purpose

把每个测试角色关进独立的 Chrome `--user-data-dir`，通过 CDP 受工具控制，色块一眼可辨，支持会话接力、快照与临时沙箱。纯本地、离线、无谷歌账号依赖，让多角色手工测试从「反复登出登录 + 手动多开浏览器」变成「受控、可视、可批量、可接力」。

## Positioning

邻类方案（bat 多开脚本、手动切号、普通多实例）无法诚实声称的机制：每个角色严格隔离的独立数据目录 + CDP 受控自动化 + 色块化可视身份 + 会话接力/快照/沙箱——在 Windows 桌面上一键起停一批隔离测试窗口，绝不触碰日常 Chrome 配置。变色龙 = 随身份变色的窗口，每角色一个色块，一眼可辨。

## Operating Context

Windows 桌面应用（WebView2 运行时；NSIS 安装器 / 便携版）。测试人员在日常 Chrome 之外管理测试角色窗口，Pure 本地运行、可离线、整个文件夹可搬运。中文界面。数据目录与 Chrome/Edge 默认配置目录严格互斥，绝不触碰日常配置。开发在 WSL2/Linux，发布目标 Windows。

## Capabilities and Constraints

- 角色隔离：每角色独立 `--user-data-dir` + CDP 端口，Cookie/LocalStorage/缓存完全隔离；中文名 + 色块标识
- 系统分组：角色可选归属一个系统，启动组批量拉起；系统级常用 URL 预设组内共享
- 会话接力：A 当前激活标签页 URL → B 新标签页；并行模式保留双窗口对比，接力模式 CDP 优雅关闭原窗口
- 常用 URL 预设：角色/系统级，点击即开；角色级可标记启动时自动打开
- 登录辅助：登录页 URL + 用户名 + 输入框选择器，自动打开登录页并填用户名；密码手输，绝不存储密码
- 一键启动 / 一键关闭 / 启动组：批量起停，仅作用测试数据目录
- 会话快照 (v2)：各角色标签页 URL + 窗口位置 JSON，一键恢复；不含内部占位页签
- 临时沙箱 (v2)：用完即毁一次性隔离窗口；崩溃残留下次启动清理
- 数据目录清理 (v2)：一键清理测试临时数据目录
- 浏览器检测：自动检测 Chrome/Edge（含按用户安装），失败可手动选，多浏览器并列
- 配置导入/导出：明文 JSON，冲突拒绝且不破坏现有配置
- 安全边界：数据目录指向默认配置目录一律拒绝；单实例锁；错误走中文文案层
- 主题：深色 = Windows 原生 Mica (Win11)/Acrylic (Win10) 半透；浅色 = 关闭 vibrancy 实色填充（#F3F3F3）；UI 偏好（主题模式 + 面板透明度 + Accent）持久化到 config.json
- 术语一律遵循 CONTEXT.md（系统/角色/数据目录/登录辅助/接力/快照/沙箱/常用 URL 预设等），不漂移

## Brand Commitments

名称「变色龙 (chameleon)」= 随身份变色的窗口，每角色窗口一个色块，一眼可辨（已绑定身份约束）。中文界面，所有错误走中文文案层。定位为纯本地、离线、无谷歌账号依赖的桌面工具。

## Evidence on Hand

- README.md — 功能清单、安装/构建说明
- CONTEXT.md — 领域术语表（系统/角色/数据目录等）
- docs/adr/ — 架构决策记录
- ui/src/ — 现有前端实现（Vue 3 + Naive UI，含 Hybrid 主题策略）
- crates/core + tests/ — CDP 控制与集成测试实现
- 缺失：无真实用户证言、营销素材、外部信息披露；未来不得虚构测试用例、客户或部署声明

## Product Principles

- 严格隔离优先：每个角色独立数据目录，与 Chrome/Edge 默认配置目录永不相交
- 受控自动化：浏览器一切操作走 CDP，可解释、可批量、可接力、可优雅关闭
- 可视身份：色块 + 中文命名让多角色与多窗口一眼可辨
- 本地优先：纯本地、离线、无谷歌账号依赖，文件夹可搬运
- 安全边界先于便利：拒绝默认配置目录、不存密码、单实例锁

## Accessibility & Inclusion

无强制合规标准（产品决策，用户确认）。保持基础可用性：足够对比度（含深色 Mica/Acrylic 半透背景）、键盘可达、可读中文文案。