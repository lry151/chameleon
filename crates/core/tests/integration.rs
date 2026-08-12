//! 集成测试：对真实 Chrome/Chromium 运行——启动→CDP 连接→开标签→读激活标签→
//! 优雅关窗→沙箱退出删目录→快照恢复。无浏览器时自动跳过。
//!
//! 运行方式：默认 `cargo test`（有可检测浏览器即跑，无则跳过）。
//! 测试环境变量：`CHAMELEON_NO_SANDBOX=1`（snap chromium / root / CI 需要）。

use chameleon_core::{
    batch, config::ConfigStore,
    handoff::HandoffMode,
    launcher,
    model::{GlobalConfig, QuickLink, Role, WindowRect},
    ports, sandbox, snapshot::SnapshotStore, safety, Session,
};
use std::path::PathBuf;
use std::sync::{Arc, Once};
use tempfile::tempdir;

static ENV_INIT: Once = Once::new();

/// 测试环境初始化：headless + 可选 no-sandbox。
fn ensure_env() {
    ENV_INIT.call_once(|| {
        std::env::set_var("CHAMELEON_HEADLESS", "1");
        // snap chromium 在受限环境需要 --no-sandbox
        if std::env::var_os("CHAMELEON_NO_SANDBOX").is_none()
            && std::env::var_os("CHAMELEON_NO_SANDBOX_AUTO").is_some()
        {
            std::env::set_var("CHAMELEON_NO_SANDBOX", "1");
        }
    });
}

/// 无可检测浏览器时跳过整个测试。
fn browser_available() -> bool {
    chameleon_core::browser::detect_browser(None).is_ok()
}

/// 构造一个临时配置 + 临时数据根目录 + 一个角色。
async fn fixture_role(name: &str) -> (tempfile::TempDir, ConfigStore, GlobalConfig, Role) {
    let dir = tempdir().unwrap();
    let store = ConfigStore::new(dir.path().join("config.json"));
    let mut cfg = GlobalConfig::default();
    cfg.data_root = dir.path().join("data");
    let role = store.create_role(&mut cfg, name.into(), "#e74c3c".into()).unwrap();
    (dir, store, cfg, role)
}

#[tokio::test]
async fn launch_open_tab_read_url_and_close() {
    ensure_env();
    if !browser_available() {
        eprintln!("skip: no browser detected");
        return;
    }
    let (_dir, store, mut cfg, role) = fixture_role("管理员").await;
    let mut session = Session::default();

    launcher::launch_role(&mut session, &cfg, &role, true).await.expect("launch");
    assert!(session.is_role_running(&role.id));

    let url = "https://example.com/";
    launcher::open_tab(&mut session, &cfg, &role.id, url).await.expect("open tab");

    // 给页面一点导航时间
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let got = launcher::read_active_tab(&session, &role.id).await.expect("read url");
    assert!(got.contains("example.com"), "got url: {got}");

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.expect("close");
    assert!(!session.is_role_running(&role.id));
}

#[tokio::test]
async fn handoff_parallel_keeps_source() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, store, mut cfg, src) = fixture_role("源角色").await;
    // 第二个角色用独立数据目录避免冲突
    let port2 = ports::pick_free_port().unwrap();
    let tgt = Role::new(
        "目标角色".into(),
        "#3498db".into(),
        cfg.data_root.join("tgt"),
        port2,
    );
    cfg.roles.push(tgt.clone());

    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &src, true).await.unwrap();
    launcher::open_tab(&mut session, &cfg, &src.id, "https://example.com/").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let url = chameleon_core::handoff::handoff(
        &mut session, &store, &mut cfg, &src.id, &tgt.id, HandoffMode::Parallel,
    )
    .await
    .expect("handoff parallel");
    assert!(url.contains("example.com"));

    // 并行模式：源窗口仍在运行
    assert!(session.is_role_running(&src.id), "parallel keeps source");
    assert!(session.is_role_running(&tgt.id), "target started");

    // 清理
    launcher::close_role(&mut session, &store, &mut cfg, &src.id).await.ok();
    launcher::close_role(&mut session, &store, &mut cfg, &tgt.id).await.ok();
}

