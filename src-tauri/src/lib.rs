//! 变色龙 Tauri 外壳：薄壳透传 chameleon-core 的全部领域逻辑。
//!
//! 命令层只做参数搬运与错误文案映射（统一走 `ChameleonError::message` 的中文文案）。

use chameleon_core::{
    batch::BatchResult,
    browser::BrowserCandidate,
    config::{app_dir, data_base, ConfigStore},
    export,
    handoff::HandoffMode,
    launcher,
    model::{QuickLinkLogin, Role, System, UiPreferences},
    quicklinks, sandbox,
    snapshot::SnapshotStore,
    ChameleonError, Session, SessionEvent,
};
use tauri::Emitter;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WindowEvent,
};
use tauri_plugin_dialog::DialogExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

mod vibrancy;

fn msg(e: ChameleonError) -> String {
    e.message()
}

/// 初始化 tracing 日志栈：dev stderr（受 `RUST_LOG` 控制）+ 文件常写（`chameleon.log`）。
/// 返回 (log_path, guard)。guard 须在 app 生命周期内持有，drop 时 flush 缓冲。
fn init_logging(log_dir: PathBuf) -> (PathBuf, tracing_appender::non_blocking::WorkerGuard) {
    let log_path = log_dir.join("chameleon.log");
    let _ = std::fs::create_dir_all(&log_dir);
    // data_base 可写性由 ADR-0011 探优；兜底 stderr（不丢日志）。
    let writer: Box<dyn std::io::Write + Send> = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(f) => Box::new(f),
        Err(_) => Box::new(std::io::stderr()),
    };
    let (non_blocking, guard) = tracing_appender::non_blocking(writer);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(EnvFilter::from_default_env()),
        )
        .with(fmt::layer().with_writer(non_blocking))
        .init();

    (log_path, guard)
}

/// 在 OS 文件管理器中打开目录（跨平台）。
fn open_dir(dir: &std::path::Path) -> std::result::Result<(), std::io::Error> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(dir).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(dir).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(dir).spawn()?;
    }
    Ok(())
}

pub struct AppState {
    pub session: Arc<tokio::sync::Mutex<Session>>,
    pub app_dir: PathBuf,
    pub log_path: PathBuf,
}

impl AppState {
    fn store(&self) -> ConfigStore {
        ConfigStore::new(self.app_dir.join("config.json"))
    }
    fn snapshots(&self) -> SnapshotStore {
        SnapshotStore::new(self.app_dir.join("snapshots"))
    }
}

#[derive(Serialize)]
pub struct RoleView {
    #[serde(flatten)]
    pub role: Role,
    pub running: bool,
}

#[derive(Serialize)]
pub struct AppStateView {
    pub roles: Vec<RoleView>,
    pub systems: Vec<System>,
    pub sandboxes: Vec<sandbox::SandboxInfo>,
    pub snapshots: Vec<String>,
    pub browser_path: Option<String>,
    pub browser_candidates: Vec<BrowserCandidate>,
    pub backdrop: String,
    pub data_root: String,
    pub log_path: String,
}

/// `{role,sandbox}-exited` 事件载荷：前端据此刷新运行态 + 非阻塞提示。
/// 仅当「外部/意外关闭」时发射（工具主动关闭由 close 路径自行刷新前端）。
#[derive(Clone, Serialize)]
struct ExitedPayload {
    id: String,
}

/// —— 查询 ——

#[tauri::command]
async fn get_state(state: State<'_, AppState>) -> Result<AppStateView, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let session = state.session.lock().await;
    // 读路径不做 prune（prune 含 1.2s CDP 探测会阻塞其他命令），
    // 死角色由命令路径（close_role_cmd 等）的 prune 清理，
    // 用户下一次操作时 UI 会刷新到最新状态。
    let roles = cfg
        .roles
        .iter()
        .map(|r| RoleView { role: r.clone(), running: session.is_role_running(&r.id) })
        .collect();
    let sandboxes = session
        .sandboxes
        .values()
        .map(|s| sandbox::SandboxInfo { id: s.id.clone(), dir: s.dir.clone() })
        .collect();
    drop(session);
    let snapshots = state.snapshots().list().unwrap_or_default();
    let browser_candidates =
        chameleon_core::browser::list_browser_candidates(cfg.browser_path.as_deref());
    Ok(AppStateView {
        roles,
        systems: cfg.systems,
        sandboxes,
        snapshots,
        browser_path: cfg.browser_path.as_ref().map(|p| p.display().to_string()),
        browser_candidates,
        backdrop: vibrancy::detect_backdrop_capability().as_str().to_string(),
        data_root: cfg.data_root.display().to_string(),
        log_path: state.log_path.display().to_string(),
    })
}

