# chameleon（变色龙）— Chrome 会话隔离管理工具

面向国内手工测试团队的桌面工具：每个测试角色运行在独立的 Chrome 用户数据目录中，通过 CDP 受工具控制，支持会话接力、快照与沙箱。纯本地、离线、无谷歌账号依赖。领域词汇见 `CONTEXT.md`。

## Agent skills

### Issue tracker

Issues 与 spec 存在于 GitHub Issues（gh CLI）。见 `docs/agents/issue-tracker.md`。

### Triage labels

五个规范角色：needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix。见 `docs/agents/triage-labels.md`。

### Domain docs

单上下文：仓库根部一个 CONTEXT.md + docs/adr/。见 `docs/agents/domain.md`。