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
        .map_err(|e| ChameleonError::SandboxLaunch { detail: e.to_string() })?;
    let dir_for_cleanup = dir.clone();
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut handler = handler;
        // Handler 流在浏览器关闭（CDP 关闭或用户直接点 X）时结束
        while handler.next().await.is_some() {}
        // 进程退出 → 删除临时数据目录（spec：工具监听沙箱进程退出即删除）。
        // 滞留的子进程可能短暂占用文件，remove_dir_all 失败则重试几次兜底。
        for _ in 0..5 {
            if std::fs::remove_dir_all(&dir_for_cleanup).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
    session.sandboxes.insert(
        id.clone(),
        RunningSandbox { id: id.clone(), dir: dir.clone(), browser },
    );
    Ok(SandboxInfo { id, dir })
}

/// 关闭沙箱窗口：CDP 优雅关闭后删除其临时数据目录。
pub async fn close(session: &mut Session, id: &str) -> Result<()> {
    let Some(mut sb) = session.sandboxes.remove(id) else {
        return Err(ChameleonError::SandboxNotFound { id: id.into() });
    };
    let _ = tokio::time::timeout(Duration::from_secs(5), sb.browser.close()).await;
    // Browser::close 收到 ACK 即返回，进程仍在退出过程中；先等进程真正退出
    // 再删目录，避免与仍在写 user-data-dir 的进程竞争（同 launcher::close_role）。
    let _ = tokio::time::timeout(Duration::from_secs(5), sb.browser.wait()).await;
    let _ = fs::remove_dir_all(&sb.dir);
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
        let _ = fs::remove_dir_all(&p);
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