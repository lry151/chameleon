//! 一键启动 / 一键关闭 / 启动组：批量拉起全部角色 / 启动某系统下全部角色；
//! CDP 优雅关闭全部测试窗口。仅作用于测试数据目录，绝不触碰日常浏览器配置。

use crate::config::ConfigStore;
use crate::launcher;
use crate::model::GlobalConfig;
use crate::session::Session;
use serde::Serialize;

/// 批量操作结果：成功数 + 失败的中文文案列表（统一走错误文案层）。
#[derive(Debug, Default, Serialize)]
pub struct BatchResult {
    pub ok: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

impl BatchResult {
    fn push_error(&mut self, e: crate::error::ChameleonError) {
        self.failed += 1;
        self.errors.push(e.message());
    }
}

/// 一键启动所有：按配置拉起全部角色窗口；已启动的角色不重复开窗。
pub async fn start_all(session: &mut Session, cfg: &GlobalConfig) -> BatchResult {
    let mut out = BatchResult::default();
    launcher::prune_dead_roles(session).await; // 清掉外部已关的死角色，避免被误判「运行中」而跳过
    for role in &cfg.roles {
        if session.is_role_running(&role.id) {
            continue;
        }
        match launcher::launch_role(session, cfg, role, true).await {
            Ok(()) => out.ok += 1,
            Err(e) => out.push_error(e),
        }
    }
    out
}

/// 启动组：启动某系统下全部角色（已启动的不重复）。
pub async fn start_system(session: &mut Session, cfg: &GlobalConfig, system_id: &str) -> BatchResult {
    let mut out = BatchResult::default();
    launcher::prune_dead_roles(session).await;
    for role in &cfg.roles {
        if role.system_id.as_deref() != Some(system_id) {
            continue;
        }
        if session.is_role_running(&role.id) {
            continue;
        }
        match launcher::launch_role(session, cfg, role, true).await {
            Ok(()) => out.ok += 1,
            Err(e) => out.push_error(e),
        }
    }
    out
}

/// 一键关闭所有：CDP 优雅关闭全部测试窗口（记录各自窗口位置），
/// 再逐个关闭沙箱窗口并删除其临时数据目录。单个沙箱已退出/失败不中断整批。
pub async fn close_all(session: &mut Session, store: &ConfigStore, cfg: &mut GlobalConfig) -> BatchResult {
    let mut out = BatchResult::default();
    launcher::prune_dead_roles(session).await;
    let ids: Vec<String> = session.roles.keys().cloned().collect();
    for id in ids {
        match launcher::close_role(session, store, cfg, &id).await {
            Ok(()) => out.ok += 1,
            Err(e) => out.push_error(e),
        }
    }
    // 清场彻底：沙箱也逐个关闭（close 会删临时数据目录）；已退出的沙箱
    // 已不在 session.sandboxes 中，自然不会报错中断整批。
    let sb_ids: Vec<String> = session.sandboxes.keys().cloned().collect();
    for id in sb_ids {
        match crate::sandbox::close(session, &id).await {
            Ok(()) => out.ok += 1,
            Err(e) => out.push_error(e),
        }
    }
    out
}