#[tokio::test]
async fn handoff_relay_closes_source() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, store, mut cfg, src) = fixture_role("源角色").await;
    let port2 = ports::pick_free_port().unwrap();
    let tgt = Role::new(
        "目标角色".into(),
        "#3498db".into(),
        cfg.data_root.join("tgt2"),
        port2,
    );
    cfg.roles.push(tgt.clone());

    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &src, true).await.unwrap();
    launcher::open_tab(&mut session, &cfg, &src.id, "https://example.com/").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    chameleon_core::handoff::handoff(
        &mut session, &store, &mut cfg, &src.id, &tgt.id, HandoffMode::Relay,
    )
    .await
    .expect("handoff relay");

    // 接力模式：源窗口已关闭
    assert!(!session.is_role_running(&src.id), "relay closes source");
    assert!(session.is_role_running(&tgt.id), "target remains");

    launcher::close_role(&mut session, &store, &mut cfg, &tgt.id).await.ok();
}

#[tokio::test]
async fn sandbox_lifecycle_deletes_dir() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let dir = tempdir().unwrap();
    let mut cfg = GlobalConfig::default();
    cfg.data_root = dir.path().join("data");
    let mut session = Session::default();

    let info = sandbox::launch(&mut session, &cfg).await.expect("sandbox launch");
    assert!(info.dir.exists(), "sandbox dir created");
    assert!(sandbox::is_sandbox_dir(&info.dir), "marker present");

    sandbox::close(&mut session, &info.id).await.expect("sandbox close");
    assert!(!info.dir.exists(), "sandbox dir deleted on close");
    assert!(!session.sandboxes.contains_key(&info.id));
}

#[tokio::test]
async fn sandbox_dir_deleted_when_browser_exits_unassisted() {
    // 直接关闭浏览器（模拟用户点 X / 进程退出）：handler 流结束 →
    // 监听任务应自动删除临时目录（spec：工具监听沙箱进程退出即删除）。
    ensure_env();
    if !browser_available() {
        return;
    }
    let dir = tempdir().unwrap();
    let mut cfg = GlobalConfig::default();
    cfg.data_root = dir.path().join("data");
    let mut session = Session::default();
    let info = sandbox::launch(&mut session, &cfg).await.expect("launch");
    assert!(info.dir.exists());
    // 直接通过 CDP 关闭浏览器（不走 sandbox::close），触发 handler 流结束
    {
        let mut s = session.sandboxes.remove(&info.id).unwrap();
        let _ = s.browser.close().await;
    }
    // 给监听任务一点时间清理
    for _ in 0..20 {
        if !info.dir.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    assert!(!info.dir.exists(), "monitor should delete dir after browser exit");
}

#[tokio::test]
async fn sandbox_orphan_cleanup() {
    let dir = tempdir().unwrap();
    let mut cfg = GlobalConfig::default();
    cfg.data_root = dir.path().join("data");
    // 手造一个孤儿沙箱目录（带标记）
    let orphan = cfg.data_root.join("sandbox").join("dead-uuid");
    std::fs::create_dir_all(&orphan).unwrap();
    std::fs::write(orphan.join(sandbox::SANDBOX_MARKER), "x").unwrap();
    // 手造一个无标记的目录，不应被删
    let keep = cfg.data_root.join("sandbox").join("keep-me");
    std::fs::create_dir_all(&keep).unwrap();

    let removed = sandbox::cleanup_orphans(&[], &cfg).expect("cleanup");
    assert_eq!(removed, 1, "only orphan with marker removed");
    assert!(!orphan.exists());
    assert!(keep.exists(), "unmarked dir preserved");
}

#[tokio::test]
async fn snapshot_save_and_restore() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (dir, store, mut cfg, role) = fixture_role("快照角色").await;
    let snap_dir = dir.path().join("snapshots");
    let snaps = SnapshotStore::new(&snap_dir);
    let mut session = Session::default();

    launcher::launch_role(&mut session, &cfg, &role, true).await.unwrap();
    launcher::open_tab(&mut session, &cfg, &role.id, "https://example.com/").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    snaps.save(&mut session, &store, &mut cfg, "回归1").await.expect("save snapshot");
    let list = snaps.list().unwrap();
    assert!(list.contains(&"回归1".to_string()));

    // 关闭角色后恢复
    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
    assert!(!session.is_role_running(&role.id));

    snaps.restore(&mut session, &store, &mut cfg, "回归1").await.expect("restore");
    assert!(session.is_role_running(&role.id), "role restarted by restore");
    let urls = launcher::list_tab_urls(&session, &role.id).await;
    assert!(urls.iter().any(|u| u.contains("example.com")), "tab reopened: {urls:?}");

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
    snaps.delete("回归1").unwrap();
}

