//! 浏览器检测：自动检测 Chrome/Edge 安装路径（返回所有候选），失败时可手动指定。

use crate::error::{ChameleonError, Result};
use std::path::{Path, PathBuf};

/// 一个检测到的浏览器候选：显示名 + 路径。
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowserCandidate {
    pub name: String,
    pub path: String,
}

/// 常见安装位置（Windows）。按序探测。
fn known_install_paths() -> Vec<(String, PathBuf)> {
    let mut paths = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local);
        paths.push(("Chrome".into(), base.join("Google").join("Chrome").join("Application").join("chrome.exe")));
        paths.push(("Chromium".into(), base.join("Chromium").join("Application").join("chrome.exe")));
        paths.push(("Edge".into(), base.join("Microsoft").join("Edge").join("Application").join("msedge.exe")));
    }
    for drive in ["C:", "D:", "E:"] {
        paths.push(("Chrome".into(), PathBuf::from(format!(
            r"{drive}\Program Files\Google\Chrome\Application\chrome.exe"
        ))));
        paths.push(("Chrome".into(), PathBuf::from(format!(
            r"{drive}\Program Files (x86)\Google\Chrome\Application\chrome.exe"
        ))));
        paths.push(("Edge".into(), PathBuf::from(format!(
            r"{drive}\Program Files\Microsoft\Edge\Application\msedge.exe"
        ))));
        paths.push(("Edge".into(), PathBuf::from(format!(
            r"{drive}\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
        ))));
    }
    paths
}

/// Linux/macOS 开发环境常见路径（单元与集成测试用）。
fn dev_paths() -> Vec<(String, PathBuf)> {
    vec![
        ("Chromium".into(), PathBuf::from("/snap/bin/chromium")),
        ("Chromium".into(), PathBuf::from("/usr/bin/chromium-browser")),
        ("Chromium".into(), PathBuf::from("/usr/bin/chromium")),
        ("Chrome".into(), PathBuf::from("/usr/bin/google-chrome")),
        ("Edge".into(), PathBuf::from("/usr/bin/microsoft-edge")),
        ("Chrome".into(), PathBuf::from("/opt/google/chrome/chrome")),
    ]
}
/// 列出所有检测到的浏览器候选（去重）。
/// 优先级：手动指定 → 注册表 App Paths → 常见安装位置 → PATH → 开发环境。
pub fn list_browser_candidates(manual_override: Option<&Path>) -> Vec<BrowserCandidate> {
    let mut out: Vec<BrowserCandidate> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let push = |out: &mut Vec<BrowserCandidate>, seen: &mut std::collections::HashSet<String>, name: &str, p: PathBuf| {
        let key = p.to_string_lossy().to_lowercase();
        if p.exists() && seen.insert(key) {
            out.push(BrowserCandidate { name: name.into(), path: p.to_string_lossy().to_string() });
        }
    };
    if let Some(p) = manual_override {
        if p.exists() {
            return vec![BrowserCandidate { name: "手动指定".into(), path: p.to_string_lossy().to_string() }];
        }
    }
    for (name, p) in registry_app_paths() { push(&mut out, &mut seen, &name, p); }
    for (name, p) in known_install_paths() { push(&mut out, &mut seen, &name, p); }
    for (name, p) in path_env_candidates() { push(&mut out, &mut seen, &name, p); }
    for (name, p) in dev_paths() { push(&mut out, &mut seen, &name, p); }
    out
}

/// 检测浏览器可执行文件（返回第一个命中）。无任何候选时 BrowserNotFound。
pub fn detect_browser(manual_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = manual_override {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return Err(ChameleonError::LaunchFailed {
            detail: format!("指定的浏览器路径不存在：{}", p.display()),
        });
    }
    let cands = list_browser_candidates(None);
    if let Some(first) = cands.first() {
        return Ok(PathBuf::from(&first.path));
    }
    Err(ChameleonError::BrowserNotFound)
}

/// Windows 注册表 App Paths 探测（chrome.exe / msedge.exe）。
fn registry_app_paths() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for key in [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe",
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe",
    ] {
        if let Some(p) = query_registry_default(key) {
            out.push(("Chrome".into(), p)); // 简化命名，按 key 区分略
        }
    }
    out
}

/// 执行 `reg query <key> //ve` 取默认值（仅 Windows 有效）。
fn query_registry_default(key: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW：reg 是控制台子系统程序，GUI 应用（无父控制台，
        // 如安装版）spawn 它会分配新控制台 → 终端框闪现。必须显式抑制。
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = std::process::Command::new("reg")
            .args(["query", key, "//ve"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.lines().find(|l| l.contains("REG_SZ"))?;
        let path = line.rsplit("REG_SZ").next()?.trim();
        if path.is_empty() {
            None
        } else {
            Some(PathBuf::from(path))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = key;
        None
    }
}

/// PATH 中名为 chrome/chromium/msedge 的可执行文件。
fn path_env_candidates() -> Vec<(String, PathBuf)> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for dir in std::env::split_paths(&path) {
        for name in ["chrome", "chromium", "chromium-browser", "google-chrome", "msedge", "microsoft-edge"] {
            #[cfg(windows)]
            let full = dir.join(format!("{name}.exe"));
            #[cfg(not(windows))]
            let full = dir.join(name);
            out.push((name.into(), full));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_override_must_exist() {
        assert!(matches!(
            detect_browser(Some(Path::new(r"C:\nonexistent\chrome.exe"))),
            Err(ChameleonError::LaunchFailed { .. })
        ));
    }

    #[test]
    fn lists_all_installed_browsers_on_dev_machine() {
        let cands = list_browser_candidates(None);
        // 开发环境至少命中一个，或为空（可接受）
        for c in &cands {
            assert!(std::path::Path::new(&c.path).exists(), "candidate must exist: {}", c.path);
        }
    }
}
