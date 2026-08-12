//! chameleon-core：变色龙全部领域逻辑。
//!
//! 分层：领域模型（role/config）→ 配置管理（读写/校验/端口分配）→
//! 浏览器控制（检测/启动/会话管理）→ 特性模块（接力/批量/快照/沙箱…）。
//! Tauri 命令层只做薄透传，业务逻辑全部在本库。

use std::future::Future;

pub mod batch;
pub mod browser;
pub mod cleanup;
pub mod config;
pub mod error;
pub mod export;
pub mod handoff;
pub mod launcher;
pub mod model;
pub mod ports;
pub mod quicklinks;
pub mod safety;
pub mod sandbox;
pub mod session;
pub mod single_instance;
pub mod snapshot;
pub mod window;

pub use error::{ChameleonError, Result};
pub use model::{GlobalConfig, QuickLink, Role, System, ThemeMode, UiPreferences, WindowRect};
pub use session::{Session, SessionEvent};

// —— tracing 降级路径辅助 ——

/// 记录 `Result` 的降级路径为 WARN（不阻塞主流程）。
/// 用法：`warn_err(page.activate().await, "page.activate 失败");`
pub fn warn_err<T, E: std::fmt::Display>(result: std::result::Result<T, E>, ctx: &str) {
    if let Err(e) = result {
        tracing::warn!(error = %e, "{ctx}");
    }
}

/// 记录 `timeout` 超时为 WARN（不阻塞主流程；内层错误在关闭/退出场景下属预期，不记）。
/// 用法：`warn_timeout(browser.close(), 5, "Browser.close").await;`
pub(crate) async fn warn_timeout<F: Future>(fut: F, secs: u64, ctx: &str) {
    if tokio::time::timeout(
        std::time::Duration::from_secs(secs),
        fut,
    )
    .await
    .is_err()
    {
        tracing::warn!("{ctx} 超时（{secs}s）");
    }
}

/// `warn_timeout` 的取值版：超时返回 `None` 并记 WARN，正常返回 `Some( fut 的输出 )`。
/// 用于关闭路径里需要结果但绝不能挂死的 CDP 调用（如 `capture_bounds` 对死句柄
/// 会永久阻塞——`Browser::execute` 用 oneshot 等响应、无单命令超时）。
pub(crate) async fn with_timeout<F: Future>(fut: F, secs: u64, ctx: &str) -> Option<F::Output> {
    match tokio::time::timeout(std::time::Duration::from_secs(secs), fut).await {
        Ok(v) => Some(v),
        Err(_) => {
            tracing::warn!("{ctx} 超时（{secs}s）");
            None
        }
    }
}