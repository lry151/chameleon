# Concurrency & Lifecycle — Scout 推断复核

> **分支**：`research/concurrency-verification`（基于 `master @ 4b20203`）
> **目的**：逐条复核 handoff §2 Tier-1 并发/生命周期推断，消除 `[INFERENCE]` 不确定性。
> **产出**：本文件为唯一交付物；未触碰任何产品代码。
> **复核方法**：每条推断对照 `master @ 4b20203` 源码行号 + 第三方依赖（chromiumoxide 0.9.1）源码验证。

---

## 0. 行号校正说明

Handoff 文档中部分 `lib.rs:NNN` 引用指向 `src-tauri/src/lib.rs`（Tauri 命令外壳，667 行），部分指向 `crates/core/src/lib.rs`（领域核心，26 行 — 不含被指代码）。以下对每处引用给出 master 上的真实行号。

---

## 1. 推断 (1)：`delete_role` 持 session mutex 跨 prune+close，可达 ~10s，阻塞所有 command

### 裁决：**确认** ✅

### 证据

**`src-tauri/src/lib.rs:118-130`**（handoff 写 `lib.rs:99-108`，实为 `get_state`，引用错误；真实 `delete_role` 起于 118）：

```rust
118: async fn delete_role(state: State<'_, AppState>, id: String) -> Result<(), String> {
119:     let store = state.store();
120:     let mut cfg = store.load().map_err(msg)?;
121:     {
122:         let mut session = state.session.lock().await;          // ← 获锁
123:         launcher::prune_dead_roles(&mut session).await;        // ← 每个角色 1.2s CDP 探测
124:         if session.is_role_running(&id) {
125:             let _ = launcher::close_role(&mut session, &store, &mut cfg, &id).await; // ← 关闭
126:         }
127:     }                                                           // ← 释锁
128:     store.delete_role(&mut cfg, &id).map_err(msg)
129: }
```

**临界区内慢操作**：

| 操作 | 源码位置 | 最坏耗时 |
|---|---|---|
| `prune_dead_roles` | `crates/core/src/launcher.rs:328-344` | N × 1.2s（每个角色一次 CDP `version()` 超时） |
| `close_role` | `crates/core/src/launcher.rs:216-248` | 5s（`browser.close()`）+ 5s（`browser.wait()`）+ 2s（端口轮询）= **12s** |

合计最坏：N × 1.2s + 12s。5 个角色、2 个死亡时 ≈ 6s + 12s = **18s 持锁**。

**被阻塞的命令**（均需 `state.session.lock().await`）：

- `launch_role_cmd`（`lib.rs:177`）
- `close_role_cmd`（`lib.rs:186`）
- `get_state`（`lib.rs:72`）— UI 状态刷新
- `login_assist_cmd`（`lib.rs:233`）
- `handoff_cmd`（`lib.rs:255+`）
- `open_quick_link` / `read_active_tab` 等

### Blast Radius

**严重**。用户删除一个角色时，若名下有 N 个角色（部分已死），UI 可能完全无响应 10-20s。所有 Tauri 命令排队等待。在低配机器或 Chrome 响应慢时更明显。

### ADR-0006 影响

无冲突。此问题与端口接管语义无关，纯粹是临界区粒度过粗。

### 修复方向

1. `prune_dead_roles` 移到锁外：先探测、收集 dead IDs，再持锁移除（或把探测放到 `get_state` 的注释已说的「命令路径 prune」中单独处理）。
2. `close_role` 拆为「从 session 取出 Browser」（短暂持锁）+「CDP 关闭 + 端口轮询」（无锁执行）— 参考 `batch.rs::close_one` 已采用此模式（`batch.rs:104-113` 短暂持锁取出，`batch.rs:116` 无锁调用 `close_role_no_session`）。
3. `delete_role` 应复用 `close_one` 的「短锁 + 无锁关闭」模式。

---

## 2. 推断 (2)：接管路径 `Browser::connect` 的 `wait()` 是 no-op + 端口轮询 2s 太短

### 裁决：**确认** ✅

### 证据

**chromiumoxide 0.9.1 源码验证**（`~/.cargo/registry/src/.../chromiumoxide-0.9.1/src/browser/mod.rs`）：

