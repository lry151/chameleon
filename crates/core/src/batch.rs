//! 一键启动 / 一键关闭 / 启动组：批量拉起全部角色 / 启动某系统下全部角色；
//! CDP 优雅关闭全部测试窗口。仅作用于测试数据目录，绝不触碰日常浏览器配置。
//!
//! 并行执行：每个角色的浏览器启动 / CDP 关闭相互独立（不同端口 + 不同数据目录），
//! 用 `tokio::spawn` + `join_all` 并发执行，仅在对 `Session` 的短暂检查 / 插入时持锁。

use crate::config::ConfigStore;
use crate::error::ChameleonError;
use crate::launcher;
use crate::model::GlobalConfig;
use crate::session::Session;
use serde::Serialize;
use std::sync::Arc;

/// 批量操作结果：成功数 + 失败的中文文案列表（统一走错误文案层）。
#[derive(Debug, Default, Serialize)]
pub struct BatchResult {
    pub ok: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

impl BatchResult {
    fn push_ok(&mut self) {
        self.ok += 1;
    }
    fn push_error(&mut self, e: ChameleonError) {
        self.failed += 1;
        self.errors.push(e.message());
    }
}

/// 合并多个子结果到一个总结果。
fn merge_into(dst: &mut BatchResult, src: BatchResult) {
    dst.ok += src.ok;
    dst.failed += src.failed;
    dst.errors.extend(src.errors);
}

/// 收集需要启动的角色列表（过滤掉已运行的）。
fn pending_roles<'a>(session: &Session, cfg: &'a GlobalConfig, system_id: Option<&str>) -> Vec<&'a crate::model::Role> {
    cfg.roles
        .iter()
        .filter(|r| {
            if let Some(sid) = system_id {
                r.system_id.as_deref() == Some(sid)
            } else {
                true
            }
        })
        .filter(|r| !session.is_role_running(&r.id))
        .collect()
}

/// 启动单个角色（在 spawn 任务内调用，持锁检查 + 解锁启动 + 持锁插入）。
async fn launch_one(
    session: Arc<tokio::sync::Mutex<Session>>,
    cfg: GlobalConfig,
    role: crate::model::Role,
) -> BatchResult {
    let mut out = BatchResult::default();
    // 持锁检查是否已在运行（避免并发启动同一角色的竞态）
    {
        let s = session.lock().await;
        if s.is_role_running(&role.id) {
            return out;
        }
    }
    // 解锁后执行慢操作（浏览器检测 + 启动）
    match launcher::launch_role_no_session(&cfg, &role, true).await {
        Ok(browser) => {
            let mut s = session.lock().await;
            s.roles.insert(
                role.id.clone(),
                crate::session::RunningRole { browser, active_page: None },
            );
            // 自动打开预设 URL
            if let Some(run) = s.roles.get_mut(&role.id) {
                for url in launcher::collect_auto_open_urls(&role, &cfg) {
                    if let Ok(page) = run.browser.new_page(chromiumoxide_cdp::cdp::browser_protocol::target::CreateTargetParams::new(&url)).await {
                        let _ = page.activate().await;
                        run.active_page = Some(page.target_id().clone());
                    }
                }
            }
            out.push_ok();
        }
        Err(e) => out.push_error(e),
    }
    out
}

/// 关闭单个角色（在 spawn 任务内调用）。
/// 并行安全：仅对 Session/cfg 短暂持锁提取数据，慢速 CDP 工作在无锁状态下执行。
async fn close_one(
    session: Arc<tokio::sync::Mutex<Session>>,
    store: ConfigStore,
    cfg: Arc<tokio::sync::Mutex<GlobalConfig>>,
    id: String,
) -> BatchResult {
    let mut out = BatchResult::default();

    // 1. 短暂持锁：从 Session 取出 RunningRole，从 cfg 读取 CDP 端口
    let (run, port) = {
        let mut s = session.lock().await;
        let run = match s.roles.remove(&id) {
            Some(r) => r,
            None => return out, // 已不在 session 中（外部已关闭），不算失败
        };
        let c = cfg.lock().await;
        let port = c.roles.iter().find(|r| r.id == id).map(|r| r.cdp_port);
        (run, port)
    };

    // 2. 无锁执行慢速 CDP 工作
    match launcher::close_role_no_session(run.browser, port).await {
        Ok(rect) => {
            // 3. 短暂持锁：保存窗口位置到配置
            if let Some(rect) = rect {
                let mut c = cfg.lock().await;
                if let Some(slot) = c.roles.iter_mut().find(|r| r.id == id) {
                    slot.window_rect = Some(rect);
                }
                let _ = store.save(&c);
            }
            out.push_ok();
        }
        Err(e) => out.push_error(e),
    }
    out
}

