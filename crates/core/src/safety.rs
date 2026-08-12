//! 安全边界：拒绝把角色数据目录指向 Chrome/Edge 默认配置目录（不可协商）。
//!
//! 同时校验：端口冲突、角色名（按系统作用域）/数据目录重复。

use crate::error::{ChameleonError, Result};
use crate::model::{GlobalConfig, Role};
use std::path::{Path, PathBuf};

/// 各类浏览器的默认配置目录（Windows 与 Linux 开发环境）。
/// 命中即拒绝启动，绝不覆盖用户日常浏览器配置。
pub fn default_browser_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local_app_data);
        dirs.push(base.join("Google").join("Chrome").join("User Data"));
        dirs.push(base.join("Microsoft").join("Edge").join("User Data"));
        dirs.push(base.join("Chromium").join("User Data"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let base = PathBuf::from(app_data);
        dirs.push(base.join("Google").join("Chrome").join("User Data"));
        dirs.push(base.join("Microsoft").join("Edge").join("User Data"));
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let base = PathBuf::from(home);
        dirs.push(base.join(".config").join("google-chrome"));
        dirs.push(base.join(".config").join("microsoft-edge"));
        dirs.push(base.join(".config").join("chromium"));
        dirs.push(base.join(".config").join("google-chrome-beta"));
        dirs.push(base.join(".config").join("google-chrome-unstable"));
    }
    dirs
}

/// 浏览器默认配置目录特征段：跨平台静态识别（命中即拒绝），
/// 防止指向 Windows 风格默认目录的路径在任意平台被放行。
const DEFAULT_DIR_MARKERS: [&str; 3] = [
    "Google/Chrome/User Data",
    "Microsoft/Edge/User Data",
    "Chromium/User Data",
];

fn normalized_lower(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

/// 判断目录是否命中默认配置目录（含其子目录）。
pub fn touches_default_dir(dir: &Path) -> Option<PathBuf> {
    if dir.as_os_str().is_empty() {
        return None;
    }
    let abs = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(dir)
    };
    if let Some(d) = default_browser_dirs().into_iter().find(|d| {
        abs == *d || abs.starts_with(d)
    }) {
        return Some(d);
    }
    let norm = normalized_lower(&abs);
    DEFAULT_DIR_MARKERS
        .iter()
        .any(|m| norm.contains(&m.to_lowercase()))
        .then(|| abs)
}

/// 校验角色是否可启动：默认目录拒绝 + 端口冲突 + 重名/重目录。
pub fn validate_role(role: &Role, config: &GlobalConfig) -> Result<()> {
    if let Some(d) = touches_default_dir(&role.profile_dir) {
        return Err(ChameleonError::DefaultDirRefused { dir: d });
    }
    if role.cdp_port == 0 {
        return Err(ChameleonError::PortConflict { port: 0 });
    }
    for other in &config.roles {
        if other.id == role.id {
            continue;
        }
        if other.cdp_port == role.cdp_port {
            return Err(ChameleonError::PortConflict { port: role.cdp_port });
        }
        if other.system_id == role.system_id && other.name == role.name {
            return Err(ChameleonError::DuplicateName { name: role.name.clone() });
        }
        if other.profile_dir == role.profile_dir {
            return Err(ChameleonError::DuplicateDir { dir: role.profile_dir.clone() });
        }
    }
    Ok(())
}

/// 校验整个配置的一致性（导入 / 加载时使用）。
pub fn validate_config(config: &GlobalConfig) -> Result<()> {
    if config.data_root.as_os_str().is_empty() {
        return Err(ChameleonError::ConfigInvalid { detail: "数据根目录不能为空".into() });
    }
    let mut ports = std::collections::HashSet::new();
    // 名称唯一性按 (system_id, name) 作用域：不同系统可重名，同一系统内不可。
    let mut names = std::collections::HashSet::new();
    let mut dirs = std::collections::HashSet::new();
    for role in &config.roles {
        if let Some(d) = touches_default_dir(&role.profile_dir) {
            return Err(ChameleonError::DefaultDirRefused { dir: d });
        }
        if !ports.insert(role.cdp_port) {
            return Err(ChameleonError::PortConflict { port: role.cdp_port });
        }
        if !names.insert((role.system_id.clone(), role.name.clone())) {
            return Err(ChameleonError::DuplicateName { name: role.name.clone() });
        }
        if !dirs.insert(role.profile_dir.clone()) {
            return Err(ChameleonError::DuplicateDir { dir: role.profile_dir.clone() });
        }
        if role.profile_dir.as_os_str().is_empty() {
            return Err(ChameleonError::ConfigInvalid { detail: format!("角色「{}」缺少数据目录", role.name) });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_chrome_default_dir() {
        let d = PathBuf::from(r"C:\Users\tester\AppData\Local\Google\Chrome\User Data");
        assert!(touches_default_dir(&d).is_some());
    }

    #[test]
    fn refuses_default_dir_subdirectory() {
        let d = PathBuf::from(r"C:\Users\tester\AppData\Local\Google\Chrome\User Data\Default");
        assert!(touches_default_dir(&d).is_some());
    }

    #[test]
    fn allows_custom_test_dir() {
        let d = PathBuf::from(r"D:\ChromeTestProfiles\erp-admin");
        assert!(touches_default_dir(&d).is_none());
    }

    #[test]
    fn validation_rejects_default_dir_role() {
        let mut cfg = GlobalConfig::default();
        let role = Role::new(
            "管理员".into(),
            "#e74c3c".into(),
            PathBuf::from(r"C:\Users\tester\AppData\Local\Google\Chrome\User Data"),
            9222,
        );
        cfg.roles.push(role);
        assert!(matches!(
            validate_config(&cfg),
            Err(ChameleonError::DefaultDirRefused { .. })
        ));
    }

    #[test]
    fn validation_allows_same_name_across_systems() {
        let mut cfg = GlobalConfig::default();
        cfg.data_root = PathBuf::from("/tmp/data");
        let mut r1 = Role::new("admin".into(), "#fff".into(), PathBuf::from("/tmp/data/A/admin"), 9222);
        r1.system_id = Some("sys-a".into());
        let mut r2 = Role::new("admin".into(), "#000".into(), PathBuf::from("/tmp/data/B/admin"), 9223);
        r2.system_id = Some("sys-b".into());
        cfg.roles = vec![r1, r2];
        assert!(validate_config(&cfg).is_ok(), "跨系统同名角色应通过校验");
    }

    #[test]
    fn validation_rejects_same_name_within_system() {
        let mut cfg = GlobalConfig::default();
        cfg.data_root = PathBuf::from("/tmp/data");
        let mut r1 = Role::new("admin".into(), "#fff".into(), PathBuf::from("/tmp/data/A/a"), 9222);
        r1.system_id = Some("sys-a".into());
        let mut r2 = Role::new("admin".into(), "#000".into(), PathBuf::from("/tmp/data/A/b"), 9223);
        r2.system_id = Some("sys-a".into());
        cfg.roles = vec![r1, r2];
        assert!(matches!(
            validate_config(&cfg),
            Err(ChameleonError::DuplicateName { .. })
        ));
    }

    #[test]
    fn validation_rejects_duplicate_ports() {
        let mut cfg = GlobalConfig::default();
        let r1 = Role::new("A".into(), "#fff".into(), PathBuf::from("/tmp/a"), 9222);
        let r2 = Role::new("B".into(), "#000".into(), PathBuf::from("/tmp/b"), 9222);
        cfg.roles = vec![r1, r2];
        assert!(matches!(
            validate_config(&cfg),
            Err(ChameleonError::PortConflict { .. })
        ));
    }
}