- **`Browser::connect_with_config`**（line 87-143）：构造 `Browser` 时 `child: None`（line 139）。
- **`Browser::wait`**（line 267-273）：
  ```rust
  pub async fn wait(&mut self) -> io::Result<Option<ExitStatus>> {
      if let Some(child) = self.child.as_mut() {
          Ok(Some(child.wait().await?))
      } else {
          Ok(None)   // ← connect 路径立即返回 None
      }
  }
  ```
  文档（line 265-266）明确："This call has no effect if this Browser did not spawn any chromium instance (e.g. connected to an existing browser through Browser::connect)"

**chameleon 关闭代码**（`crates/core/src/launcher.rs:232-247`）：

```rust
232:    let mut browser = run.browser;
233:    let _ = tokio::time::timeout(Duration::from_secs(5), browser.close()).await;
234:    // 等 Chrome 进程真正退出；对 launched 实例 wait() 等到进程退出，
235:    // 对 takeover（connect）实例 wait() 立即返回，靠下面轮询端口兜底。
236:    let _ = tokio::time::timeout(Duration::from_secs(5), browser.wait()).await;
237:    // 兜底轮询端口：takeover 路径下 wait() 不阻塞，Chrome 异步退出期间
238:    // 端口仍占用，轮询到端口释放或最多 2 秒后返回。
239:    if let Some(port) = port {
240:        for _ in 0..20 {
241:            if !port_open(port) {
242:                return Ok(());
243:            }
244:            tokio::time::sleep(Duration::from_millis(100)).await;
245:        }
246:    }
```

**端口轮询上限**：20 × 100ms = **2s**。

代码注释（line 214, 234-238）已主动承认此行为 — 作者知情。

handoff 引用 `launcher.rs:240-244` 基本准确；`launcher.rs:281-296` 实为 `open_tab`（271-290），引用错误。

### Blast Radius

**中等**。接管实例关闭后，若 Chrome 在 2s 内未释放端口（重负载、多标签、磁盘慢），后续 `launch_role` 的 `port_open()` 判断端口仍占用 → 尝试 `Browser::connect` → 若 Chrome 正在退出中，CDP 连接可能成功也可能失败 → 若失败触发 ADR-0006 硬错误 `PortTakenNotRole`，用户看到「请一键关闭所有后重试」。

实际场景：正常退出 Chrome 通常在 ~500ms 内完成。2s 对多数情况足够，但对极端场景（数百标签、机械硬盘）可能不足。

### ADR-0006 影响

**间接相关**。ADR-0006 设计意图是「接管成功则接管，连接失败则硬错误」。2s 轮询不足时，关闭 → 紧接启动 可能误触发 `PortTakenNotRole` 硬错误，但这不是端口被外部占用的真实情况，而是自家 Chrome 还没退完。**属于 ADR-0006 的 false positive 边界**。

### 修复方向

1. 轮询上限从 2s 提至 5-8s（与 `browser.close()` 超时对齐）。
2. 或：takeover 路径关闭后，改用 CDP `Browser.close` 的 ACK + 端口轮询双重确认，而非仅靠端口。
3. 记录轮询实际耗时（`tracing::debug!`），便于诊断。

---

## 3. 推断 (3)：`shutdown` 无超时；`ExitRequested` 空处理 → 强杀时 Chrome 成孤儿

### 裁决：**部分确认** ⚠️

### 证据

**`shutdown` 函数**（`src-tauri/src/lib.rs:474-485`；handoff 写 `lib.rs:613-622`，实为 `TrayIconBuilder` 段，引用错误）：

```rust
474: async fn shutdown(session: Arc<tokio::sync::Mutex<Session>>, app_dir: PathBuf) {
475:     let store = ConfigStore::new(app_dir.join("config.json"));
476:     let mut cfg = store.load().unwrap_or_default();
477:     let mut session = session.lock().await;
478:     launcher::prune_dead_roles(&mut session).await;
479:     launcher::close_all_roles(&mut session, &store, &mut cfg).await;
480:     let ids: Vec<String> = session.sandboxes.keys().cloned().collect();
481:     for id in ids {
482:         let _ = sandbox::close(&mut session, &id).await;
483:     }
484: }
```

- **无整体超时**：`close_all_roles`（`launcher.rs:380-385`）串行逐角色 `close_role`，每个最坏 12s。10 个角色 = 最坏 120s。
- **串行而非并行**：`batch::close_all`（`batch.rs:241-284`）用 `tokio::spawn` + `join_all` 并行关闭，但 `shutdown` 走的是 `close_all_roles`（串行）。
- handoff 写 `lib.rs:613-622` — 实为托盘图标构建段；真实 `shutdown` 在 474-485。

**`ExitRequested` 空处理**（`src-tauri/src/lib.rs:652-656`，**引用准确**）：

