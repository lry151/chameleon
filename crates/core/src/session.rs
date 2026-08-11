//! 运行时会话状态：当前受控的角色窗口与临时沙箱。
//!
//! 每个运行中的角色对应一个已连接的 CDP `Browser` 句柄；Tauri 命令层通过
//! `Arc<Mutex<Session>>` 共享本状态。

use chromiumoxide::browser::Browser;
use chromiumoxide_cdp::cdp::browser_protocol::target::TargetId;
use std::collections::HashMap;
use std::path::PathBuf;

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
}

impl Session {
    pub fn is_role_running(&self, role_id: &str) -> bool {
        self.roles.contains_key(role_id)
    }
}
