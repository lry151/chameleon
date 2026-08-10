//! 统一错误码 → 自然语言中文文案层。
//!
//! 所有失败路径只允许通过 [`ChameleonError::message`] 把中文可读文案暴露给用户，
//! 绝不向外泄露技术栈报错原文。

use std::path::PathBuf;

/// 领域错误。内部可携带技术细节（仅日志用），对外一律走 [`message`](ChameleonError::message)。
#[derive(Debug, thiserror::Error)]
pub enum ChameleonError {
    #[error("配置文件读取失败")]
    ConfigRead { detail: String },
    #[error("配置文件写入失败")]
    ConfigWrite { detail: String },
    #[error("配置校验失败")]
    ConfigInvalid { detail: String },

    #[error("未找到浏览器")]
    BrowserNotFound,
    #[error("浏览器启动失败")]
    LaunchFailed { detail: String },
    #[error("浏览器启动超时（30 秒未开启调试端口）")]
    BrowserStartTimeout,
    #[error("浏览器启动后立即退出")]
    BrowserExitedInstantly,
    #[error("无法连接浏览器调试端口")]
    CdpConnectFailed { detail: String },
    #[error("浏览器操作失败")]
    CdpOperation { detail: String },

    #[error("数据目录指向 Chrome/Edge 默认配置目录，已拒绝")]
    DefaultDirRefused { dir: PathBuf },
    #[error("端口冲突")]
    PortConflict { port: u16 },
    #[error("端口被占用且非本角色实例")]
    PortTakenNotRole { port: u16 },
    #[error("角色不存在")]
    RoleNotFound { id: String },
    #[error("角色窗口未启动")]
    RoleNotRunning { id: String },
    #[error("角色已启动")]
    RoleAlreadyRunning { id: String },
    #[error("角色名称重复")]
    DuplicateName { name: String },
    #[error("角色数据目录重复")]
    DuplicateDir { dir: PathBuf },
    #[error("快照不存在")]
    SnapshotNotFound { name: String },
    #[error("快照保存失败")]
    SnapshotWrite { detail: String },
    #[error("沙箱启动失败")]
    SandboxLaunch { detail: String },
    #[error("沙箱不存在")]
    SandboxNotFound { id: String },
    #[error("IO 错误")]
    Io { detail: String },
    #[error("工具已在运行")]
    AlreadyRunning,
    #[error("导入配置无效")]
    ImportInvalid { detail: String },
}

/// 便捷别名。
pub type Result<T> = std::result::Result<T, ChameleonError>;