```rust
652:    .run(|_app_handle, event| {
653:        if let tauri::RunEvent::ExitRequested { .. } = event {
654:            // 清理在显式退出路径（托盘 quit / quit_app 命令）的 spawn 任务里
655:            // 已完成；此处不再 block_on，避免主线程阻塞导致托盘「退出」卡死。
656:        }
657:    });
```

### 退出路径完整追踪

| 退出触发 | shutdown 是否执行 | 孤儿风险 |
|---|---|---|
| 托盘「退出」菜单 | ✅ `on_menu_event` → spawn `shutdown` → `app.exit(0)` | 无 |
| `quit_app` 命令 | ✅ 直接 `shutdown` → `app.exit(0)` | 无 |
| `app.exit(0)` 触发 `ExitRequested` | ✅ shutdown 已在上一路径执行 | 无 |
| 用户 SIGTERM（Ctrl+C / 任务管理器结束） | ❌ 进程立即终止，shutdown 不执行 | **有** |
| 用户 SIGKILL | ❌ 无法捕获 | **有** |
| shutdown 执行中用户不耐烦强杀 | ❌ shutdown 被中断 | **有** |

### Blast Radius

**中-高**。前两条正常退出路径安全。真正的孤儿风险来自：
1. **shutdown 太慢 → 用户强杀**：串行 close_all_roles 最坏 120s，用户在「退出」卡住 10s+ 后倾向于任务管理器强杀 → Chrome 孤儿 → 下次启动 `--user-data-dir` 被占。
2. **SIGTERM**：桌面端较少见，但 Windows 任务管理器「结束任务」发 WM_CLOSE → Tauri 可能走 `ExitRequested`（空处理）而非 tray quit 路径。

handoff 称此为「下次数据目录占用根因」— **部分正确**：它是 chain 中的一环（shutdown 慢 → 用户强杀 → 孤儿 → 目录占用），但根因更准确地说是 **shutdown 无超时 + 串行执行导致用户感知卡死**。

### ADR-0006 影响

无直接冲突。但 shutdown 不完整导致的孤儿进程，在下次启动时可能触发 ADR-0006 的 `PortTakenNotRole` 硬错误（端口被孤儿 Chrome 占用且 CDP 不可达）。

### 修复方向

1. `shutdown` 整体加 `tokio::time::timeout(Duration::from_secs(30), ...)` 包裹。
2. `shutdown` 改用 `batch::close_all` 的并行模式（或直接在 shutdown 里 spawn 并行关闭）。
3. `ExitRequested` 加防重入 + 兜底：若 shutdown 未在 tray/quit_app 路径执行过，在此处 spawn 一个带超时的清理。
4. 考虑 Windows Job Object / Linux process group 级联终止子进程（handoff rule §4.6 已建议）。

---

## 4. 推断 (4)：`prune_dead_roles` 不杀 Chrome；`cleanup_orphans` 静默

### 裁决：**部分反驳** ⚠️（prune 实际会杀 Chrome，cleanup_orphans 是沙箱目录非 Chrome）

### 证据

**`prune_dead_roles`**（`crates/core/src/launcher.rs:328-344`）：

```rust
328: pub async fn prune_dead_roles(session: &mut Session) -> usize {
329:     let mut dead = Vec::new();
330:     for (id, run) in &session.roles {
331:         let alive = matches!(
332:             tokio::time::timeout(Duration::from_millis(1200), run.browser.version()).await,
333:             Ok(Ok(_))
334:         );
335:         if !alive {
336:             dead.push(id.clone());
337:         }
338:     }
339:     let n = dead.len();
340:     for id in &dead {
341:         session.roles.remove(id);   // ← 移除 → Browser drop
342:     }
343:     n
344: }
```

当 `session.roles.remove(id)` 执行时，`RunningRole` 被 drop → 内含的 `chromiumoxide::Browser` 被 drop。

**chromiumoxide `Browser::Drop`**（`browser/mod.rs:504-523`）：

```rust
impl Drop for Browser {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            if let Ok(Some(_)) = child.try_wait() {
                // Already exited
            } else {
                // We set the `kill_on_drop` property for the child process...
                tracing::warn!("Browser was not closed manually, it will be killed automatically...");
            }
        }
    }
}
```

**`kill_on_drop` 确认**（`async_process.rs:23`）：`inner.kill_on_drop(true);` — chromiumoxide 对所有启动的子进程设置了 `kill_on_drop`。

