//! 变色龙 Tauri 外壳：薄壳透传 chameleon-core 的全部领域逻辑。
//!
//! 命令层只做参数搬运与错误文案映射（统一走 `ChameleonError::message` 的中文文案）。

use chameleon_core::{
    batch::BatchResult,
    browser::BrowserCandidate,
    config::{app_dir, ConfigStore},
    export,
    handoff::HandoffMode,
    launcher,
    model::{LoginConfig, Role, System},
    quicklinks, sandbox,
    snapshot::SnapshotStore,
    ChameleonError, Session,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WindowEvent,
};
use tauri_plugin_dialog::DialogExt;

fn msg(e: ChameleonError) -> String {
    e.message()
}

pub struct AppState {
    pub session: Arc<tokio::sync::Mutex<Session>>,
    pub app_dir: PathBuf,
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
    pub data_root: String,
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
        data_root: cfg.data_root.display().to_string(),
    })
}

/// —— 角色管理 ——

#[tauri::command]
async fn create_role(state: State<'_, AppState>, name: String, color: String) -> Result<Role, String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    store.create_role(&mut cfg, name, color).map_err(msg)
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
            let _ = launcher::close_role(&mut session, &store, &mut cfg, &id).await;
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

/// —— 启动 / 关闭 ——

#[tauri::command]
async fn launch_role_cmd(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let role = cfg.roles.iter().find(|r| r.id == id).cloned().ok_or_else(|| msg(ChameleonError::RoleNotFound { id: id.clone() }))?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    launcher::launch_role(&mut session, &cfg, &role, true).await.map_err(msg)
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
    let mut session = state.session.lock().await;
    Ok(chameleon_core::batch::start_all(&mut session, &cfg).await)
}

#[tauri::command]
async fn launch_system(state: State<'_, AppState>, system_id: String) -> Result<BatchResult, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    Ok(chameleon_core::batch::start_system(&mut session, &cfg, &system_id).await)
}

#[tauri::command]
async fn close_all(state: State<'_, AppState>) -> Result<BatchResult, String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    Ok(chameleon_core::batch::close_all(&mut session, &store, &mut cfg).await)
}

/// —— 登录辅助 ——

#[tauri::command]
async fn login_assist_cmd(state: State<'_, AppState>, role_id: String) -> Result<(), String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    launcher::login_assist(&mut session, &cfg, &role_id).await.map_err(msg)
}

/// —— 角色登录配置 ——

#[tauri::command]
async fn set_role_login(state: State<'_, AppState>, role_id: String, login: Option<LoginConfig>) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    if let Some(slot) = cfg.roles.iter_mut().find(|r| r.id == role_id) {
        slot.login = login;
        store.save(&cfg).map_err(msg)
    } else {
        Err(msg(ChameleonError::RoleNotFound { id: role_id }))
    }
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
async fn add_quick_link(state: State<'_, AppState>, role_id: String, name: String, url: String, auto_open: bool) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::add(&store, &mut cfg, &role_id, &name, &url, auto_open).map_err(msg)
}

#[tauri::command]
async fn edit_quick_link(state: State<'_, AppState>, role_id: String, old_name: String, name: String, url: String, auto_open: bool) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::edit(&store, &mut cfg, &role_id, &old_name, &name, &url, auto_open).map_err(msg)
}

#[tauri::command]
async fn remove_quick_link(state: State<'_, AppState>, role_id: String, name: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::remove(&store, &mut cfg, &role_id, &name).map_err(msg)
}

#[tauri::command]
async fn open_quick_link(state: State<'_, AppState>, role_id: String, name: String) -> Result<String, String> {
    let store = state.store();
    let cfg = store.load().map_err(msg)?;
    let mut session = state.session.lock().await;
    launcher::prune_dead_roles(&mut session).await;
    quicklinks::open(&mut session, &cfg, &role_id, &name).await.map_err(msg)
}

#[tauri::command]
async fn add_system_quick_link(state: State<'_, AppState>, system_id: String, name: String, url: String, auto_open: bool) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::add_system(&store, &mut cfg, &system_id, &name, &url, auto_open).map_err(msg)
}

#[tauri::command]
async fn edit_system_quick_link(state: State<'_, AppState>, system_id: String, old_name: String, name: String, url: String, auto_open: bool) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::edit_system(&store, &mut cfg, &system_id, &old_name, &name, &url, auto_open).map_err(msg)
}

#[tauri::command]
async fn remove_system_quick_link(state: State<'_, AppState>, system_id: String, name: String) -> Result<(), String> {
    let store = state.store();
    let mut cfg = store.load().map_err(msg)?;
    quicklinks::remove_system(&store, &mut cfg, &system_id, &name).map_err(msg)
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
        let _ = sandbox::close(&mut session, &id).await;
    }
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
    std::process::Command::new("reg")
        .args(["query", key])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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
    {
        let cfg = ConfigStore::new(app_dir.join("config.json"))
            .load()
            .unwrap_or_default();
        let _ = sandbox::cleanup_orphans(&[], &cfg);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // 二次启动 → 激活已有实例主窗口（替换单实例文件锁）
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .manage(AppState {
            session: Arc::new(tokio::sync::Mutex::new(Session::default())),
            app_dir: app_dir.clone(),
        })
        .setup(|app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("chameleon — Chrome 会话隔离管理工具")
            .inner_size(1180.0, 820.0)
            .min_inner_size(960.0, 600.0)
            .build()?;

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
            create_system, update_system, delete_system,
            launch_role_cmd, close_role_cmd, launch_all, launch_system, close_all,
            login_assist_cmd, set_role_login,
            handoff_cmd,
            add_quick_link, edit_quick_link, remove_quick_link, open_quick_link,
            add_system_quick_link, edit_system_quick_link, remove_system_quick_link,
            pick_browser_path, set_browser_path,
            export_config_cmd, import_config_cmd,
            save_snapshot, list_snapshots, restore_snapshot, delete_snapshot,
            launch_sandbox, close_sandbox, cleanup_temp,
            quit_app
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