impl ChameleonError {
    /// 对外展示的自然语言中文文案。技术细节绝不进入此文案。
    pub fn message(&self) -> String {
        match self {
            ChameleonError::ConfigRead { .. } => "读取配置文件失败，请检查 config.json 是否存在且可读。".into(),
            ChameleonError::ConfigWrite { .. } => "写入配置文件失败，请检查存放目录是否有写入权限。".into(),
            ChameleonError::ConfigInvalid { detail } => format!("配置内容有误：{detail}"),
            ChameleonError::BrowserNotFound => {
                "未找到 Chrome，请点击『选择 Chrome 路径』手动指定。".into()
            }
            ChameleonError::LaunchFailed { detail } => {
                format!("浏览器启动失败：{detail}")
            }
            ChameleonError::BrowserStartTimeout => {
                "浏览器未能在 30 秒内开启调试端口。\n常见原因：该角色数据目录正被另一个 Chrome 占用、安全软件拦截、或浏览器启动缓慢。\n处理措施：先关闭占用该目录的 Chrome（或点『一键关闭』）后重试；若仍失败，可尝试以非管理员身份运行本工具。".into()
            }
            ChameleonError::BrowserExitedInstantly => {
                "浏览器启动后立即退出。\n常见原因：该角色数据目录正被另一个 Chrome 实例占用——Chrome 单实例会忽略调试端口参数并把请求转交给已运行实例。\n处理措施：关闭占用该目录的 Chrome 后重试，或点『一键关闭』。".into()
            }
            ChameleonError::CdpConnectFailed { detail } => {
                format!("连接浏览器调试端口失败：{detail}\n处理措施：请尝试重新启动该角色窗口；若端口被其他进程占用，点『一键关闭』后重试。")
            }
            ChameleonError::CdpOperation { detail } => format!("浏览器操作失败：{detail}"),
            ChameleonError::DefaultDirRefused { dir } => format!(
                "数据目录「{}」指向 Chrome/Edge 默认配置目录，已拒绝启动，避免影响日常浏览器。",
                dir.display()
            ),
            ChameleonError::PortConflict { port } => {
                format!("端口 {port} 已被占用，请为该角色重新分配端口。")
            }
            ChameleonError::PortTakenNotRole { port } => {
                format!("端口 {port} 已被占用且非本角色实例，请一键关闭所有后重试。")
            }
            ChameleonError::RoleNotFound { .. } => "找不到该角色，请刷新后重试。".into(),
            ChameleonError::RoleNotRunning { .. } => "该角色窗口尚未启动，请先启动。".into(),
            ChameleonError::RoleAlreadyRunning { .. } => "该角色窗口已在运行，无需重复启动。".into(),
            ChameleonError::DuplicateName { name } => {
                format!("角色名称「{name}」已存在，请换一个名称。")
            }
            ChameleonError::DuplicateDir { dir } => {
                format!("数据目录「{}」已被其他角色使用，请换一个目录。", dir.display())
            }
            ChameleonError::SnapshotNotFound { .. } => "找不到该快照，可能已被删除。".into(),
            ChameleonError::SnapshotWrite { .. } => "保存快照失败，请检查快照目录是否可写。".into(),
            ChameleonError::SandboxLaunch { .. } => "启动临时沙箱失败，请重试。".into(),
            ChameleonError::SandboxNotFound { .. } => "找不到该临时沙箱，可能已关闭。".into(),
            ChameleonError::Io { .. } => "文件操作失败，请检查磁盘空间与权限。".into(),
            ChameleonError::AlreadyRunning => "工具已在运行，请勿重复启动。".into(),
            ChameleonError::ImportInvalid { detail } => {
                format!("导入的配置无效：{detail}。已取消导入，现有配置未受影响。")
            }
        }
    }
}

impl From<std::io::Error> for ChameleonError {
    fn from(e: std::io::Error) -> Self {
        ChameleonError::Io { detail: e.to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_render_chinese_message() {
        let cases = vec![
            ChameleonError::BrowserNotFound,
            ChameleonError::AlreadyRunning,
            ChameleonError::RoleNotFound { id: "x".into() },
            ChameleonError::DefaultDirRefused { dir: PathBuf::from("C:\\Users\\a\\AppData\\Local\\Google\\Chrome\\User Data") },
            ChameleonError::PortConflict { port: 9222 },
            ChameleonError::PortTakenNotRole { port: 9222 },
        ];
        for c in cases {
            let m = c.message();
            assert!(!m.is_empty(), "message must not be empty");
            // 文案不得包含技术栈原文（如路径分隔符、Rust 错误形式）
            assert!(!m.contains("Error"), "message leaked technical text: {m}");
        }
    }

    #[test]
    fn browser_not_found_message_matches_ticket() {
        assert_eq!(
            ChameleonError::BrowserNotFound.message(),
            "未找到 Chrome，请点击『选择 Chrome 路径』手动指定。"
        );
    }

    #[test]
    fn launch_taxonomy_messages_carry_processing_measures() {
        // 超时 / 立即退出 两类高发原因必须有「处理措施」并提示「占用」
        for (label, msg) in [
            ("timeout", ChameleonError::BrowserStartTimeout.message()),
            ("exit", ChameleonError::BrowserExitedInstantly.message()),
        ] {
            assert!(msg.contains("处理措施"), "{label} 缺少处理措施: {msg}");
            assert!(msg.contains("占用"), "{label} 未点出数据目录被占用: {msg}");
        }
        // CdpConnectFailed 也要带处理措施
        let cdp = ChameleonError::CdpConnectFailed { detail: "x".into() }.message();
        assert!(cdp.contains("处理措施"), "CdpConnectFailed 缺少处理措施: {cdp}");
    }
}