/// —— 角色管理 ——

#[tauri::command]
async fn create_role(state: State<'_, AppState>, name: String, color: String, system_id: Option<String>) -> Result<Role, String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    store.create_role(&mut cfg, name, color, system_id).map_err(msg)
}

#[tauri::command]
async fn update_role(state: State<'_, AppState>, role: Role) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    store.update_role(&mut cfg, role).map_err(msg)
}

#[tauri::command]
async fn delete_role(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    {
        let mut session = state.session.lock().await;
        launcher::prune_dead_roles(&mut session).await;
        if session.is_role_running(&id) {
            chameleon_core::warn_err(launcher::close_role(&mut session, &store, &mut cfg, &id).await, "delete_role 关闭运行角色失败");
        }
    }
    store.delete_role(&mut cfg, &id).map_err(msg)
}

/// —— 系统管理 ——

#[tauri::command]
async fn create_system(state: State<'_, AppState>, name: String) -> Result<System, String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    store.create_system(&mut cfg, name).map_err(msg)
}

#[tauri::command]
async fn update_system(state: State<'_, AppState>, system: System) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    store.update_system(&mut cfg, system).map_err(msg)
}

#[tauri::command]
async fn delete_system(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    store.delete_system(&mut cfg, &id).map_err(msg)
}

#[tauri::command]
async fn delete_system_with_roles(state: State<'_, AppState>, id: String) -> Result<usize, String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    // 收集待删角色 ID（在 delete_system 解除关联前）
    let role_ids: Vec<String> = cfg.roles.iter().filter(|r| r.system_id.as_deref() == Some(&id)).map(|r| r.id.clone()).collect();
    // 先删除系统
    store.delete_system(&mut cfg, &id).map_err(msg)?;
    // 逐个删除角色
    for rid in &role_ids {
        store.delete_role(&mut cfg, rid).map_err(msg)?;
    }
    Ok(role_ids.len())
}

/// —— 启动 / 关闭 ——

#[tauri::command]
async fn launch_role_cmd(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let role = cfg.roles.iter().find(|r| r.id == id).cloned().ok_or_else(|| msg(ChameleonError::RoleNotFound { id: id.clone() }))?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    launcher::launch_role(&mut session, &cfg, &role, true).await.map_err(|e| {
        tracing::error!(error = %e, role_id = %id, cdp_port = role.cdp_port, "角色启动失败");
        msg(e)
    })
}

#[tauri::command]
async fn close_role_cmd(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    // prune 可能已移除外部落关闭的死角色，此时不必再报错。
    if !session.is_role_running(&id) {
        return Ok(());
    }
    launcher::close_role(&mut session, &store, &mut cfg, &id).await.map_err(msg)
}

#[tauri::command]
async fn launch_all(state: State<'_, AppState>) -> Result<BatchResult, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    Ok(chameleon_core::batch::start_all(state.session.clone(), &cfg).await)
}

#[tauri::command]
async fn launch_system(state: State<'_, AppState>, system_id: String) -> Result<BatchResult, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    Ok(chameleon_core::batch::start_system(state.session.clone(), &cfg, &system_id).await)
}

#[tauri::command]
async fn close_system(state: State<'_, AppState>, system_id: String) -> Result<BatchResult, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    Ok(chameleon_core::batch::close_system(
        state.session.clone(), store, Arc::new(tokio::sync::Mutex::new(cfg)), &system_id,
    ).await)
}

#[tauri::command]
async fn close_all(state: State<'_, AppState>) -> Result<BatchResult, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    Ok(chameleon_core::batch::close_all(
        state.session.clone(), store, Arc::new(tokio::sync::Mutex::new(cfg)),
    ).await)
}

/// —— 接力 ——

#[tauri::command]
async fn handoff_cmd(
    state: State<'_, AppState>,
    source_id: String,
    target_id: String,
    mode: HandoffMode,
) -> Result<String, String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    chameleon_core::handoff::handoff(&mut session, &store, &mut cfg, &source_id, &target_id, mode)
        .await
        .map_err(msg)
}

/// —— 常用 URL 预设（角色级 + 系统级） ——

#[tauri::command]
async fn add_quick_link(state: State<'_, AppState>, role_id: String, name: Option<String>, url: String, auto_open: bool, login: Option<QuickLinkLogin>) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::add(&store, &mut cfg, &role_id, name, &url, auto_open, login).map_err(msg)
}

