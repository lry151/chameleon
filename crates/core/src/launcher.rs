//! 角色窗口启动 / 连接 / 关闭：隔离参数（独立数据目录 + CDP 端口）、
//! 窗口位置恢复、新标签页、读激活标签、CDP 优雅关闭。

use crate::browser;
use crate::config::ConfigStore;
use crate::error::{ChameleonError, Result};
use crate::model::{GlobalConfig, Role};
use crate::safety;
use crate::session::{RunningRole, Session};
use crate::window;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide_cdp::cdp::browser_protocol::target::{CreateTargetParams, TargetId};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// 测试/无显示环境（WSL、CI）置 `CHAMELEON_HEADLESS=1` 走 headless 模式。
fn headless() -> bool {
    std::env::var_os("CHAMELEON_HEADLESS").is_some()
}

/// 构建角色的隔离启动参数。
fn build_config(role: &Role, browser_path: &Path, cfg: &GlobalConfig) -> Result<BrowserConfig> {
    safety::validate_role(role, cfg)?;
    let mut b = BrowserConfig::builder();
    b = if headless() {
        b.new_headless_mode()
    } else {
        b.with_head()
    };
    let mut b = b
        .viewport(None) // 真实窗口，不做视口模拟
        .port(role.cdp_port)
        .user_data_dir(&role.profile_dir)
        .chrome_executable(browser_path);
    if std::env::var_os("CHAMELEON_NO_SANDBOX").is_some() {
        // ponytail: 测试环境（snap chromium / root / CI）需要 --no-sandbox；生产绝不设置此变量。
        b = b.no_sandbox();
    }
    let mut b = b
        .launch_timeout(Duration::from_secs(30))
        .arg("no-first-run")
        .arg("no-default-browser-check");
    if let Some(rect) = role.window_rect {
        let pos = format!("{},{}", rect.x, rect.y);
        b = b
            .window_size(rect.width, rect.height)
            .arg(("window-position", pos.as_str()));
    }
    b.build().map_err(|e| ChameleonError::LaunchFailed { detail: e })
}

/// 端口是否已被占用（用于判断既有实例是否需要接管）。
fn port_open(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// 驱动 chromiumoxide Handler 流（必须被轮询，否则 CDP 请求无响应）。
fn spawn_handler(handler: chromiumoxide::Handler) {
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut handler = handler;
        while handler.next().await.is_some() {}
    });
}

/// 启动角色窗口；已启动则幂等返回。端口被占用且 CDP 可达时接管既有实例。
pub async fn launch_role(session: &mut Session, cfg: &GlobalConfig, role: &Role) -> Result<()> {
    if session.roles.contains_key(&role.id) {
        return Ok(());
    }
    let browser_path = browser::detect_browser(cfg.browser_path.as_deref())?;
    if port_open(role.cdp_port) {
        match Browser::connect(format!("http://127.0.0.1:{}", role.cdp_port)).await {
            Ok((browser, handler)) => {
                spawn_handler(handler);
                session.roles.insert(
                    role.id.clone(),
                    RunningRole { browser, active_page: None },
                );
                return Ok(());
            }
            Err(_) => { /* 端口被非浏览器占用，继续尝试启动 */ }
        }
    }
    let config = build_config(role, &browser_path, cfg)?;
    let (browser, handler) = Browser::launch(config)
        .await
        .map_err(|e| ChameleonError::LaunchFailed { detail: e.to_string() })?;
    spawn_handler(handler);
    session.roles.insert(
        role.id.clone(),
        RunningRole { browser, active_page: None },
    );
    Ok(())
}