/// 一键启动所有：按配置拉起全部角色窗口；已启动的角色不重复开窗。
pub async fn start_all(session: Arc<tokio::sync::Mutex<Session>>, cfg: &GlobalConfig) -> BatchResult {
    // 先清掉外部已关的死角色
    {
        let mut s = session.lock().await;
        launcher::prune_dead_roles(&mut s).await;
    }
    let pending: Vec<crate::model::Role> = {
        let s = session.lock().await;
        pending_roles(&s, cfg, None).into_iter().cloned().collect()
    };
    if pending.is_empty() {
        return BatchResult::default();
    }
    let cfg_clone = cfg.clone();
    let handles: Vec<_> = pending
        .into_iter()
        .map(|role| {
            let sess = Arc::clone(&session);
            let c = cfg_clone.clone();
            tokio::spawn(launch_one(sess, c, role))
        })
        .collect();
    let results = futures::future::join_all(handles).await;
    let mut out = BatchResult::default();
    for r in results {
        match r {
            Ok(batch) => merge_into(&mut out, batch),
            Err(_) => out.failed += 1,
        }
    }
    out
}

/// 启动组：启动某系统下全部角色（已启动的不重复）。
pub async fn start_system(session: Arc<tokio::sync::Mutex<Session>>, cfg: &GlobalConfig, system_id: &str) -> BatchResult {
    {
        let mut s = session.lock().await;
        launcher::prune_dead_roles(&mut s).await;
    }
    let sid = system_id.to_string();
    let pending: Vec<crate::model::Role> = {
        let s = session.lock().await;
        pending_roles(&s, cfg, Some(&sid)).into_iter().cloned().collect()
    };
    if pending.is_empty() {
        return BatchResult::default();
    }
    let cfg_clone = cfg.clone();
    let handles: Vec<_> = pending
        .into_iter()
        .map(|role| {
            let sess = Arc::clone(&session);
            let c = cfg_clone.clone();
            tokio::spawn(launch_one(sess, c, role))
        })
        .collect();
    let results = futures::future::join_all(handles).await;
    let mut out = BatchResult::default();
    for r in results {
        match r {
            Ok(batch) => merge_into(&mut out, batch),
            Err(_) => out.failed += 1,
        }
    }
    out
}

/// 关闭组：CDP 优雅关闭某系统下全部运行中的角色。
pub async fn close_system(
    session: Arc<tokio::sync::Mutex<Session>>,
    store: ConfigStore,
    cfg: Arc<tokio::sync::Mutex<GlobalConfig>>,
    system_id: &str,
) -> BatchResult {
    let to_close: Vec<String> = {
        let s = session.lock().await;
        let c = cfg.lock().await;
        c.roles
            .iter()
            .filter(|r| r.system_id.as_deref() == Some(system_id))
            .filter(|r| s.is_role_running(&r.id))
            .map(|r| r.id.clone())
            .collect()
    };
    if to_close.is_empty() {
        return BatchResult::default();
    }
    let handles: Vec<_> = to_close
        .into_iter()
        .map(|id| {
            let sess = Arc::clone(&session);
            let c = Arc::clone(&cfg);
            tokio::spawn(close_one(sess, store.clone(), c, id))
        })
        .collect();
    let results = futures::future::join_all(handles).await;
    let mut out = BatchResult::default();
    for r in results {
        match r {
            Ok(batch) => merge_into(&mut out, batch),
            Err(_) => out.failed += 1,
        }
    }
    out
}

/// 一键关闭所有：CDP 优雅关闭全部测试窗口，再逐个关闭沙箱。
pub async fn close_all(
    session: Arc<tokio::sync::Mutex<Session>>,
    store: ConfigStore,
    cfg: Arc<tokio::sync::Mutex<GlobalConfig>>,
) -> BatchResult {
    let role_ids: Vec<String> = {
        let s = session.lock().await;
        s.roles.keys().cloned().collect()
    };
    let sb_ids: Vec<String> = {
        let s = session.lock().await;
        s.sandboxes.keys().cloned().collect()
    };
    let mut out = BatchResult::default();
    // 并行关闭角色
    if !role_ids.is_empty() {
        let handles: Vec<_> = role_ids
            .into_iter()
            .map(|id| {
                let sess = Arc::clone(&session);
                let c = Arc::clone(&cfg);
                tokio::spawn(close_one(sess, store.clone(), c, id))
            })
            .collect();
        let results = futures::future::join_all(handles).await;
        for r in results {
            match r {
                Ok(batch) => merge_into(&mut out, batch),
                Err(_) => out.failed += 1,
            }
        }
    }
    // 串行关闭沙箱（沙箱数量通常很少，且 close 含目录删除不宜并发）
    {
        let mut s = session.lock().await;
        for id in &sb_ids {
            match crate::sandbox::close(&mut s, id).await {
                Ok(()) => out.ok += 1,
                Err(e) => out.push_error(e),
            }
        }
    }
    out
}
