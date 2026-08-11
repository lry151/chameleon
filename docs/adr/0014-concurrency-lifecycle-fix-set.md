# 并发/生命周期修复集：shutdown 超时+防重入、delete_role 拆锁、takeover 端口轮询、prune 不变量、close 超时可观测

定稿 #33 的并发/生命周期修复集。R1（#29）源码复核已确认/修正各推断，本 ADR 把「修什么、为何、值多少、不变量是什么」固化为实现契约，供 `fix/lifecycle-determinism` 分支的 ready-for-agent 工单执行。**只解决策，不写产品代码**（map #28 plan-not-do）。

修复优先级：P1 = 用户可直接感知的卡顿 / 数据目录占用根因链；P2 = 可观测性前置项 / ADR-0006 false positive；P3 = 功能已对但不可观测。

---

## 1. shutdown：全局超时（30s）+ 防重入（P1）

**事实**（`src-tauri/src/lib.rs:474-485`）：`shutdown` 串行执行 `prune_dead_roles`（N×1.2s）+ `close_all_roles`（`launcher.rs:380-384` 串行，每角色最坏 12s）+ 关沙箱。无整体超时，10 角色最坏 ~120s。正常退出路径（托盘 quit `lib.rs:599`/ `quit_app` `lib.rs:466-469`）安全；但串行 120s 让用户「退出」卡住 → 强杀 → Chrome 孤儿 → 下次启动 `--user-data-dir` 被占（ADR-0006 `PortTakenNotRole` 根因链一环）。

**决议**：
- **全局超时 30s**：`tokio::time::timeout(30s, …)` 包裹整个 shutdown 体。值选 30s 的理由：并行 close 下总耗时 ≈ 单个最慢角色 close（≤12s）+ prune 探测（N×1.2s 串行），30s 给足头部余量，同时把用户可感知挂起从 120s 压到 30s 内。
- **防重入**：`AtomicBool` compare-exchange 守卫。shutdown 有两个入口（托盘 quit spawn + `quit_app` 命令），并发触发须只执行一次；重入直接返回。
- **close 并行化**：串行 `close_all_roles` 改为复用 `batch::close_all` 的 spawn + `join_all` 并行模式（`batch.rs:241-290`），否则超时对多角色频繁触发。这是「让 30s 超时不被常态触发的」前提，与超时同批落地。

**不变量**：shutdown 常路径（托盘/quit_app）在 30s 内完成或超时兜底；任何时刻最多一个 shutdown 在跑；超时后 app 仍 `exit(0)`,不残留可观测卡死。

## 2. prune_dead_roles：收紧到「杀 Chrome」，不改成「仅记录+不动」（P3）

**事实**（R1 修正 handoff 误判）：`prune_dead_roles`（`launcher.rs:328-343`）移除死角色 → `RunningRole` drop → chromiumoxide `kill_on_drop(true)` **杀了 launched Chrome**（`browser/mod.rs:504-523`）；takeover（`child:None`）不杀 = **正确**（不拥有别家进程）。handoff「prune 不杀」是误判。

**决议**：**收紧到「杀 Chrome」语义**（维持 remove→Drop 兜底杀 launched 死 Chrome），不改成「仅记录+不动」。不杀会留下锁数据目录的孤儿进程 = 下次启动占用根因。收紧点是**让杀可观测**：prune 移除时 WARN 记录 role_id + 探测耗时（依赖 T-LOG）。**不引入** graceful-close-before-drop——对探测确认不可达的浏览器是浪费。

**不变量**：prune 只移除「探测确认不可达（`version()` 1.2s 超时）」的角色；takeover（`child:None`）永不杀；绝不主动杀探测仍存活（version 通过）的 Chrome；杀仅作为 launched 死 Chrome 的 Drop 兜底，不显式 `kill()`。

## 3. Windows Job Object：延后到 Windows 专项（延后）

**事实**：Job Object 是「chameleon 进程被强杀/任务管理器结束」时级联终止子 Chrome 的终极兜底。R1 裁定真正的孤儿风险来自「shutdown 慢→用户强杀」与 SIGTERM/任务管理器结束路径。

