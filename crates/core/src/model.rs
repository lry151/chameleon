//! 领域模型：角色（Role）、全局配置（GlobalConfig）、快照、窗口位置。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 常用 URL 预设：名称 + 地址，点击即在角色窗口新标签页打开。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickLink {
    pub name: String,
    pub url: String,
}

/// 窗口位置与大小（上次记忆，用于启动恢复）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 角色（Role）：命名隔离单元 = 名称 + 颜色 + 数据目录 + CDP 端口 + 常用 URL 列表。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Role {
    pub id: String,
    pub name: String,
    /// 色块颜色，十六进制如 "#e74c3c"。
    pub color: String,
    /// 角色专属数据目录（--user-data-dir），与默认配置目录严格互斥。
    pub profile_dir: PathBuf,
    /// 该角色独立的 CDP 调试端口，创建时分配并持久化，重启不变。
    pub cdp_port: u16,
    pub quick_links: Vec<QuickLink>,
    /// 上次窗口位置，None 表示未记忆过。
    pub window_rect: Option<WindowRect>,
}

impl Role {
    pub fn new(name: String, color: String, profile_dir: PathBuf, cdp_port: u16) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            color,
            profile_dir,
            cdp_port,
            quick_links: Vec::new(),
            window_rect: None,
        }
    }
}

/// 全局配置：角色列表 + 浏览器路径 / 数据根目录等全局设置。
/// config.json 为唯一配置源，明文 JSON，人工可改。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 手动指定的浏览器路径（Chrome 或 Edge），None 表示自动检测。
    pub browser_path: Option<PathBuf>,
    /// 测试数据根目录，所有角色数据目录 / 沙箱目录默认放这里。
    pub data_root: PathBuf,
    pub roles: Vec<Role>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            browser_path: None,
            data_root: PathBuf::from("data"),
            roles: Vec::new(),
        }
    }
}

/// 会话快照：某一时刻所有角色的打开标签页 URL 与窗口位置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub name: String,
    pub created_at: String,
    pub roles: Vec<SnapshotRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRole {
    pub role_id: String,
    pub role_name: String,
    pub tabs: Vec<String>,
    pub window_rect: Option<WindowRect>,
}