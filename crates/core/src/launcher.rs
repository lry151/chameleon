//! 角色窗口启动 / 连接 / 关闭：隔离参数（独立数据目录 + CDP 端口）、
//! 窗口位置恢复、新标签页、读激活标签、CDP 优雅关闭、
//! 首次启动自动打开预设、登录辅助（半自动填用户名）。

use crate::browser;
use crate::config::{is_writable, ConfigStore};
use crate::error::{ChameleonError, Result};
use crate::model::{GlobalConfig, QuickLinkLogin, Role};
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
    debug_assert!(role.profile_dir.is_absolute(),
        "profile_dir 须绝对（数据根愈合在 load 时），实为 {:?}", role.profile_dir);
    if !is_writable(&role.profile_dir) {
        return Err(ChameleonError::LaunchFailed {
            detail: format!(
                "数据目录不可写：{}。变色龙可能装在受保护位置（如 Program Files），\
                 或 config.json 的 data_root 指向不可写路径。请移到普通文件夹，\
                 或在 config.json 把 data_root 设为可写的绝对路径。",
                role.profile_dir.display()
            ),
        });
    }
    let mut b = BrowserConfig::builder();
    b = if headless() { b.new_headless_mode() } else { b.with_head() };
    let mut b = b
        .viewport(None)
        .port(role.cdp_port)
        .user_data_dir(&role.profile_dir)
        .chrome_executable(browser_path);
    if std::env::var_os("CHAMELEON_NO_SANDBOX").is_some() {
        // ponytail: 测试环境需要 --no-sandbox；生产绝不设置此变量。
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

/// 把 chromiumoxide 启动错误分类为可操作的中文提示（不泄漏原始 stderr 技术栈）。
/// 区分高发原因：启动超时 / 立即退出（多为数据目录被另一 Chrome 占用）→ 独立错误码，
/// 其余归入 LaunchFailed / CdpConnectFailed，detail 一律是自洽中文。
pub fn classify_launch_err(e: chromiumoxide::error::CdpError) -> ChameleonError {
    use chromiumoxide::error::CdpError;
    match e {
        CdpError::LaunchTimeout(_) => ChameleonError::BrowserStartTimeout,
        CdpError::LaunchExit(..) => ChameleonError::BrowserExitedInstantly,
        CdpError::LaunchIo(..) => ChameleonError::LaunchFailed {
            detail: "读取浏览器启动输出失败。".into(),
        },
        CdpError::Io(_) => ChameleonError::LaunchFailed {
            detail: "无法启动浏览器进程，请确认浏览器路径有效且当前用户有运行权限。".into(),
        },
        CdpError::Ws(_) | CdpError::NoResponse => ChameleonError::CdpConnectFailed {
            detail: "与浏览器调试端口握手失败。".into(),
        },
        _ => ChameleonError::LaunchFailed {
            detail: "浏览器未能以调试模式启动，请重试。".into(),
        },
    }
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
/// 首次启动（非接管）后打开角色首页锚点页签；`auto_open=true` 时再自动打开
/// 标记 `auto_open` 的预设（角色级 + 所属系统级）。`auto_open=false`（快照恢复）
/// 抑制默认预设，避免默认页与快照页叠加（ADR-0005）。
pub async fn launch_role(
    session: &mut Session,
    cfg: &GlobalConfig,
    role: &Role,
    auto_open: bool,
) -> Result<()> {
    if session.roles.contains_key(&role.id) {
        return Ok(());
    }
    let browser_path = browser::detect_browser(cfg.browser_path.as_deref())?;
    let was_running = port_open(role.cdp_port);
    if was_running {
        match Browser::connect(format!("http://127.0.0.1:{}", role.cdp_port)).await {
            Ok((browser, handler)) => {
                spawn_handler(handler);
                session.roles.insert(
                    role.id.clone(),
                    RunningRole { browser, active_page: None },
                );
                return Ok(()); // 接管既有实例，不重复打开预设
            }
            Err(e) => {
                // ADR-0006：端口被非浏览器/僵尸实例占用且无法接管 → 硬错误，
                // 不再 fall-through 到 Browser::launch（在占用端口/锁定目录上反复
                // spawn 瞬时窗口 = 「窗口一直闪」根因）。
                tracing::error!(error = %e, role_id = %role.id, cdp_port = role.cdp_port, "端口被占用但 CDP 接管失败");
                return Err(ChameleonError::PortTakenNotRole { port: role.cdp_port });
            }
        }
    }
    let config = build_config(role, &browser_path, cfg)?;
    let (browser, handler) = match Browser::launch(config).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, role_id = %role.id, cdp_port = role.cdp_port, "浏览器启动失败");
            return Err(classify_launch_err(e));
        }
    };
    spawn_handler(handler);
    session.roles.insert(
        role.id.clone(),
        RunningRole { browser, active_page: None },
    );
    tracing::info!(
        role_id = %role.id,
        role_name = %role.name,
        cdp_port = role.cdp_port,
        profile_dir = %role.profile_dir.display(),
        browser_path = %browser_path.display(),
        "角色窗口启动",
    );

    // 首次启动 → 自动打开标记 auto_open 的预设（角色级 + 系统级）；失败跳过不阻塞。
    // 直接 new_page（不走 open_tab）避免 async 互递归。
    if auto_open {
        let auto_urls: Vec<String> = collect_auto_open_urls(role, cfg);
        for url in auto_urls {
            if let Some(run) = session.roles.get_mut(&role.id) {
                if let Ok(page) = run.browser.new_page(CreateTargetParams::new(&url)).await {
                    let id = page.target_id().clone();
                    crate::warn_err(page.activate().await, "启动自动预设 page.activate 失败");
                    run.active_page = Some(id);
                }
            }
        }
    }
    Ok(())
}

