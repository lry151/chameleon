# 纯 tracing 日志栈 + tracing-panic + 事件 taxonomy（不引入 tauri-plugin-log）

日志基础设施依 R2 调研（#30）定稿：纯 `tracing` + `tracing-subscriber` + `tracing-appender` 栈，不引入 `tauri-plugin-log`。层栈 = `Registry` + `fmt::layer`（stderr，`EnvFilter` per-layer，dev 受 `RUST_LOG` 控制）+ `fmt::layer`（文件常写，无 filter）+ `tracing-appender::non_blocking`。panic hook = `tracing-panic`（最小、直接进 tracing pipeline → 落 rolling file）。文件路径 = `data_base().join("logs").join("chameleon.log")`（ADR-0011 的可写 `data_base` 是前提，已合 PR #27）。`get_state` 返回 `log_path`；设置页加「打开日志文件夹」命令。

事件 taxonomy：启停 / 角色 launch+close（role_id + cdp_port + profile_dir + browser_path）/ config save = INFO；所有 `let _ =` 降级路径 = WARN；启动失败 = ERROR（完整 detail 进日志，用户只见 `ChameleonError::message` 净化文案）。补全：批量启动/关闭、沙箱、快照、接力操作同样按 INFO/WARN 落日志。红线：绝不全量 log `QuickLinkLogin.password`；CDP 嗅探密码处也不进日志（登录辅助 / 预设级登录注入路径只记 role_id + url，不记 password）。

**Considered Options**: `tauri-plugin-log`——否决，基于 fern + `log` facade，`log::set_boxed_logger` 全局唯一与 `tracing-log::LogTracer` 冲突，chameleon 需要结构化日志 + 按日滚动 + span 追踪，纯 tracing 栈原生支持，plugin 的按大小滚动（默认 40KB）不合需求；`human-panic`——否决，适合 CLI 打印用户友好消息到 stderr，GUI 应用不需要；自建 panic hook——否决，`tracing-panic` 已做好（单函数 ~50 行，直接 emit `tracing::error!`）；按日滚动 `tracing-appender::rolling::daily`——暂缓，间歇使用的桌面工具日志增长有限，固定 `chameleon.log` 追加写已够用，文件膨胀成问题时再加（升级路径明确）。

**Consequences**: 所有错误路径不再盲区——`let _ =` 降级一律 WARN、启动失败完整 detail 落日志；用户只见净化文案，调试可看完整 detail；`WorkerGuard` 在 `run()` 生命周期内持有，drop 时 flush 缓冲；`RUST_LOG` 只影响 stderr，文件常写不受限；release 构建无 console 时 stderr 静默，文件仍写；密码红线 = 登录注入路径的 tracing 事件只记 role_id + url，不记 password 字段；用户可经设置页「打开日志文件夹」直达日志目录。
