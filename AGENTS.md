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