/// 启动角色但不修改 Session（并行启动用）：返回建好的 Browser + handler。
/// 调用方负责 spawn handler 并将 Browser 插入 Session。
pub async fn launch_role_no_session(
    cfg: &GlobalConfig,
    role: &Role,
    auto_open: bool,
) -> Result<Browser> {
    let browser_path = browser::detect_browser(cfg.browser_path.as_deref())?;
    let was_running = port_open(role.cdp_port);
    let browser = if was_running {
        match Browser::connect(format!("http://127.0.0.1:{}", role.cdp_port)).await {
            Ok((browser, handler)) => {
                spawn_handler(handler);
                browser
            }
            Err(e) => {
                tracing::error!(error = %e, role_id = %role.id, cdp_port = role.cdp_port, "端口被占用但 CDP 接管失败");
                return Err(ChameleonError::PortTakenNotRole { port: role.cdp_port });
            }
        }
    } else {
        let config = build_config(role, &browser_path, cfg)?;
        let (browser, handler) = match Browser::launch(config).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, role_id = %role.id, cdp_port = role.cdp_port, "浏览器启动失败");
                return Err(classify_launch_err(e));
            }
        };
        spawn_handler(handler);
        browser
    };

    // 自动打开预设（调用方已通过 insert 将 browser 放入 session 后才能执行）
    // 此处只做 URL 收集，页面打开由 batch 层在 insert 后处理。
    if auto_open {
        let _urls = collect_auto_open_urls(role, cfg);
        // auto_open 页面由 batch 层在 insert 后通过 session 操作完成
    }
    Ok(browser)
}

/// 收集角色应自动打开的 URL（角色级 auto_open + 所属系统级 auto_open）。
pub fn collect_auto_open_urls(role: &Role, cfg: &GlobalConfig) -> Vec<String> {
    let mut v: Vec<String> = role
        .quick_links
        .iter()
        .filter(|q| q.auto_open)
        .map(|q| q.url.clone())
        .collect();
    if let Some(sid) = &role.system_id {
        if let Some(sys) = cfg.systems.iter().find(|s| s.id == *sid) {
            v.extend(sys.quick_links.iter().filter(|q| q.auto_open).map(|q| q.url.clone()));
        }
    }
    v
}

