//! 角色窗口启动 / 连接 / 关闭：隔离参数（独立数据目录 + CDP 端口）、
//! 窗口位置恢复、新标签页、读激活标签、CDP 优雅关闭、
//! 首次启动自动打开预设、登录辅助（半自动填用户名）。

use crate::browser;
use crate::config::ConfigStore;
use crate::error::{ChameleonError, Result};
use crate::model::{GlobalConfig, LoginConfig, Role};
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

/// 角色首页锚点页签的识别标记（data URL 中该属性名原样保留，用于快照过滤/恢复定位）。
const ROLE_HOME_MARKER: &str = "data-chameleon-role-home";

/// 构建角色的隔离启动参数。
fn build_config(role: &Role, browser_path: &Path, cfg: &GlobalConfig) -> Result<BrowserConfig> {
    safety::validate_role(role, cfg)?;
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
            Err(_) => {
                // ADR-0006：端口被非浏览器/僵尸实例占用且无法接管 → 硬错误，
                // 不再 fall-through 到 Browser::launch（在占用端口/锁定目录上反复
                // spawn 瞬时窗口 = 「窗口一直闪」根因）。
                return Err(ChameleonError::PortTakenNotRole { port: role.cdp_port });
            }
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
    // 角色首页锚点页签：文档标题=角色名，启动时窗口一眼可辨（切其他页签标题变网站名，Chrome 限制）。
    let home = role_home_url(role);
    if let Some(run) = session.roles.get_mut(&role.id) {
        if let Ok(page) = run.browser.new_page(CreateTargetParams::new(&home)).await {
            let id = page.target_id().clone();
            let _ = page.activate().await;
            run.active_page = Some(id);
        }
    }
    // 首次启动 → 自动打开标记 auto_open 的预设（角色级 + 系统级）；失败跳过不阻塞。
    // 直接 new_page（不走 open_tab）避免 async 互递归。
    if auto_open {
        let auto_urls: Vec<String> = collect_auto_open_urls(role, cfg);
        for url in auto_urls {
            if let Some(run) = session.roles.get_mut(&role.id) {
                if let Ok(page) = run.browser.new_page(CreateTargetParams::new(&url)).await {
                    let id = page.target_id().clone();
                    let _ = page.activate().await;
                    run.active_page = Some(id);
                }
            }
        }
    }
    Ok(())
}

/// 收集角色应自动打开的 URL（角色级 auto_open + 所属系统级 auto_open）。
fn collect_auto_open_urls(role: &Role, cfg: &GlobalConfig) -> Vec<String> {
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
    let _ = tokio::time::timeout(Duration::from_secs(5), browser.close()).await;
    // 等 Chrome 进程真正退出；对 launched 实例 wait() 等到进程退出，
    // 对 takeover（connect）实例 wait() 立即返回，靠下面轮询端口兜底。
    let _ = tokio::time::timeout(Duration::from_secs(5), browser.wait()).await;
    // 兜底轮询端口：takeover 路径下 wait() 不阻塞，Chrome 异步退出期间
    // 端口仍占用，轮询到端口释放或最多 2 秒后返回。
    if let Some(port) = port {
        for _ in 0..20 {
            if !port_open(port) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
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
        launch_role(session, cfg, role, true).await?;
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

/// 探测角色浏览器是否可响应（CDP 短超时），防半开连接的操作挂死。
pub async fn is_role_alive(session: &Session, role_id: &str) -> bool {
    let Some(run) = session.roles.get(role_id) else {
        return false;
    };
    matches!(
        tokio::time::timeout(Duration::from_millis(1200), run.browser.version()).await,
        Ok(Ok(_))
    )
}

/// 清理「浏览器已被外部直接关闭」的死角色：对每个运行角色做 CDP 存活探测，
/// 超时/失败即移除并返回移除数量。UI 刷新与角色命令前调用，避免对死浏览器
/// 的操作无限挂起（用户直接关 Chrome 后按钮卡死）。
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

/// 读取角色所有标签页 URL（快照用）。跳过 chameleon 角色首页锚点页签
/// （data URL 含标记），快照只记真实测试页。
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
            if !u.is_empty() && u != "about:blank" && !u.contains(ROLE_HOME_MARKER) {
                urls.push(u);
            }
        }
    }
    urls
}

/// 定位角色首页锚点页签的 target id（快照恢复时纳入 close_other_tabs 的 keep 集，
/// 保住窗口标题角色名）。无锚点页签返回 None。
pub async fn find_role_home_tab(session: &Session, role_id: &str) -> Option<TargetId> {
    let run = session.roles.get(role_id)?;
    let pages = run.browser.pages().await.ok()?;
    for p in pages {
        if let Ok(Some(u)) = p.url().await {
            if u.contains(ROLE_HOME_MARKER) {
                return Some(p.target_id().clone());
            }
        }
    }
    None
}

/// 关闭角色窗口内除 `keep` 外的所有标签页（快照恢复用）。
pub async fn close_other_tabs(session: &mut Session, role_id: &str, keep: &[TargetId]) {
    let Some(run) = session.roles.get(role_id) else {
        return;
    };
    if keep.is_empty() {
        return;
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

/// 登录辅助：打开登录页 → 等输入框出现 → 自动填用户名 → 聚焦密码框等用户手输。
/// 不存储密码。选择器为 None 时自动找（`input[type=password]` + 其前最近的 text/email）。
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
    let _ = page.activate().await;
    run.active_page = Some(id);

    let js = build_login_js(&login);
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

/// 构造登录辅助注入 JS（用户名/选择器经 JSON 转义安全嵌入）。
fn build_login_js(login: &LoginConfig) -> String {
    let u = serde_json::to_string(&login.username).unwrap_or_else(|_| "\"\"".into());
    let us = serde_json::to_string(login.username_selector.as_deref().unwrap_or("")).unwrap_or_else(|_| "\"\"".into());
    let ps = serde_json::to_string(login.password_selector.as_deref().unwrap_or("")).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"(function(u, usel, psel){{
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
  pw.focus();
  return 'ok';
}})({}, {}, {})"#,
        u, us, ps
    )
}

/// 角色首页锚点页签 URL：文档标题=角色名，启动时窗口标题一眼可辨。
/// body 带 `data-chameleon-role-home` 标记，供快照过滤与恢复定位锚点页签。
fn role_home_url(role: &Role) -> String {
    let html = format!(
        "<!doctype html><html><head><meta charset=utf-8><title>{} — chameleon</title></head><body data-chameleon-role-home style='margin:0;height:100vh;display:flex;align-items:center;justify-content:center;background:#1c1b22;color:#ecebf1;font-family:sans-serif'><div style='text-align:center'><div style='width:72px;height:72px;border-radius:16px;background:{};margin:0 auto 14px;box-shadow:0 0 0 3px #00000033'></div><h1 style='margin:0 0 4px'>{}</h1><p style='color:#9a98a8;margin:0'>chameleon 角色窗口 · 切换标签页标题会变</p></div></body></html>",
        role.name, role.color, role.name
    );
    format!("data:text/html;charset=utf-8,{}", data_encode(&html))
}

/// data URL 百分号编码：非字母数字符号按 UTF-8 字节 %XX（避免引 percent-encoding 新依赖）。
fn data_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}