#[tauri::command]
async fn edit_quick_link(state: State<'_, AppState>, role_id: String, link_id: String, name: Option<String>, url: String, auto_open: bool, login: Option<QuickLinkLogin>) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::edit(&store, &mut cfg, &role_id, &link_id, name, &url, auto_open, login).map_err(msg)
}

#[tauri::command]
async fn remove_quick_link(state: State<'_, AppState>, role_id: String, link_id: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::remove(&store, &mut cfg, &role_id, &link_id).map_err(msg)
}

#[tauri::command]
async fn open_quick_link(state: State<'_, AppState>, role_id: String, link_id: String) -> Result<String, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    quicklinks::open(&mut session, &cfg, &role_id, &link_id).await.map_err(msg)
}

#[tauri::command]
async fn add_system_quick_link(state: State<'_, AppState>, system_id: String, name: Option<String>, url: String, auto_open: bool) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::add_system(&store, &mut cfg, &system_id, name, &url, auto_open).map_err(msg)
}

#[tauri::command]
async fn edit_system_quick_link(state: State<'_, AppState>, system_id: String, link_id: String, name: Option<String>, url: String, auto_open: bool) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::edit_system(&store, &mut cfg, &system_id, &link_id, name, &url, auto_open).map_err(msg)
}

#[tauri::command]
async fn remove_system_quick_link(state: State<'_, AppState>, system_id: String, link_id: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::remove_system(&store, &mut cfg, &system_id, &link_id).map_err(msg)
}

/// —— 浏览器路径 ——

#[tauri::command]
async fn pick_browser_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("选择 Chrome 或 Edge 可执行文件")
        .blocking_pick_file();
    Ok(picked.and_then(|f| f.into_path().ok().map(|p| p.display().to_string())))
}

#[tauri::command]
async fn set_browser_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    cfg.browser_path = Some(PathBuf::from(path));
    store.save(&cfg).map_err(msg)
}

/// —— UI 偏好 ——

#[tauri::command]
async fn get_ui_preferences(state: State<'_, AppState>) -> Result<UiPreferences, String> {
    let cfg = state.store().load().map_err(msg)?;
    Ok(cfg.ui_preferences)
}

#[tauri::command]
async fn set_ui_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    prefs: UiPreferences,
) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    let old_theme = cfg.ui_preferences.theme;
    cfg.ui_preferences = prefs.clone();
    store.save(&cfg).map_err(msg)?;

    // 仅当 theme 真正变化时重应用 vibrancy，避免冗余 Win32 调用。
    // 非 Windows 平台 apply 为 no-op，无需 cfg gate。
    if old_theme != prefs.theme {
        if let Some(window) = app.get_webview_window("main") {
            if let Err(e) = vibrancy::apply_vibrancy_for_theme(&window, prefs.theme) {
                tracing::warn!(error = %e, "切换 vibrancy 失败");
            }
        }
    }
    Ok(())
}

/// —— 配置导出 / 导入 ——

#[tauri::command]
async fn export_config_cmd(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let cfg = state.store().load().map_err(msg)?;
    let picked = app
        .dialog()
        .file()
        .set_title("导出角色配置")
        .set_file_name("chameleon-config.json")
        .add_filter("JSON", &["json"])
        .blocking_save_file();
    let Some(fp) = picked else { return Ok(None) };
    let dest = fp.into_path().map_err(|_| "所选保存路径无效。".to_string())?;
    export::export_config(&cfg, &dest).map_err(msg)?;
    Ok(Some(dest.display().to_string()))
}

#[tauri::command]
async fn import_config_cmd(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("导入角色配置")
        .add_filter("JSON", &["json"])
        .blocking_pick_file();
    let Some(fp) = picked else { return Ok(0) };
    let src = fp.into_path().map_err(|_| "所选文件路径无效。".to_string())?;
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    export::import_config(&store, &mut cfg, &src).map_err(msg)
}

/// —— 会话快照（v2） ——

#[tauri::command]
async fn save_snapshot(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    // 先清理死角色：否则 list_tab_urls 会拿到空 Vec，快照静默丢失该角色的所有页签 URL。
    launcher::prune_dead_roles(&mut session).await;
    state.snapshots().save(&mut session, &store, &mut cfg, &name).await.map_err(msg)
}

#[tauri::command]
async fn list_snapshots(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.snapshots().list().map_err(msg)
}

