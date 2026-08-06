# Issue tracker: GitHub

本仓库的 issues 与 spec 均为 GitHub issues。所有操作使用 `gh` CLI。

## 约定

- **创建 issue**：`gh issue create --title "..." --body "..."`。多行正文用 heredoc。
- **读 issue**：`gh issue view <number> --comments`，用 `jq` 过滤评论并取标签。
- **列 issue**：`gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'`，配合 `--label` 与 `--state` 过滤。
- **评论 issue**：`gh issue comment <number> --body "..."`
- **打 / 去标签**：`gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **关闭**：`gh issue close <number> --comment "..."`

仓库由 `git remote -v` 推断——在 clone 内运行 `gh` 会自动指向它。

## Pull requests 作为分诊面

**PRs 作为请求面：否**。（若仓库把外部 PR 当 feature 请求则设为 `yes`；`/triage` 读此标记。）

设为 `yes` 时，PR 走与 issues 相同的标签和状态，使用 `gh pr` 对应命令：

- **读 PR**：`gh pr view <number> --comments` 与 `gh pr diff <number>`。
- **列外部 PR 分诊**：`gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments` 只保留 `authorAssociation` 为 `CONTRIBUTOR`、`FIRST_TIME_CONTRIBUTOR` 或 `NONE` 的（丢弃 `OWNER`/`MEMBER`/`COLLABORATOR`）。
- **评论 / 标签 / 关闭**：`gh pr comment`、`gh pr edit --add-label`/`--remove-label`、`gh pr close`。

GitHub 的 issues 与 PR 共享同一编号空间，裸 `#42` 可能是二者之一——用 `gh pr view 42` 解析，回退 `gh issue view 42`。

## 当技能说"发布到 issue tracker"时

创建一条 GitHub issue。

## 当技能说"取回相关工单"时

运行 `gh issue view <number> --comments`。

## 寻路操作

由 `/wayfinder` 使用。**map** 是一条标记 `wayfinder:map` 的 issue，**子**工单作为 tickets。

- **Map**：单条 `wayfinder:map` 标签的 issue，承载 Notes / Decisions-so-far / Fog 正文。`gh issue create --label wayfinder:map`。
- **子工单**：作为 GitHub sub-issue 链接到 map（`gh api` 子任务端点）。未开 sub-issue 时，把子工单加进 map 正文的任务列表，并在子工单正文顶部写 `Part of #<map>`。标签：`wayfinder:<type>`（`research`/`prototype`/`grilling`/`task`）。被认领后，工单分配给驱动的开发者。
- **阻塞**：GitHub 原生 issue 依赖——规范、UI 可见的表示。用 `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>` 加边，`<blocker-db-id>` 是阻塞者的数字**数据库 id**（`gh api repos/<owner>/<repo>/issues/<n> --jq .id`，非 `#number` 或 `node_id`）。GitHub 用 `issue_dependencies_summary.blocked_by` 报告（仅 open blockers——实时门）。依赖不可用时，退化为子工单正文顶部的 `Blocked by: #<n>, #<n>` 一行。一个工单在其每个 blocker 关闭时才算解除阻塞。
- **前沿查询**：列出 map 的 open 子工单（`gh issue list --state open`，限定 map 的 sub-issues / 任务列表），丢弃带 open blocker（`issue_dependencies_summary.blocked_by > 0`，或 `Blocked by` 行中有 open issue）或已 assignee 的；map 顺序第一个胜出。
- **认领**：`gh issue edit <n> --add-assignee @me`——会话的第一次写。
- **解决**：`gh issue comment <n> --body "<answer>"`，然后 `gh issue close <n>`，再把上下文指针（gist + 链接）追加到 map 的 Decisions-so-far。