**结论**：
- **Launched 角色**（`child: Some`）：`session.roles.remove` → Browser drop → `kill_on_drop` → **Chrome 被杀** ✅
- **Takeover 角色**（`child: None`）：`session.roles.remove` → Browser drop → `child` 为 None → **无进程操作**（正确，我们不拥有该进程）

handoff 写「prune 不杀 Chrome」**对 launched 角色不准确** — Chrome 确实被杀，只是通过 Drop 隐式机制而非显式 `kill()` 调用。这意味着：
- 没有日志记录（除了 chromiumoxide 内部的 `tracing::warn!`，但 chameleon 未配置 tracing subscriber，所以该 warn 也不可见）
- 没有优雅关闭尝试（直接 kill，不先发 `Browser.close`）
- 对 takeover 角色行为正确

**`cleanup_orphans`**（`crates/core/src/sandbox.rs:110-133`；handoff 引 `launcher.rs:233`，实为 `close_role` 的 `let _ = timeout(close)`，引用错误）：

```rust
110: pub fn cleanup_orphans(session_live_ids: &[String], cfg: &GlobalConfig) -> Result<usize> {
...
129:     let _ = fs::remove_dir_all(&p);   // ← 静默丢弃错误
130:     removed += 1;
...
133: }
```

这是**沙箱临时目录**清理，不是 Chrome 进程清理。`let _ = fs::remove_dir_all` 确实静默吞错，但与 Chrome 孤儿无关。

### Blast Radius

**低-中**。prune 通过 kill_on_drop 杀 Chrome 在功能上正确，但：
- 隐式机制 = 不可观测（无日志、无指标）
- 非优雅关闭（直接 SIGKILL，Chrome 未保存状态）
- 若 chromiumoxide 未来改变 Drop 行为，chameleon 无防御

`cleanup_orphans` 静默吞错对沙箱目录清理来说影响有限（最坏残留临时目录占磁盘）。

### ADR-0006 影响

无冲突。

### 修复方向

1. `prune_dead_roles` 在移除前先尝试 `browser.close()`（带短超时），失败再 drop → 从 SIGKILL 升级为优雅关闭。
2. 显式 `browser.kill()` 或至少 `tracing::warn!` 记录 prune 杀了哪些角色。
3. `cleanup_orphans` 的 `let _ =` 改为 `if let Err(e) = ... { tracing::warn!(...) }`。

---

## 5. 推断 (5)：close 超时不记录

### 裁决：**确认** ✅

### 证据

全仓库 `let _ =` 吞错审计（仅列 close/timeout 相关）：

| 位置 | 代码 | 吞了什么 |
|---|---|---|
| `src-tauri/src/lib.rs:126` | `let _ = launcher::close_role(...)` | delete_role 关闭失败/超时 |
| `src-tauri/src/lib.rs:550` | `let _ = sandbox::cleanup_orphans(...)` | 沙箱清理失败 |
| `crates/core/src/launcher.rs:233` | `let _ = tokio::time::timeout(5s, browser.close())` | CDP close 超时 |
| `crates/core/src/launcher.rs:236` | `let _ = tokio::time::timeout(5s, browser.wait())` | 进程退出等待超时 |
| `crates/core/src/launcher.rs:257` | `let _ = timeout(5s, browser.close())` | close_role_no_session 同上 |
| `crates/core/src/launcher.rs:258` | `let _ = timeout(5s, browser.wait())` | 同上 |
| `crates/core/src/sandbox.rs:100` | `let _ = timeout(5s, sb.browser.close())` | 沙箱 CDP close 超时 |
| `crates/core/src/sandbox.rs:103` | `let _ = timeout(5s, sb.browser.wait())` | 沙箱进程等待超时 |
| `crates/core/src/sandbox.rs:104` | `let _ = fs::remove_dir_all(&sb.dir)` | 目录删除失败 |
| `crates/core/src/batch.rs:124` | `let _ = store.save(&c)` | 窗口位置配置保存失败 |

handoff 引用 `lib.rs:126` ✅ 准确；`lib.rs:550` ✅ 准确；`launcher.rs:233` ✅ 准确（但 handoff 把它归到推断 4 而非 5，归类有小偏差）。

**全仓库零 `tracing` 依赖**（`Cargo.toml` 无 `tracing` / `tracing-subscriber`），所有诊断靠 `eprintln!`（生产 stderr 用户不可见）。

### Blast Radius

