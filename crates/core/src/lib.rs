//! chameleon-core：变色龙全部领域逻辑。
//!
//! 分层：领域模型（role/config）→ 配置管理（读写/校验/端口分配）→
//! 浏览器控制（检测/启动/会话管理）→ 特性模块（接力/批量/快照/沙箱…）。
//! Tauri 命令层只做薄透传，业务逻辑全部在本库。

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
pub use model::{GlobalConfig, LoginConfig, QuickLink, Role, System, ThemeMode, UiPreferences, WindowRect};
pub use session::Session;