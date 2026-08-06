//! 领域模型：角色、系统、全局配置、登录辅助、快照、窗口位置。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 常用 URL 预设：名称 + 地址 + 是否启动时自动打开。
/// 可选挂登录凭据（含密码）—— 点击该预设时自动打开 URL 并填用户名/密码。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickLink {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub auto_open: bool,
    /// 挂在此预设上的登录凭据；Some 时点击该预设触发自动登录（填用户名+密码）。
    #[serde(default)]
    pub login: Option<QuickLinkLogin>,
}

/// 预设级登录凭据：挂在 QuickLink 上的自动登录配置（存密码，角色级隔离）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickLinkLogin {
    pub username: String,
    pub password: String,
    /// 用户名输入框 CSS 选择器；None = 自动找（密码框前最近的 text/email）。
    pub username_selector: Option<String>,
    /// 密码输入框 CSS 选择器；None = `input[type=password]`。
    pub password_selector: Option<String>,
}

/// 登录辅助：半自动登录配置（不存密码）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginConfig {
    pub login_url: String,
    pub username: String,
    /// 用户名输入框 CSS 选择器；None = 自动找（密码框前最近的 text/email）。
    pub username_selector: Option<String>,
    /// 密码输入框 CSS 选择器；None = `input[type=password]`。
    pub password_selector: Option<String>,
}

/// 窗口位置与大小（上次记忆，用于启动恢复）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 角色（Role）：命名隔离单元 = 名称 + 颜色 + 数据目录 + CDP 端口 + 预设 + 登录辅助。
/// 可选属于一个系统；各角色数据目录与登录态严格隔离。
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
    /// 所属系统；None = 未分组。
    #[serde(default)]
    pub system_id: Option<String>,
    /// 登录辅助配置；None = 无。
    #[serde(default)]
    pub login: Option<LoginConfig>,
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
            system_id: None,
            login: None,
        }
    }
}

/// 系统 (System)：被测应用的命名容器，含多角色 + 系统级常用 URL 预设（组内共享）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct System {
    pub id: String,
    pub name: String,
    pub quick_links: Vec<QuickLink>,
}

impl System {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            quick_links: Vec::new(),
        }
    }
}

/// 主题模式：深色 / 浅色 / 跟随系统。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

/// UI 偏好：主题 + 面板透明度 + Accent 颜色。持久化到 config.json。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiPreferences {
    pub theme: ThemeMode,
    /// 面板透明度 0.5–1.0。
    pub panel_opacity: f32,
    /// Accent 颜色，十六进制如 "#1abc9c"。
    pub accent_color: String,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Dark,
            panel_opacity: 0.72,
            accent_color: "#1abc9c".into(),
        }
    }
}

/// 全局配置：角色列表 + 系统列表 + 浏览器路径 / 数据根目录等全局设置。
/// config.json 为唯一配置源，明文 JSON，人工可改。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 手动指定的浏览器路径（Chrome 或 Edge），None 表示自动检测。
    pub browser_path: Option<PathBuf>,
    /// 测试数据根目录，所有角色数据目录 / 沙箱目录默认放这里。
    pub data_root: PathBuf,
    pub roles: Vec<Role>,
    #[serde(default)]
    pub systems: Vec<System>,
    /// UI 偏好（主题/透明度/Accent）；旧配置自动填充默认值。
    #[serde(default)]
    pub ui_preferences: UiPreferences,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            browser_path: None,
            data_root: PathBuf::from("data"),
            roles: Vec::new(),
            systems: Vec::new(),
            ui_preferences: UiPreferences::default(),
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