**决议**：**延后到独立 Windows 专项工单**。理由：①与并发确定性修复正交，是独立平台兜底；②需平台代码（`CreateJobObjectW` + `AssignProcessToJobObject` + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`），自包含、值得单独验证；③P1 修复（shutdown 30s 超时 + 防重入）已消除「用户等不及强杀」这一主要孤儿源。延后 = 单列工单不丢失，优先级低于本批 P1/P2。

## 4. delete_role mutex：范围缩减 — 不持 session mutex 跨 CDP/网络探测（P1）

**事实**（`src-tauri/src/lib.rs:118-130`）：`delete_role` 持 `session.lock()` 跨 `prune_dead_roles`（N×1.2s）+ `close_role`(12s)，最坏 ~18s 持锁，阻塞所有需锁命令（`launch_role_cmd`/`close_role_cmd`/`get_state`/`login_assist_cmd`/`handoff_cmd`…）。`is_role_running` 只是 `roles.contains_key`（`session.rs:37-39`），全局 prune 对删除单个角色非必需。

**决议**：**确认缩减**。delete_role 复用 `batch::close_one` 的短锁模式（`batch.rs:104-113`）：①短锁取出 `RunningRole` + 从 cfg 读 port；②无锁执行 `close_role_no_session`（`launcher.rs:252-271`）；③短锁写回窗口位置 + 删 cfg 角色。删除路径去掉全局 `prune_dead_roles` 调用（只关心目标角色）。

**不变量**：任何 Tauri 命令持 session mutex 的时间与 CDP/网络操作解耦——锁内只有内存操作，慢速 CDP 工作一律无锁执行。

## 5. close 超时：记录到日志 WARN 层（P2，依赖 T-LOG #31）

**事实**（R1）：全仓库 10+ 处 `let _ = timeout(…)` 吞错（`lib.rs:126/550`、`launcher.rs:233/236/257/258`、`sandbox.rs:100/103/104`、`batch.rs:124`），零 tracing 依赖，close 超时完全不可观测——与 P1 叠加：shutdown 慢→用户看不到原因→强杀→孤儿。

**决议**：**确认记录到 WARN**。所有 `let _ = timeout(…)` 降级路径改 `match`：`Ok(Ok(_))`→debug、`Ok(Err(e))`→warn、`Err(_)`→warn("close timed out after 5s")；`let _ = store.save(…)` 改 `if let Err(e) → warn`。**依赖 T-LOG #31 的 WARN 层**——排序在 `feature/logging-infra` 落地之后，riding on 其 tracing 栈。

## 6. （R1 新增，归入 #33）takeover 端口轮询 2s→5s（P2）

**事实**（#28 Not-yet-specified 归入 #33）：`close_role` 兜底轮询端口 20×100ms=2s（`launcher.rs:239-246`）。接管（`child:None`）下 `Browser::wait()` 是 no-op（chromiumoxide `browser/mod.rs:267-273`），全靠轮询确认端口释放。重负载下 Chrome 未在 2s 内退完 → 紧接启动误判端口占用 → 触发 ADR-0006 `PortTakenNotRole` false positive（自家 Chrome 没退完 ≠ 外部占用）。

**决议**：**轮询上限从 2s 提至 5s**（与 `browser.close()` 的 5s 超时对齐）。同时 `tracing::debug!` 记录轮询实际耗时，便于诊断何时仍不足。属实现修正，非 ADR-0006 设计变更。

---

## Considered Options

- **shutdown 超时值**：15s（太紧，并行下多角色 + 探测易常态触发）／30s（**采纳**，给足头部余量且把挂起压到界定内）／60s（太长，用户仍会等不及强杀）。
- **shutdown 串行 vs 并行**：串行（现状，120s 卡死根因）／并行 spawn+join_all（**采纳**，复用 batch 既有模式）。
- **prune 不变量**：仅记录不动（**否决**——留孤儿锁目录，下次启动占用根因）／维持 remove→Drop 杀 launched 死 Chrome（**采纳**）+ 可观测化。
- **prune graceful-close-before-drop**（R1 建议）：**否决**——对探测确认不可达的浏览器再发 close 是浪费且引入额外延迟。
- **Windows Job Object**：现在做（**否决**——需平台代码自包含验证，优先级低于 P1/P2）／延后到 Windows 专项工单（**采纳**——单列不丢失）。
- **delete_role**：持锁跨 CDP（现状，18s 阻塞）／短锁+无锁 close（**采纳**，复用 close_one 模式）。
- **端口轮询**：保持 2s（**否决**——重负载 false positive）／提至 5s（**采纳**，对齐 close 超时）。

## Consequences

- **毕业工单**（走 `fix/lifecycle-determinism`，均带回归 + tdd/diagnosing-bugs 红→绿）：
  - T-CONC-1（P1）delete_role 拆锁（短锁取 + 无锁 close）。
  - T-CONC-2（P1）shutdown 30s 超时 + AtomicBool 防重入 + close 并行化。
  - T-CONC-3（P2）takeover 端口轮询 2s→5s + debug 耗时。
  - T-CONC-4（P3）prune 可观测（WARN 记录 pruned role_id + 探测耗时），不改杀语义——依赖 #31。
  - T-CONC-5（P2）close 超时 WARN 日志（10+ 处 `let _ =`）——依赖 #31。
  - T-CONC-6（延后）Windows Job Object 级联终止——独立 Windows 专项工单。
- 依赖排序：T-CONC-1/2/3 不依赖日志，可先行；T-CONC-4/5 排在 `feature/logging-infra`（#31）之后。
- 不变量：①持锁时间与 CDP/网络解耦；②任何时刻最多一个 shutdown；③prune 绝不杀探测仍存活的 Chrome、永不杀 takeover；④shutdown 30s 内常路径完成或超时兜底。
- ADR-0006 语义不变：接管成功则接管、连接失败硬错误——本批只消除其 false positive 边界（5s 轮询），不改变设计意图。