#[tokio::test]
async fn default_dir_role_refused_on_launch() {
    ensure_env();
    // 即使有浏览器，指向默认配置目录的角色必须被拒绝启动
    let dir = tempdir().unwrap();
    let mut cfg = GlobalConfig::default();
    cfg.data_root = dir.path().join("data");
    let bad = Role::new(
        "危险角色".into(),
        "#000".into(),
        PathBuf::from(r"C:\Users\t\AppData\Local\Google\Chrome\User Data"),
        ports::pick_free_port().unwrap(),
    );
    cfg.roles.push(bad.clone());
    let mut session = Session::default();
    let err = launcher::launch_role(&mut session, &cfg, &bad, true).await;
    assert!(err.is_err(), "default-dir role must be refused");
    assert!(matches!(
        err.unwrap_err(),
        chameleon_core::ChameleonError::DefaultDirRefused { .. }
    ));
}

#[test]
fn window_rect_serializes_in_launch_args() {
    // 验证 build_config 路径（经 launch_role 内部）：有 window_rect 时不报错。
    // 这里只验证 safety 校验逻辑与 WindowRect 模型可往返。
    let mut cfg = GlobalConfig::default();
    cfg.data_root = PathBuf::from("/tmp/data");
    let mut role = Role::new(
        "位置角色".into(),
        "#fff".into(),
        cfg.data_root.join("pos"),
        9555,
    );
    role.window_rect = Some(WindowRect { x: 100, y: 50, width: 800, height: 600 });
    cfg.roles.push(role.clone());
    assert!(safety::validate_role(&role, &cfg).is_ok());
}

// —— v0.3 行为一致性（工单 #2/#3/#4）——

/// #3：一键关闭同时关闭沙箱窗口并删除临时目录。
#[tokio::test]
async fn close_all_closes_sandbox() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let dir = tempdir().unwrap();
    let cfg = GlobalConfig { data_root: dir.path().join("data"), ..GlobalConfig::default() };
    let mut sess = Session::default();
    let store = ConfigStore::new(dir.path().join("config.json"));

    let info = sandbox::launch(&mut sess, &cfg).await.expect("sandbox launch");
    assert!(sess.sandboxes.contains_key(&info.id));
    assert!(info.dir.exists(), "sandbox dir created");

    let session = Arc::new(tokio::sync::Mutex::new(sess));
    let res = batch::close_all(session.clone(), store.clone(), Arc::new(tokio::sync::Mutex::new(cfg))).await;
    assert_eq!(res.failed, 0, "close_all errors: {:?}", res.errors);
    assert!(!info.dir.exists(), "sandbox temp dir deleted by close_all");
    assert!(!session.lock().await.sandboxes.contains_key(&info.id), "sandbox removed from session");
}

/// #4：快照保存排除角色首页锚点页签（只记真实测试页）。
#[tokio::test]
async fn snapshot_excludes_role_home_anchor() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (dir, store, mut cfg, role) = fixture_role("锚点排除").await;
    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &role, true).await.unwrap();
    launcher::open_tab(&mut session, &cfg, &role.id, "https://example.com/").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // 锚点页签被过滤：list_tab_urls 不返回标记 URL
    let urls = launcher::list_tab_urls(&session, &role.id).await;
    assert!(urls.iter().any(|u| u.contains("example.com")), "real tab present: {urls:?}");
    assert!(
        urls.iter().all(|u| !u.contains("data-chameleon-role-home")),
        "anchor must be excluded: {urls:?}"
    );

    // 快照落盘后的 tabs 同样不含锚点
    let snaps = SnapshotStore::new(&dir.path().join("snapshots"));
    snaps.save(&mut session, &store, &mut cfg, "无锚点").await.unwrap();
    let raw = std::fs::read_to_string(dir.path().join("snapshots/无锚点.json")).unwrap();
    let snap: chameleon_core::model::Snapshot = serde_json::from_str(&raw).unwrap();
    let role_tabs = snap.roles.iter().find(|r| r.role_id == role.id).unwrap();
    assert!(role_tabs.tabs.iter().any(|u| u.contains("example.com")));
    assert!(
        role_tabs.tabs.iter().all(|u| !u.contains("data-chameleon-role-home")),
        "saved snapshot must exclude anchor: {:?}",
        role_tabs.tabs
    );

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
}

