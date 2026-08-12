# 预设系统重构：统一概念模型、真共享、预设级登录

现状预设系统存在三个结构性问题：(1) 两套登录机制（角色级 LoginConfig 半自动不存密码、预设级 QuickLinkLogin 全自动存密码）字段几乎相同但语义冲突，用户分不清该配哪套；(2) 系统级预设是"拷贝到角色"而非"真共享"，需要"应用到角色"按钮手动同步，改了系统级预设已应用过的角色不更新；(3) RoleCard/SystemBox 的 chip 点击走 `window.open` 在 chameleon WebView 打开而非角色隔离 Chrome，是 bug。

## 概念模型

**预设最小内核 = 命名 URL 书签**（name + url）。登录辅助和 auto_open 是挂在预设上的正交维度，不是预设的定义性属性。

**系统级预设 = 真共享**：组内所有角色实时可见可点，无需"应用到角色"拷贝。角色卡同时显示角色级预设和所属系统的系统级预设，分区排版（上区角色级、下区系统级）。

**登录辅助只留预设级一套**：删掉角色级 LoginConfig，统一为挂在 QuickLink 上的 QuickLinkLogin（用户名 + 密码 + 选择器）。一个角色可挂多个登录辅助（对应不同测试账号）。存储密码——纯本地离线工具，风险等同 Chrome 记住密码。系统级预设不支持登录辅助（共享 URL 不共享身份）。

**预设内部以 id 为唯一键**：QuickLink 加 `id: String`（uuid），name 变纯显示字段（可空、可重、可改）。add/remove/edit/open 全按 id 操作。

**auto_open 留在预设上作为正交标记**：启动时合并角色级 + 系统级 auto_open 预设，按 id 顺序执行，不按 URL 去重（用户勾选即执行）。

**不引入"环境"子层级**：环境差异用两个系统表达（如「XX-测试」「XX-生产」），零结构改动。

## 数据结构

- `QuickLink` 加 `id: String` 字段，`name` 变可选显示字段
- 删 `Role.login: Option<LoginConfig>` 字段
- 删 `LoginConfig` struct
- `System.quick_links` 和 `Role.quick_links` 结构不变，真共享体现在读取层（角色卡合并显示）

## UI 布局

- **RoleCard 分区排版**：上区角色级预设 chip，下区系统级预设 chip（带小标签区分来源）
- **chip 点击走 CDP**：调 `tauri.openQuickLink(role_id, preset_id)`，角色未启动则自动启动再打开。删 `window.open`
- **LinksDialog 行内编辑**：列表为主，每行可展开编辑（name + url + auto_open + 登录辅助字段），不弹子表单。登录辅助字段 = 用户名 + 密码（主区必填）+ 选择器（高级区可选）
- **删"测试登录"按钮**：配置和执行分离，测试 = 保存后点角色卡 chip
- **预设管理入口只留角色卡/系统卡上的按钮**：RoleDialog/SystemDialog 只管身份属性，删"管理预设"入口
- **角色级和系统级共用 LinksDialog**：系统级隐藏登录字段（`v-if="isRole"`）

## 实现优先级

分三批：
1. **批次 1（核心）**：QuickLink 加 id + 后端按 id 查找 + 前端 chip 点击走 CDP（修 bug）
2. **批次 2（UI）**：RoleCard 分区排版 + LinksDialog 行内编辑 + 密码字段
3. **批次 3（清理）**：删 Role.login + 删 applySystemLinks + 删"测试登录"按钮 + 删 RoleDialog/SystemDialog 的"管理预设"入口

## 未来方向

预设自动化脚本（Playwright/CDP 集成）：在预设上挂一段自动化脚本，点预设时执行。这是登录辅助的自然延伸（从"填两个输入框"到"执行一段脚本"）。需要单独设计 session，不影响当前重构。

## 数据迁移

- 现有 config.json 里的 `role.login` 数据不迁移，直接删。chameleon 早期工具，用户量小，让用户在清晰的新模型下重新配
- 现有 QuickLink 无 id，load 时自动补 uuid