/// 优雅关闭角色窗口：先记录窗口位置到配置，再 CDP `Browser.close`。
pub async fn close_role(
    session: &mut Session,
    store: &ConfigStore,
    cfg: &mut GlobalConfig,
    role_id: &str,
) -> Result<()> {
    let Some(run) = session.roles.remove(role_id) else {
        return Err(ChameleonError::RoleNotRunning { id: role_id.into() });
    };
    if let Ok(rect) = window::capture_bounds(&run.browser).await {
        if let Some(slot) = cfg.roles.iter_mut().find(|r| r.id == role_id) {
            slot.window_rect = Some(rect);
            store.save(cfg)?;
        }
    }
    let mut browser = run.browser;
    let _ = tokio::time::timeout(Duration::from_secs(5), browser.close()).await;
    Ok(())
}

/// 在角色窗口新标签页打开 URL 并聚焦（未启动先拉起）。
pub async fn open_tab(session: &mut Session, cfg: &GlobalConfig, role_id: &str, url: &str) -> Result<()> {
    let role = cfg
        .roles
        .iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    if !session.roles.contains_key(role_id) {
        launch_role(session, cfg, role).await?;
    }
    let run = session.roles.get_mut(role_id).expect("just launched");
    let page = run
        .browser
        .new_page(CreateTargetParams::new(url))
        .await
        .map_err(|e| ChameleonError::CdpOperation { detail: e.to_string() })?;
    let id = page.target_id().clone();
    let _ = page.activate().await;
    run.active_page = Some(id);
    Ok(())
}

/// 读取角色窗口当前激活标签页 URL。
///
/// 主路径：工具记录的最近激活标签页；兜底：页签列表中最后一个非空页。
/// 目标角色未登录时读到登录页 URL 属预期（权限差异验证的目的）。
pub async fn read_active_tab(session: &Session, role_id: &str) -> Result<String> {
    let run = session
        .roles
        .get(role_id)
        .ok_or_else(|| ChameleonError::RoleNotRunning { id: role_id.into() })?;
    if let Some(tid) = &run.active_page {
        if let Ok(page) = run.browser.get_page(tid.clone()).await {
            if let Ok(Some(url)) = page.url().await {
                return Ok(url);
            }
        }
    }
    let pages = run
        .browser
        .pages()
        .await
        .map_err(|e| ChameleonError::CdpOperation { detail: e.to_string() })?;
    for page in pages.into_iter().rev() {
        if let Ok(Some(url)) = page.url().await {
            if !url.is_empty() && url != "about:blank" {
                return Ok(url);
            }
        }
    }
    Err(ChameleonError::CdpOperation { detail: "未能读取当前标签页地址".into() })
}

/// 从会话中移除角色（CDP 已断开等场景）。
pub fn drop_role(session: &mut Session, role_id: &str) {
    session.roles.remove(role_id);
}

/// 读取角色所有标签页 URL（快照用）。
pub async fn list_tab_urls(session: &Session, role_id: &str) -> Vec<String> {
    let Some(run) = session.roles.get(role_id) else {
        return Vec::new();
    };
    let Ok(pages) = run.browser.pages().await else {
        return Vec::new();
    };
    let mut urls = Vec::new();
    for p in pages {
        if let Ok(Some(u)) = p.url().await {
            if !u.is_empty() && u != "about:blank" {
                urls.push(u);
            }
        }
    }
    urls
}

/// 关闭角色窗口内除 `keep` 外的所有标签页（快照恢复用）。
pub async fn close_other_tabs(session: &mut Session, role_id: &str, keep: &[TargetId]) {
    let Some(run) = session.roles.get(role_id) else {
        return;
    };
    if keep.is_empty() {
        return; // 不关闭最后一个标签页，避免窗口关闭
    }
    if let Ok(pages) = run.browser.pages().await {
        for page in pages {
            if !keep.contains(page.target_id()) {
                let _ = page.close().await;
            }
        }
    }
}

/// 关闭所有运行中的角色窗口并记录各自位置（工具退出时调用）。
pub async fn close_all_roles(session: &mut Session, store: &ConfigStore, cfg: &mut GlobalConfig) {
    let ids: Vec<String> = session.roles.keys().cloned().collect();
    for id in ids {
        let _ = close_role(session, store, cfg, &id).await;
    }
}