#[tauri::command]
async fn restore_snapshot(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    // 先清理死角色：否则 launch_role 的幂等守卫对死条目返回 Ok，
    // 后续 open_tab/close_other_tabs 在死 Browser 上挂起或报错。
    launcher::prune_dead_roles(&mut session).await;
    state.snapshots().restore(&mut session, &store, &mut cfg, &name).await.map_err(msg)
}

#[tauri::command]
async fn delete_snapshot(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.snapshots().delete(&name).map_err(msg)
}

/// —— 临时沙箱（v2） ——

#[tauri::command]
async fn launch_sandbox(state: State<'_, AppState>) -> Result<sandbox::SandboxInfo, String> {
    let cfg = state.store().load().map_err(msg)?;
    let mut session = state.session.lock().await;
    sandbox::launch(&mut session, &cfg).await.map_err(msg)
}

#[tauri::command]
async fn close_sandbox(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut session = state.session.lock().await;
    sandbox::close(&mut session, &id).await.map_err(msg)
}

#[tauri::command]
async fn cleanup_temp(state: State<'_, AppState>) -> Result<usize, String> {
    let cfg = state.store().load().map_err(msg)?;
    let mut session = state.session.lock().await;
    chameleon_core::cleanup::cleanup_all(&mut session, &cfg).await.map_err(msg)
}

/// —— 退出（优雅关闭全部窗口） ——

#[tauri::command]
async fn quit_app(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    shutdown(state.session.clone(), state.app_dir.clone()).await;
    app.exit(0);
    Ok(())
}

/// —— 日志 ——

#[tauri::command]
async fn open_log_folder(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.log_path.parent().unwrap_or(&state.log_path);
    open_dir(dir).map_err(|e| {
        tracing::warn!(error = %e, "打开日志文件夹失败");
        format!("打开日志文件夹失败：{e}")
    })
}

/// 优雅关闭全部角色窗口与沙箱。取 Arc+app_dir 的克隆，便于从托盘菜单
/// 的 spawn 任务调用，不在主线程 block_on（避免托盘「退出」卡死）。
async fn shutdown(session: Arc<tokio::sync::Mutex<Session>>, app_dir: PathBuf) {
    let store = ConfigStore::new(app_dir.join("config.json"));
    let mut cfg = store.load().unwrap_or_default();
    let mut session = session.lock().await;
    // 先清理死角色，避免 close_all_roles 对半开连接操作挂起（用户直接关 Chrome 场景）。
    launcher::prune_dead_roles(&mut session).await;
    launcher::close_all_roles(&mut session, &store, &mut cfg).await;
    let ids: Vec<String> = session.sandboxes.keys().cloned().collect();
    for id in ids {
        chameleon_core::warn_err(sandbox::close(&mut session, &id).await, "shutdown 关闭沙箱失败");
    }
    tracing::info!("chameleon 退出");
}

#[cfg(windows)]
pub fn webview2_installed() -> bool {
    const KEYS: [&str; 3] = [
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        r"HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    ];
    KEYS.iter().any(|k| reg_key_present(k))
}

#[cfg(windows)]
fn reg_key_present(key: &str) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("reg")
        .args(["query", key])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// —— 窗口控制（无边框模式） ——

#[tauri::command]
fn app_minimize(window: tauri::Window) { let _ = window.minimize(); }

#[tauri::command]
fn app_maximize(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) { let _ = window.unmaximize(); } else { let _ = window.maximize(); }
}

#[tauri::command]
fn app_hide(window: tauri::Window) { let _ = window.hide(); }