/// #4：恢复未运行角色时抑制 auto_open 默认页，只开锚点 + 快照页。
#[tokio::test]
async fn restore_suppresses_auto_open() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (dir, store, mut cfg, mut role) = fixture_role("抑制auto").await;
    // 给角色配一个 auto_open 预设：恢复后不应重新打开
    role.quick_links.push(QuickLink::new(
        Some("默认页".into()),
        "https://auto-open.example/".into(),
        true,
        None,
    ));
    let snaps = SnapshotStore::new(&dir.path().join("snapshots"));
    let mut session = Session::default();

    // 以 auto_open=false 启动（只开锚点），再开一个快照页并保存
    launcher::launch_role(&mut session, &cfg, &role, false).await.unwrap();
    launcher::open_tab(&mut session, &cfg, &role.id, "https://example.com/").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    snaps.save(&mut session, &store, &mut cfg, "恢复抑制").await.unwrap();

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
    assert!(!session.is_role_running(&role.id));

    // 恢复：窗口应只有锚点 + example.com，无 auto-open 默认页
    snaps.restore(&mut session, &store, &mut cfg, "恢复抑制").await.expect("restore");
    let urls = launcher::list_tab_urls(&session, &role.id).await;
    assert!(urls.iter().any(|u| u.contains("example.com")), "snapshot page reopened: {urls:?}");
    assert!(
        urls.iter().all(|u| !u.contains("auto-open.example")),
        "auto_open preset must NOT be reopened: {urls:?}"
    );

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
}

/// #4：恢复后角色首页锚点仍在（窗口标题含角色名）。
#[tokio::test]
#[ignore = "find_role_home_tab was removed in ddd9e96; test needs rewrite to use current API"]
async fn restore_preserves_anchor() {
    // TODO: find_role_home_tab was removed in ddd9e96 along with ROLE_HOME_MARKER.
    // Test needs rewrite to verify anchor-tab preservation using current snapshot API.
    ensure_env();
    if !browser_available() {
        return;
    }
    let (dir, store, mut cfg, role) = fixture_role("锚点保留").await;
    let snaps = SnapshotStore::new(&dir.path().join("snapshots"));
    let mut session = Session::default();

    launcher::launch_role(&mut session, &cfg, &role, false).await.unwrap();
    launcher::open_tab(&mut session, &cfg, &role.id, "https://example.com/").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    snaps.save(&mut session, &store, &mut cfg, "留锚点").await.unwrap();

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
    snaps.restore(&mut session, &store, &mut cfg, "留锚点").await.expect("restore");

    // 锚点页签仍在，且标题含角色名
    // TODO: rewrite using list_tab_urls or similar current API
    // let anchor = launcher::find_role_home_tab(&session, &role.id).await
    //     .expect("anchor tab preserved after restore");
    // let run = session.roles.get(&role.id).unwrap();
    // let page = run.browser.get_page(anchor).await.expect("get anchor page");
    // let title = page.get_title().await.ok().flatten().unwrap_or_default();
    // assert!(title.contains("锚点保留"), "window title keeps role name: {title:?}");

    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
}

