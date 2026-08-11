//! 临时沙箱：用完即毁的一次性隔离窗口。临时数据目录与任何角色登录态隔离；
//! 关闭后自动删除目录；工具每次启动时扫描并清理崩溃残留的孤儿临时目录。

use crate::browser;
use crate::error::{ChameleonError, Result};
use crate::model::GlobalConfig;
use crate::ports;
use crate::session::{RunningSandbox, Session};
use chromiumoxide::browser::{Browser, BrowserConfig};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 沙箱目录标记文件：清理逻辑只删除带此标记的目录，绝不触碰角色数据目录。
pub const SANDBOX_MARKER: &str = ".chameleon-sandbox";

/// 沙箱信息（返回给前端展示）。
#[derive(Debug, Serialize)]
pub struct SandboxInfo {
    pub id: String,
    pub dir: PathBuf,
}

/// 启动一个临时沙箱窗口。
pub async fn launch(session: &mut Session, cfg: &GlobalConfig) -> Result<SandboxInfo> {
    let browser_path = browser::detect_browser(cfg.browser_path.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();
    let dir = cfg.data_root.join("sandbox").join(&id);
    fs::create_dir_all(&dir)
        .map_err(|e| ChameleonError::SandboxLaunch { detail: e.to_string() })?;
    fs::write(dir.join(SANDBOX_MARKER), "chameleon sandbox")
        .map_err(|e| ChameleonError::SandboxLaunch { detail: e.to_string() })?;
    let port = ports::pick_free_port()?;
    let mut b = BrowserConfig::builder();
    b = if std::env::var_os("CHAMELEON_HEADLESS").is_some() {
        b.new_headless_mode()
    } else {
        b.with_head()
    };
    let mut b = b
        .viewport(None)
        .port(port)
        .user_data_dir(&dir)
        .chrome_executable(browser_path);
    if std::env::var_os("CHAMELEON_NO_SANDBOX").is_some() {
        // ponytail: 测试环境需要 --no-sandbox；生产绝不设置此变量。
        b = b.no_sandbox();
    }
    let config = b
        .launch_timeout(Duration::from_secs(30))
        .arg("no-first-run")
        .arg("no-default-browser-check")
        .window_size(1280, 800)
        .build()
        .map_err(|e| ChameleonError::SandboxLaunch { detail: e })?;
    let (browser, handler) = Browser::launch(config)
        .await
        .map_err(|e| match crate::launcher::classify_launch_err(e) {
            ChameleonError::BrowserStartTimeout => ChameleonError::SandboxLaunch {
                detail: "浏览器未能在 30 秒内开启调试端口，数据目录可能被占用。".into(),
            },
            ChameleonError::BrowserExitedInstantly => ChameleonError::SandboxLaunch {
                detail: "浏览器启动后立即退出，数据目录可能被另一个 Chrome 占用。".into(),
            },
            ChameleonError::LaunchFailed { detail } | ChameleonError::CdpConnectFailed { detail } => {
                ChameleonError::SandboxLaunch { detail }
            }
            _ => ChameleonError::SandboxLaunch {
                detail: "浏览器未能以调试模式启动。".into(),
            },
        })?;
    // 复用 launcher::spawn_handler（含 kill 路径的 Some(Err) 终止修复），
    // 避免 sandbox 自带循环重复实现、且修好「沙箱被 kill 时目录不删」的潜在泄漏。
    let handle = crate::launcher::spawn_handler(handler);
    let dir_for_cleanup = dir.clone();
    let id_for_event = id.clone();
    let tx = session.event_tx.clone();
    tokio::spawn(async move {
        let _ = handle.await;
        // 进程退出 → 删除临时数据目录。滞留子进程可能短暂占用文件，
        // remove_dir_all 失败则重试几次兜底。
        for _ in 0..5 {
            if std::fs::remove_dir_all(&dir_for_cleanup).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        // 通知接收端：外部关时前端刷新 + 提示；工具关（close 已先 remove）静默。
        if let Some(tx) = tx {
            let _ = tx.send(crate::session::SessionEvent::SandboxExited { id: id_for_event });
        }
    });
    session.sandboxes.insert(
        id.clone(),
        RunningSandbox { id: id.clone(), dir: dir.clone(), browser },
    );
    tracing::info!(sandbox_id = %id, sandbox_dir = %dir.display(), "沙箱启动");
    Ok(SandboxInfo { id, dir })
}

/// 关闭沙箱窗口：CDP 优雅关闭后删除其临时数据目录。
pub async fn close(session: &mut Session, id: &str) -> Result<()> {
    let Some(mut sb) = session.sandboxes.remove(id) else {
        return Err(ChameleonError::SandboxNotFound { id: id.into() });
    };
    crate::warn_timeout(sb.browser.close(), 5, "沙箱 Browser.close").await;
    // Browser::close 收到 ACK 即返回，进程仍在退出过程中；先等进程真正退出
    // 再删目录，避免与仍在写 user-data-dir 的进程竞争（同 launcher::close_role）。
    crate::warn_timeout(sb.browser.wait(), 5, "沙箱 Browser.wait").await;
    crate::warn_err(fs::remove_dir_all(&sb.dir), "沙箱目录删除失败");
    tracing::info!(sandbox_id = id, "沙箱关闭");
    Ok(())
}

/// 清理孤儿临时目录：数据根下带沙箱标记、且不在运行集合中的目录
/// （进程崩溃残留），工具启动时调用。
pub fn cleanup_orphans(session_live_ids: &[String], cfg: &GlobalConfig) -> Result<usize> {
    let root = cfg.data_root.join("sandbox");
    if !root.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(&root)
        .map_err(|e| ChameleonError::Io { detail: e.to_string() })?
    {
        let entry = entry.map_err(|e| ChameleonError::Io { detail: e.to_string() })?;
        let p = entry.path();
        if !p.is_dir() || !p.join(SANDBOX_MARKER).exists() {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if session_live_ids.contains(&name.to_string()) {
                continue;
            }
        }
        crate::warn_err(fs::remove_dir_all(&p), "孤儿沙箱目录删除失败");
        removed += 1;
    }
    Ok(removed)
}

/// 数据根下沙箱目录是否存在（供前端判断一键清理可用性，非必需）。
pub fn sandbox_root(cfg: &GlobalConfig) -> PathBuf {
    cfg.data_root.join("sandbox")
}

/// 目录是否带沙箱标记（测试用）。
pub fn is_sandbox_dir(dir: &Path) -> bool {
    dir.join(SANDBOX_MARKER).exists()
}