**中**。close 超时是高频事件（Chrome 无响应、网络延迟、系统负载），但用户/开发者完全无法观测：
- 用户只看到「关闭成功」或「关闭失败」，不知道是超时还是 CDP 错误
- 开发者无法从日志定位「为什么这次关闭花了 7s」
- 与推断 (3) 叠加：shutdown 慢 → 用户看不到原因 → 强杀 → 孤儿

### ADR-0006 影响

无冲突。

### 修复方向

1. 引入 `tracing` + `tracing-appender`（handoff §3.1 已规划）。
2. 所有 `let _ = timeout(...)` 改为 `match timeout(...) { Ok(Ok(_)) => debug!(...), Ok(Err(e)) => warn!(...), Err(_) => warn!("close timed out after 5s") }`。
3. `let _ = store.save(...)` 改为 `if let Err(e) = store.save(...) { warn!("save window rect failed: {e}"); }`。

---

## 6. 总结矩阵

| # | 推断 | 裁决 | 核心证据 | 与 ADR-0006 关系 | 修复优先级 |
|---|---|---|---|---|---|
| 1 | delete_role 持锁跨 prune+close ~10s | ✅ 确认 | `src-tauri/src/lib.rs:122-127` 临界区含 1.2s×N + 12s | 无冲突 | **P1** — UI 卡顿直接可感知 |
| 2 | takeover wait() no-op + 2s 轮询短 | ✅ 确认 | chromiumoxide `browser/mod.rs:267-273` + `launcher.rs:240-245` | 间接：可能导致 ADR-0006 false positive | P2 |
| 3 | shutdown 无超时; ExitRequested 空 | ⚠️ 部分确认 | `lib.rs:474-485` 无超时; `lib.rs:652-656` 空处理 | 间接：孤儿触发下次 ADR-0006 硬错误 | **P1** — 数据目录占用根因链 |
| 4 | prune 不杀 Chrome; cleanup_orphans 静默 | ⚠️ 部分反驳 | chromiumoxide `kill_on_drop(true)` → launched Chrome 被杀; `sandbox.rs:110-133` 是沙箱目录 | 无冲突 | P3 — 功能正确但不可观测 |
| 5 | close 超时不记录 | ✅ 确认 | 10+ 处 `let _ =` 吞错; 零 tracing 依赖 | 无冲突 | P2 — 可观测性前置项 |

---

## 7. Handoff 行号准确性汇总

| Handoff 引用 | 实际位置 (master @ 4b20203) | 准确性 |
|---|---|---|
| `lib.rs:99-108` (推断1) | `src-tauri/src/lib.rs:118-130` | ❌ 偏差（99-108 是 get_state） |
| `launcher.rs:285` (推断1) | `crates/core/src/launcher.rs:216-248` (close_role) | ❌ 偏差（285 在 open_tab 内） |
| `launcher.rs:240-244` (推断2) | `crates/core/src/launcher.rs:239-246` | ✅ 基本准确 |
| `launcher.rs:281-296` (推断2) | `crates/core/src/launcher.rs:216-248` (close_role) | ❌ 偏差（281-290 是 open_tab） |
| `lib.rs:613-622` (推断3) | `src-tauri/src/lib.rs:474-485` (shutdown) | ❌ 偏差（613-622 是 TrayIcon 段） |
| `lib.rs:652-656` (推断3) | `src-tauri/src/lib.rs:652-656` | ✅ 准确 |
| `launcher.rs:233` (推断4) | `crates/core/src/launcher.rs:233` (close_role 内 `let _ = timeout(close)`) | ✅ 行号准确，但归类有偏差（这是 close 超时吞错，非 prune 不杀） |
| `lib.rs:126` (推断5) | `src-tauri/src/lib.rs:126` | ✅ 准确 |
| `lib.rs:550` (推断5) | `src-tauri/src/lib.rs:550` | ✅ 准确 |

---

## 8. 给后续落地工单的建议

1. **日志基础设施先行**（handoff §3.1）：引入 tracing 后，推断 4/5 的「不可观测」问题自然消解。
2. **delete_role 拆锁**（推断 1）：参照 `batch::close_one` 模式 — 短锁取 Browser，无锁做 CDP 关闭。
3. **shutdown 并行化 + 超时**（推断 3）：复用 `batch::close_all` 的 spawn + join_all 模式，外层包 30s timeout。
4. **端口轮询提至 5s**（推断 2）：与 `browser.close()` 超时对齐，减少 ADR-0006 false positive。
5. **Windows Job Object**（推断 3 延伸）：作为终极兜底，确保 chameleon 进程退出时所有子 Chrome 被级联终止。