/// #2：端口被非浏览器/僵尸实例占用且无法接管 → 硬错误、不 spawn 窗口（闪窗回归守卫）。
#[tokio::test]
async fn port_occupied_non_browser_hard_errors() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, _store, cfg, role) = fixture_role("端口占用").await;
    // 用裸 TcpListener 占住该角色的 cdp_port（非浏览器实例，CDP 无法接管）
    let listener = std::net::TcpListener::bind(("127.0.0.1", role.cdp_port)).unwrap();
    std::thread::spawn(move || {
        // 接受并立即断开，让 Browser::connect 的 /json/version 请求快速失败
        for stream in listener.incoming() {
            drop(stream);
        }
    });
    let mut session = Session::default();

    let err = launcher::launch_role(&mut session, &cfg, &role, true).await;
    let err = err.expect_err("port occupied by non-browser must hard-error");
    assert!(matches!(err, chameleon_core::ChameleonError::PortTakenNotRole { .. }));
    assert!(
        err.message().contains("一键关闭所有"),
        "error must guide user to 一键关闭: {}",
        err.message()
    );
    // 不 spawn 任何新窗口
    assert!(!session.is_role_running(&role.id), "no window may be spawned");
}

/// 工单：用户直接关闭浏览器后程序应感知（prune_dead_roles 清理死角色），
/// 否则 UI 显示陈旧「运行中」、对死浏览器的操作挂死（按钮卡死）。
#[tokio::test]
async fn role_pruned_after_browser_closed_externally() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, _store, cfg, role) = fixture_role("外部关闭").await;
    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &role, false).await.expect("launch");
    assert!(session.is_role_running(&role.id));
    // 模拟用户直接关浏览器：不走 close_role，程序未感知，session 仍持死句柄
    {
        let run = session.roles.get_mut(&role.id).unwrap();
        let _ = run.browser.close().await;
    }
    // 给 handler 流结束一点时间
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(session.is_role_running(&role.id), "bug: stale entry remains");
    let removed = launcher::prune_dead_roles(&mut session).await;
    assert_eq!(removed, 1, "dead role should be pruned");
    assert!(!session.is_role_running(&role.id), "role removed after prune");
}

/// close_role 后端口应真正释放，紧接的启动不应误判为僵尸占用（CI 闪窗回归守卫）。
#[tokio::test]
async fn close_role_releases_port_for_immediate_relaunch() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (dir, store, mut cfg, role) = fixture_role("端口释放").await;
    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &role, false).await.expect("launch");
    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.expect("close");
    // 紧接在同一端口重启 —— 修复前会因端口未释放误判为僵尸占用 → PortTakenNotRole
    launcher::launch_role(&mut session, &cfg, &role, false).await
        .expect("relaunch on same port must succeed after close_role released it");
    assert!(session.is_role_running(&role.id));
    launcher::close_role(&mut session, &store, &mut cfg, &role.id).await.ok();
    let _ = dir;
}

/// 工单：手动 X 掉角色窗口后点「关闭组」一直转圈。
/// close_system 必须先 prune 死角色，否则对死 CDP 句柄执行无超时
/// `capture_bounds` 会永久挂起（chromiumoxide 对断开的 websocket 不回响应）。
#[tokio::test]
async fn close_system_returns_after_role_window_closed_externally() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, store, mut cfg, mut role) = fixture_role("关闭组死句柄").await;
    let sys = store.create_system(&mut cfg, "测试系统".into()).unwrap();
    role.system_id = Some(sys.id.clone());
    store.update_role(&mut cfg, role.clone()).unwrap();

    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &role, false).await.expect("launch");
    assert!(session.is_role_running(&role.id));
    // 模拟用户手动 X 掉窗口：杀子进程（进程消失、websocket 非优雅断开），
    // session 仍持死 CDP 句柄。kill() 而非 close() —— 后者让 handler 干净退出、
    // 后续 execute 快速失败，复现不出真挂死；只有进程消失才会让 execute 永久等响应。
    {
        let run = session.roles.get_mut(&role.id).unwrap();
        let _ = run.browser.kill().await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(session.is_role_running(&role.id), "stale dead entry remains pre-close");

    let session = Arc::new(tokio::sync::Mutex::new(session));
    let cfg = Arc::new(tokio::sync::Mutex::new(cfg));
    // 修复前：close_system 不 prune → close_one 对死句柄 capture_bounds 永久挂 → 超时（红）
    // 修复后：先 prune 探活（version 超时失败）移除死角色 → to_close 空 → 立即返回
    let res = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        batch::close_system(session.clone(), store.clone(), cfg, &sys.id),
    )
    .await
    .expect("close_system must not hang on a dead browser handle");
    assert_eq!(res.failed, 0, "close_system errors: {:?}", res.errors);
    assert!(
        !session.lock().await.is_role_running(&role.id),
        "dead role removed from session"
    );
}