/// 优雅关闭角色窗口：先记录窗口位置到配置，再 CDP `Browser.close`，
/// 并轮询 CDP 端口确保真正释放（`Browser::wait` 对 takeover 实例是 no-op，
/// 需要轮询端口来确保接管场景下端口也被释放，避免紧接的启动误判僵尸占用）。
pub async fn close_role(
    session: &mut Session,
    store: &ConfigStore,
    cfg: &mut GlobalConfig,
    role_id: &str,
) -> Result<()> {
    let port = cfg.roles.iter().find(|r| r.id == role_id).map(|r| r.cdp_port);
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
    crate::warn_timeout(browser.close(), 5, "Browser.close").await;
    // 等 Chrome 进程真正退出；对 launched 实例 wait() 等到进程退出，
    // 对 takeover（connect）实例 wait() 立即返回，靠下面轮询端口兜底。
    crate::warn_timeout(browser.wait(), 5, "Browser.wait").await;
    // 兜底轮询端口：takeover 路径下 wait() 不阻塞，Chrome 异步退出期间
    // 端口仍占用，轮询到端口释放或最多 2 秒后返回。
    if let Some(port) = port {
        for _ in 0..20 {
            if !port_open(port) {
                tracing::info!(role_id = role_id, "角色窗口关闭");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    tracing::info!(role_id = role_id, "角色窗口关闭");
    Ok(())
}

/// 优雅关闭角色（不含 Session 操作，并行关闭用）。
/// 调用方负责从 Session 中取出 RunningRole，本函数执行慢速 CDP 工作并返回窗口位置。
pub async fn close_role_no_session(
    mut browser: Browser,
    port: Option<u16>,
) -> Result<Option<crate::model::WindowRect>> {
    let rect = window::capture_bounds(&browser).await.ok();
    crate::warn_timeout(browser.close(), 5, "Browser.close").await;
    crate::warn_timeout(browser.wait(), 5, "Browser.wait").await;
    if let Some(port) = port {
        for _ in 0..20 {
            if !port_open(port) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    Ok(rect)
}

/// 在角色窗口新标签页打开 URL 并聚焦（未启动先拉起）。
pub async fn open_tab(session: &mut Session, cfg: &GlobalConfig, role_id: &str, url: &str) -> Result<()> {
    let role = cfg
        .roles
        .iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    if !session.roles.contains_key(role_id) {
        launch_role(session, cfg, role, true).await?;
    }
    let run = session.roles.get_mut(role_id).expect("just launched");
    let page = run
        .browser
        .new_page(CreateTargetParams::new(url))
        .await
        .map_err(|e| ChameleonError::CdpOperation { detail: e.to_string() })?;
    let id = page.target_id().clone();
    crate::warn_err(page.activate().await, "open_tab page.activate 失败");
    run.active_page = Some(id);
    Ok(())
}

/// 读取角色窗口当前激活标签页 URL。
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

/// 清理「浏览器已被外部直接关闭」的死角色：对每个运行角色做 CDP 存活探测，
/// 超时/失败即移除并返回移除数量。命令前调用，避免对死浏览器的操作挂起
/// （用户直接关 Chrome 后按钮卡死）。
pub async fn prune_dead_roles(session: &mut Session) -> usize {
    let mut dead = Vec::new();
    for (id, run) in &session.roles {
        let alive = matches!(
            tokio::time::timeout(Duration::from_millis(1200), run.browser.version()).await,
            Ok(Ok(_))
        );
        if !alive {
            dead.push(id.clone());
        }
    }
    let n = dead.len();
    for id in &dead {
        session.roles.remove(id);
    }
    n
}

/// 读取角色所有标签页 URL（快照用）。跳过 about:blank 空页签。
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
/// 若 `keep` 为空，则关闭所有标签页。
pub async fn close_other_tabs(session: &mut Session, role_id: &str, keep: &[TargetId]) {
    let Some(run) = session.roles.get(role_id) else {
        return;
    };
    if let Ok(pages) = run.browser.pages().await {
        for page in pages {
            if !keep.contains(page.target_id()) {
                crate::warn_err(page.close().await, "close_other_tabs page.close 失败");
            }
        }
    }
}

/// 关闭所有运行中的角色窗口并记录各自位置（工具退出时调用）。
pub async fn close_all_roles(session: &mut Session, store: &ConfigStore, cfg: &mut GlobalConfig) {
    let ids: Vec<String> = session.roles.keys().cloned().collect();
    for id in ids {
        crate::warn_err(close_role(session, store, cfg, &id).await, "close_all_roles 关闭失败");
    }
}

/// 登录辅助（角色级 LoginConfig）：打开登录页 → 填用户名 → 填密码。
/// 选择器为 None 时自动找（`input[type=password]` + 其前最近的 text/email）。
pub async fn login_assist(session: &mut Session, cfg: &GlobalConfig, role_id: &str) -> Result<()> {
    let role = cfg
        .roles
        .iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?
        .clone();
    let login = role
        .login
        .clone()
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "该角色未配置登录辅助".into() })?;
    if !session.is_role_running(&role.id) {
        launch_role(session, cfg, &role, true).await?;
    }
    let run = session.roles.get_mut(role_id).expect("just launched");
    let page = run
        .browser
        .new_page(CreateTargetParams::new(&login.login_url))
        .await
        .map_err(|e| ChameleonError::CdpOperation { detail: e.to_string() })?;
    let id = page.target_id().clone();
    crate::warn_err(page.activate().await, "login_assist page.activate 失败");
    run.active_page = Some(id);

    let js = build_login_js(&login.username, "", login.username_selector.as_deref(), login.password_selector.as_deref());
    // SPA 延迟渲染：轮询等输入框出现（最多 5s）
    let mut last = String::from("pending");
    for _ in 0..50 {
        match page.evaluate(js.as_str()).await {
            Ok(r) => {
                last = r.into_value::<String>().unwrap_or_default();
                if last == "ok" {
                    return Ok(());
                }
                if last != "no_password" && last != "no_username" {
                    break;
                }
            }
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let detail = match last.as_str() {
        "no_password" => "未找到密码输入框，请手动登录",
        "no_username" => "未找到用户名输入框，请手动登录",
        _ => "登录辅助执行失败，请手动登录",
    };
    Err(ChameleonError::CdpOperation { detail: detail.into() })
}

/// 登录辅助（预设级 QuickLinkLogin）：打开指定 URL → 填用户名 + 密码。
pub async fn login_assist_link(
    session: &mut Session,
    cfg: &GlobalConfig,
    role_id: &str,
    url: &str,
    login: &QuickLinkLogin,
) -> Result<()> {
    let role = cfg
        .roles
        .iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?
        .clone();
    if !session.is_role_running(&role.id) {
        launch_role(session, cfg, &role, true).await?;
    }
    let run = session.roles.get_mut(role_id).expect("just launched");
    let page = run
        .browser
        .new_page(CreateTargetParams::new(url))
        .await
        .map_err(|e| ChameleonError::CdpOperation { detail: e.to_string() })?;
    let id = page.target_id().clone();
    crate::warn_err(page.activate().await, "login_assist_link page.activate 失败");
    run.active_page = Some(id);

    let js = build_login_js(&login.username, &login.password, login.username_selector.as_deref(), login.password_selector.as_deref());
    let mut last = String::from("pending");
    for _ in 0..50 {
        match page.evaluate(js.as_str()).await {
            Ok(r) => {
                last = r.into_value::<String>().unwrap_or_default();
                if last == "ok" {
                    return Ok(());
                }
                if last != "no_password" && last != "no_username" {
                    break;
                }
            }
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let detail = match last.as_str() {
        "no_password" => "未找到密码输入框，请手动登录",
        "no_username" => "未找到用户名输入框，请手动登录",
        _ => "登录辅助执行失败，请手动登录",
    };
    Err(ChameleonError::CdpOperation { detail: detail.into() })
}
/// 构造登录辅助注入 JS（用户名/密码/选择器经 JSON 转义安全嵌入）。
/// 同时填充用户名和密码（不再只填用户名）。
fn build_login_js(username: &str, password: &str, username_selector: Option<&str>, password_selector: Option<&str>) -> String {
    let u = serde_json::to_string(username).unwrap_or_else(|_| "\"\"".into());
    let p = serde_json::to_string(password).unwrap_or_else(|_| "\"\"".into());
    let us = serde_json::to_string(username_selector.unwrap_or("")).unwrap_or_else(|_| "\"\"".into());
    let ps = serde_json::to_string(password_selector.unwrap_or("")).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(function(u, p, usel, psel){{
  var pw = psel ? document.querySelector(psel) : document.querySelector('input[type=password]');
  if (!pw || !pw.offsetParent) return 'no_password';
  var un = usel ? document.querySelector(usel) : null;
  if (!un) {{
    var inputs = Array.prototype.slice.call(document.querySelectorAll('input'));
    var idx = inputs.indexOf(pw);
    for (var i = idx - 1; i >= 0; i--) {{
      var t = (inputs[i].type || '').toLowerCase();
      if (t === 'text' || t === 'email' || t === 'tel') {{ un = inputs[i]; break; }}
    }}
  }}
  if (!un) return 'no_username';
  var setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
  setter.call(un, u);
  un.dispatchEvent(new Event('input', {{ bubbles: true }}));
  un.dispatchEvent(new Event('change', {{ bubbles: true }}));
  setter.call(pw, p);
  pw.dispatchEvent(new Event('input', {{ bubbles: true }}));
  pw.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return 'ok';
}})({}, {}, {}, {})"#,
        u, p, us, ps
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromiumoxide::error::{BrowserStderr, CdpError};

    #[test]
    fn classify_launch_err_maps_timeout_to_start_timeout() {
        let e = CdpError::LaunchTimeout(BrowserStderr::new(Vec::new()));
        assert!(matches!(
            classify_launch_err(e),
            ChameleonError::BrowserStartTimeout
        ));
    }

    #[test]
    fn classify_launch_err_maps_io_to_launch_failed_chinese() {
        let e = CdpError::Io(std::io::Error::new(std::io::ErrorKind::Other, "boom"));
        match classify_launch_err(e) {
            ChameleonError::LaunchFailed { detail } => {
                assert!(detail.contains("无法启动浏览器"), "detail 应为中文: {detail}");
            }
            other => panic!("应为 LaunchFailed，得到 {other:?}"),
        }
    }

    #[test]
    fn build_config_rejects_unwritable_profile_dir() {
        // 显式指向不可写位置（如残留在 Program Files 下的旧 data_root）→ 启动前即清晰报错，
        // 不让 Chrome 弹 "cannot read and write to its data directory"。
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let role = Role::new("x".into(), "#fff".into(), blocker.join("admin"), 9222);
        let cfg = GlobalConfig { data_root: blocker.join("data"), ..Default::default() };
        match build_config(&role, Path::new("dummy-browser"), &cfg) {
            Err(ChameleonError::LaunchFailed { detail }) => {
                assert!(detail.contains("数据目录不可写"), "detail 应点明不可写: {detail}");
            }
            other => panic!("应为 LaunchFailed（数据目录不可写），得到 {other:?}"),
        }
    }
}

