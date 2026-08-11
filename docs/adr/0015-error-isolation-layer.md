# 错误隔离层范围：`ChameleonError::message()` 唯一用户文案层

定稿 #35 的错误隔离层范围（handoff §2 Tier-2 错误隔离）。源码复核已确认/修正各推断，本 ADR 把「用户可见文案层的边界、哪些变体透传原文、前端 catch 策略、与 T-CRED 的边界」固化为实现契约，供 `fix/error-isolation` 分支的 ready-for-agent 工单执行。**只解决策，不写产品代码**（map #28 plan-not-do）。

前置：#31（日志基础设施 ADR-0012-logging-infrastructure）——原文只进日志依赖 tracing 栈，本批净化排序在 logging-infra 落地之后。

---

## 1. 确认：`ChameleonError::message()` 是唯一用户可见文案层

**事实**（`src-tauri/src/lib.rs:29-30`）：`msg(e) = e.message()`，每条 Tauri 命令的错误路径都经 `msg` 映射为文案；前端 `notifyBackendError` 展示的即 Tauri rejection 携带的 `message()`。全仓库无第二条「错误 → 用户文案」路径。**确认成立**。

**附带事实**：thiserror 的 `#[error(…)]` Display 串（`error.rs:11-60`）与 `message()` 分离，仅用于日志 / 调试，非用户面。二者文案可各自演进——存在潜在漂移风险，记入关卡，非本工单范围。

## 2. 源事实修正：真·透传原文仅 `CdpOperation`（+ `build()` 一条），非三变体全透传

**修正 handoff 误判**：handoff 指 `LaunchFailed` / `CdpConnectFailed` 也透传原文（`error.rs:73,79-80,82`）。源码复核显示 launch 分类已中文化：

- `classify_launch_err`（`launcher.rs:71-88`）把 `chromiumoxide::CdpError` 映射为**自洽中文 detail**：`LaunchFailed` → "数据目录不可写：…" / "读取浏览器启动输出失败。"；`CdpConnectFailed` → "与浏览器调试端口握手失败。"。**不是原文**。
- `CdpConnectFailed` 仅经 classify 与 `sandbox.rs:66` 产生，无 `e.to_string()` 直传。
- 残留一条：`BrowserConfig::build()`（`launcher.rs:60`）`detail: e`，`e` 为 chromiumoxide `build()` 返回的 `String`（`config.rs:337` `Result<BrowserConfig, String>`，来自 `default_executable()?` 英文串）——属透传，量小。

**真·透传原文的是 `CdpOperation`**，6 处 `detail: e.to_string()`（`e` = chromiumoxide `CdpError`，英文技术原文）：

- `launcher.rs:285`（open_tab 建页）、`309`（读激活标签页列表）、`408`（quick link 开页）、`461`（login assist 填表）
- `window.rs:14`（读窗口 rect）

**净化范围 = `CdpOperation` 6 处 + `build()` 1 处**，不是 issue 列出的三变体全改。

## 3. 净化方式：`message()` 剥离原文，原文只进日志

**决议**：
- `message()` 保持纯函数；`CdpOperation` 剥离 `{detail}` → 固定中文文案（如 "浏览器操作失败，请重试。"），`LaunchFailed` 的 `build()` 分支 detail 同样中文化或剥离。
- enum 的 `detail` 字段**保留**，供 `Debug` 展示——命令层 `tracing::warn!(error = ?e, …)` 把完整原文写入日志（依赖 #31 的 tracing 栈）。
- **红线**：CDP 密码嗅探处（login assist 填表）的 detail 不进日志——与 #31 同款红线，凭证侧由 T-CRED 处理，本层不越界。

**依赖 #31**：现状零 tracing，剥离即永久丢信息；排序在 ADR-0012 落地之后，riding on 其 tracing 栈。与 #41/#42 同级依赖。

## 4. 前端 catch 策略：一律 `notifyBackendError` 透传后端 `message()`，dev/init 例外

**事实**（前端 catch 盘点）：
- 已用 `notifyBackendError`：`RoleCard.vue:239`（启动）、`Topbar.vue:152`（启动组）。
- 裸 `console.error`：`BrowserBar.vue:34,46`（设浏览器路径 / 选浏览器）。
- 通用 `message.error("固定文案")` 吞掉 detail：`RoleCard`（关/删）、`RoleDialog`、`SandboxesPanel`、`SettingsDialog`、`SnapshotsPanel`、`Topbar`（全关/清理/导出/导入）、`LinksDialog`、`HandoffDialog`.
- dev/init 例外（保持现状）：启动期静默吞错 `useAppState.ts:21`、`usePrefs.ts:13`（后端未就绪 / 旧版读取失败 → 保持默认，不打扰）；纯 dev 调试 `console` 输出。

**决议**：
- 用户动作触发的后端错误**一律** `notifyBackendError(message, err, prefix)` 透传后端 `message()`；前端不再自造后端错误文案（固定文案只作前缀/语境）。
- dev/init 例外排除在外（上列两处静默吞错 + dev console 输出）。
- **依赖 §3**：剥离原文后后端 `message()` 才安全全量上屏；前端统一排序在 §3 之后，避免把英文原文抛给用户。

与 T-CRED 边界：#35 只管通用错误文案层；凭证侧错误语义 / 脱敏由 T-CRED 工单处理，重叠处（login assist 填表失败）本层只保证「原文不进用户面、不进日志」。

---

## 毕业工单（`fix/error-isolation`）

**#44（合并工单，依赖 #31）**——错误隔离层落地：
- 后端：`CdpOperation` 6 处 + `build()` 1 处剥离原文 detail，`message()` 返回净化中文；命令层 `tracing::warn!` 记录完整原文（含 login assist 填表处的**脱敏**原文，不落密码）。
- 前端：所有用户动作 catch 统一 `notifyBackendError`（含 `BrowserBar.vue:34,46`），dev/init 例外保留。
- 回归：`error.rs` 现有 `all_variants_render_chinese_message` 测试扩展断言「message 不含英文原文」；前端 catch 盘点走 review 把关。

依赖排序：排在 `feature/logging-infra`（#31）之后（已加 GitHub blocked_by 边）。