/// 对称守卫：一键关闭同样先 prune 死角色，否则对死句柄 close_one 挂死。
#[tokio::test]
async fn close_all_returns_after_role_window_closed_externally() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, store, cfg, role) = fixture_role("一键关闭死句柄").await;
    let mut session = Session::default();
    launcher::launch_role(&mut session, &cfg, &role, false).await.expect("launch");
    {
        let run = session.roles.get_mut(&role.id).unwrap();
        let _ = run.browser.kill().await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    assert!(session.is_role_running(&role.id), "stale dead entry remains pre-close");

    let session = Arc::new(tokio::sync::Mutex::new(session));
    let cfg = Arc::new(tokio::sync::Mutex::new(cfg));
    let res = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        batch::close_all(session.clone(), store.clone(), cfg),
    )
    .await
    .expect("close_all must not hang on a dead browser handle");
    assert_eq!(res.failed, 0, "close_all errors: {:?}", res.errors);
    assert!(
        !session.lock().await.is_role_running(&role.id),
        "dead role removed from session"
    );
}

/// 被动感知回归守卫：launch 后 watcher 必须监听 handler 完成，
/// 浏览器被杀时 event 通道必须收到 RoleExited（毫秒级，不依赖 prune）。
#[tokio::test]
async fn watcher_emits_role_exited_on_external_close() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, _store, cfg, role) = fixture_role("watcher-退出事件").await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<chameleon_core::SessionEvent>();
    let mut session = Session {
        event_tx: Some(std::sync::Arc::new(tx)),
        ..Session::default()
    };
    launcher::launch_role(&mut session, &cfg, &role, false).await.expect("launch");
    // kill（SIGKILL 进程）= 最严苛场景：连接出错而非优雅关闭。
    // 修复前：消费循环不识别 Some(Err) 为终止 → handler 任务挂起 → watcher 不触发（5s 超时红）。
    // 修复后：spawn_handler 遇 Some(Err) 也 break → 任务结束 → watcher 发事件。
    {
        let run = session.roles.get_mut(&role.id).unwrap();
        let _ = run.browser.kill().await;
    }

    // 被动感知应在秒级内发出；不依赖 prune。
    let ev = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("watcher must emit RoleExited within 5s of external close")
        .expect("channel must not close before event");
    match ev {
        chameleon_core::SessionEvent::RoleExited { id } => assert_eq!(id, role.id),
        other => panic!("expected RoleExited, got {other:?}"),
    }
}

/// 沙箱被动感知 + kill 路径目录清理回归守卫。
/// 修复前：沙箱被 kill 时消费循环不终止 → 清理任务不跑 → 临时目录泄漏、无事件。
#[tokio::test]
async fn sandbox_watcher_emits_on_kill_and_cleans_dir() {
    ensure_env();
    if !browser_available() {
        return;
    }
    let (_dir, _store, cfg, _role) = fixture_role("沙箱watcher").await;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<chameleon_core::SessionEvent>();
    let mut session = Session {
        event_tx: Some(std::sync::Arc::new(tx)),
        ..Session::default()
    };
    let info = sandbox::launch(&mut session, &cfg).await.expect("sandbox launch");
    let sb_dir = info.dir.clone();
    let id = info.id.clone();
    assert!(sb_dir.exists(), "sandbox dir created");

    // kill 沙箱进程 → spawn_handler 遇 Some(Err) 终止 → 清理任务跑。
    {
        let sb = session.sandboxes.get_mut(&id).unwrap();
        let _ = sb.browser.kill().await;
    }

    let ev = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("sandbox watcher must emit SandboxExited within 5s of kill")
        .expect("channel must not close before event");
    match ev {
        chameleon_core::SessionEvent::SandboxExited { id: eid } => assert_eq!(eid, id),
        other => panic!("expected SandboxExited, got {other:?}"),
    }

    // 清理任务删临时目录（kill 路径修复前会泄漏）。
    for _ in 0..30 {
        if !sb_dir.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    assert!(!sb_dir.exists(), "sandbox temp dir must be deleted on kill");
}