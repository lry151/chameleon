//! 浏览器检测：自动检测 Chrome/Edge 安装路径，失败时可手动指定。

use crate::error::{ChameleonError, Result};
use std::path::{Path, PathBuf};

/// 常见安装位置（Windows）。按序探测，返回第一个存在的。
fn known_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    // 按用户安装（%LOCALAPPDATA%）：现代 Chrome/Edge 的默认安装位置
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local);
        paths.push(base.join("Google").join("Chrome").join("Application").join("chrome.exe"));
        paths.push(base.join("Chromium").join("Application").join("chrome.exe"));
        paths.push(base.join("Microsoft").join("Edge").join("Application").join("msedge.exe"));
    }
    for drive in ["C:", "D:", "E:"] {
        paths.push(PathBuf::from(format!(
            r"{drive}\Program Files\Google\Chrome\Application\chrome.exe"
        )));
        paths.push(PathBuf::from(format!(
            r"{drive}\Program Files (x86)\Google\Chrome\Application\chrome.exe"
        )));
        paths.push(PathBuf::from(format!(
            r"{drive}\Program Files\Microsoft\Edge\Application\msedge.exe"
        )));
        paths.push(PathBuf::from(format!(
            r"{drive}\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
        )));
    }
    paths
}

/// Linux/macOS 开发环境常见路径（单元与集成测试用）。
fn dev_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/snap/bin/chromium"),
        PathBuf::from("/usr/bin/chromium-browser"),
        PathBuf::from("/usr/bin/chromium"),
        PathBuf::from("/usr/bin/google-chrome"),
        PathBuf::from("/usr/bin/microsoft-edge"),
        PathBuf::from("/opt/google/chrome/chrome"),
    ]
}

/// 检测浏览器可执行文件。
///
/// 优先级：手动指定（config.browser_path）→ Windows 注册表 App Paths →
/// 常见安装位置 → PATH → 开发环境路径。
pub fn detect_browser(manual_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = manual_override {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        return Err(ChameleonError::LaunchFailed {
            detail: format!("指定的浏览器路径不存在：{}", p.display()),
        });
    }

    let mut candidates = Vec::new();
    candidates.extend(registry_app_paths());
    candidates.extend(known_install_paths());
    candidates.extend(path_env_candidates());
    candidates.extend(dev_paths());

    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }
    Err(ChameleonError::BrowserNotFound)
}

/// Windows 注册表 App Paths 探测（chrome.exe / msedge.exe）。
fn registry_app_paths() -> Vec<PathBuf> {
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
            out.push(p);
        }
    }
    out
}

/// 执行 `reg query <key> //ve` 取默认值（仅 Windows 有效）。
fn query_registry_default(key: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("reg")
            .args(["query", key, "//ve"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        // 输出形如: (默认)    REG_SZ    C:\path\chrome.exe
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
fn path_env_candidates() -> Vec<PathBuf> {
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
            out.push(full);
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
    fn detects_an_installed_browser_on_dev_machine() {
        // 开发环境（WSL/CI）至少命中一个已知路径，或返回 BrowserNotFound（可接受）。
        match detect_browser(None) {
            Ok(p) => assert!(p.exists(), "detected path must exist"),
            Err(ChameleonError::BrowserNotFound) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}