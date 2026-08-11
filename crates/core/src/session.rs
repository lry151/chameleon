//! 运行时会话状态：当前受控的角色窗口与临时沙箱。
//!
//! 每个运行中的角色对应一个已连接的 CDP `Browser` 句柄；Tauri 命令层通过
//! `Arc<Mutex<Session>>` 共享本状态。

use chromiumoxide::browser::Browser;
use chromiumoxide_cdp::cdp::browser_protocol::target::TargetId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

/// 运行时会话向 UI 推送的事件。
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// 角色的浏览器连接断开（窗口被关 / 进程退出 / 崩溃）。
    /// 由 launch 的 watcher 发出。接收端据 `roles.remove(&id)` 是否命中区分：
    /// 命中（Some）= 外部/意外关闭 → 提示用户；未命中（None）= 工具主动关闭
    RoleExited { id: String },
    /// 沙箱窗口被关 / 进程退出。沙箱无名，前端提示用「沙箱」即可。
    /// 接收端据 `sandboxes.remove(&id)` 是否命中区分外部关 vs 工具关（同 RoleExited）。
    SandboxExited { id: String },
}
/// 运行中的角色窗口。
pub struct RunningRole {
    pub browser: Browser,
    /// 最近一次由工具打开/激活的标签页。
    ///
    /// ponytail: 用户手动切换标签页无法从 CDP 直接感知，接力读取以此为主、
    /// 页签列表为兜底；升级路径 = 订阅 Target.targetInfoChanged + OS 窗口焦点事件。
    pub active_page: Option<TargetId>,
}

/// 运行中的临时沙箱窗口。
pub struct RunningSandbox {
    pub id: String,
    /// 沙箱专属临时数据目录，关闭后删除。
    pub dir: PathBuf,
    pub browser: Browser,
}

/// 会话状态：`role_id -> 运行中角色`；`沙箱 id -> 运行中沙箱`。
#[derive(Default)]
pub struct Session {
    pub roles: HashMap<String, RunningRole>,
    pub sandboxes: HashMap<String, RunningSandbox>,
    /// 角色被动退出事件通道。None = 不监听（测试 / 非 Tauri 上下文），
    /// launch 时不 spawn watcher，回退到命令前的 on-demand prune。
    pub event_tx: Option<Arc<UnboundedSender<SessionEvent>>>,
}

impl Session {
    pub fn is_role_running(&self, role_id: &str) -> bool {
        self.roles.contains_key(role_id)
    }
}