pub fn show_error_box(title: &str, msg: &str) {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let msg_w: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let _ = MessageBoxW(
                None,
                PCWSTR(msg_w.as_ptr()),
                PCWSTR(title_w.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("{title}: {msg}");
    }
}

pub fn run() {
    let app_dir = app_dir();
    let log_dir = data_base().join("logs");
    let (log_path, _log_guard) = init_logging(log_dir);
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));
    tracing::info!(log_path = %log_path.display(), "chameleon 启动");
    {
        let cfg = ConfigStore::new(app_dir.join("config.json"))
            .load()
            .unwrap_or_default();
        chameleon_core::warn_err(sandbox::cleanup_orphans(&[], &cfg), "启动清理孤儿沙箱失败");
    }

    // 被动感知浏览器退出：channel 把 launch 的 watcher 信号转发到前端。
    // 接收任务在 setup 里 spawn，拥有 Arc<Session> + AppHandle。
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<SessionEvent>();
    let session = Arc::new(tokio::sync::Mutex::new(Session {
        event_tx: Some(Arc::new(event_tx)),
        ..Session::default()
    }));
    let session_for_forwarder = Arc::clone(&session);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // 二次启动 → 激活已有实例主窗口（替换单实例文件锁）
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .manage(AppState {
            session,
            app_dir: app_dir.clone(),
            log_path: log_path.clone(),
        })
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("chameleon — Chrome 会话隔离管理工具")
            .inner_size(1180.0, 820.0)
            .min_inner_size(960.0, 600.0)
            .decorations(false)
            .transparent(true)
            .build()?;

            // Hybrid 主题：启动时按当前 prefs.theme 应用 vibrancy。
            // 非 Windows 平台 apply 为 no-op。
            if let Some(window) = app.get_webview_window("main") {
                let initial_theme = ConfigStore::new(app_dir.join("config.json"))
                    .load()
                    .map(|c| c.ui_preferences.theme)
                    .unwrap_or_default();
                if let Err(e) = vibrancy::apply_vibrancy_for_theme(&window, initial_theme) {
                    tracing::warn!(error = %e, "启动 vibrancy 失败");
                }
            }

            // 系统托盘：常驻图标，关窗最小化到托盘，左键/菜单恢复
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = MenuBuilder::new(app).items(&[&show_item, &quit_item]).build()?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("chameleon")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main_window(app),
                    "quit" => {
                        let s = app.state::<AppState>();
                        let session = s.session.clone();
                        let app_dir = s.app_dir.clone();
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            shutdown(session, app_dir).await;
                            app.exit(0);
                        });
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;
            // 单接收任务：把 watcher 发出的 SessionEvent 路由到前端。
            // remove 在工具主动关闭路径已先行完成 → 此处返回 None → 静默；
            // 仅「外部/意外关闭」时 emit，前端据此刷新角色按钮 + 非阻塞提示。
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = event_rx;
                while let Some(ev) = rx.recv().await {
                match ev {
                    SessionEvent::RoleExited { id } => {
                        let unexpected = session_for_forwarder
                            .lock()
                            .await
                            .roles
                            .remove(&id)
                            .is_some();
                        if unexpected {
                            let _ = app_handle.emit("role-exited", ExitedPayload { id });
                        }
                    }
                    SessionEvent::SandboxExited { id } => {
                        let unexpected = session_for_forwarder
                            .lock()
                            .await
                            .sandboxes
                            .remove(&id)
                            .is_some();
                        if unexpected {
                            let _ = app_handle.emit("sandbox-exited", ExitedPayload { id });
                        }
                    }
                }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭主窗口 → 隐藏到托盘（不退出），避免找不到已运行实例
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            create_role, update_role, delete_role,
            create_system, update_system, delete_system, delete_system_with_roles,
            launch_role_cmd, close_role_cmd, launch_all, launch_system, close_system, close_all,
            handoff_cmd,
            add_quick_link, edit_quick_link, remove_quick_link, open_quick_link,
            add_system_quick_link, edit_system_quick_link, remove_system_quick_link,
            pick_browser_path, set_browser_path,
            get_ui_preferences, set_ui_preferences,
            export_config_cmd, import_config_cmd,
            save_snapshot, list_snapshots, restore_snapshot, delete_snapshot,
            launch_sandbox, close_sandbox, cleanup_temp,
            app_minimize, app_maximize, app_hide,
            quit_app,
            open_log_folder
        ])
        .build(tauri::generate_context!())
        .expect("构建 chameleon 应用失败")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                // 清理在显式退出路径（托盘 quit / quit_app 命令）的 spawn 任务里
                // 已完成；此处不再 block_on，避免主线程阻塞导致托盘「退出」卡死。
            }
        });
}

/// 显示主窗口到前台（从托盘恢复 / 二次启动激活）。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 冒烟验证：init_logging 创建 chameleon.log 并写入 tracing 事件。
    /// tracing_subscriber::init() 全局唯一，src-tauri 仅此一个测试，不冲突。
    #[test]
    fn init_logging_creates_file_and_writes_events() {
        let dir = std::env::temp_dir().join("chameleon-log-smoke");
        let _ = std::fs::remove_dir_all(&dir);

        let (log_path, guard) = init_logging(dir.clone());
        tracing::info!("smoke test event");
        drop(guard); // flush non_blocking 缓冲

        let content = std::fs::read_to_string(&log_path)
            .expect("chameleon.log 应存在且可读");
        assert!(
            content.contains("smoke test event"),
            "日志文件应包含写入的事件，实得: {content}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
