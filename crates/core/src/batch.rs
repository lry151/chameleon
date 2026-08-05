//! 一键启动 / 一键关闭：批量拉起全部角色窗口；CDP 优雅关闭全部测试窗口。
//!
//! 仅作用于测试数据目录中的窗口，绝不触碰日常浏览器配置。

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
    for role in &cfg.roles {
        if session.is_role_running(&role.id) {
            continue;
        }
        match launcher::launch_role(session, cfg, role).await {
            Ok(()) => out.ok += 1,
            Err(e) => out.push_error(e),
        }
    }
    out
}

/// 一键关闭所有：CDP 优雅关闭全部测试窗口（记录各自窗口位置）。
pub async fn close_all(session: &mut Session, store: &ConfigStore, cfg: &mut GlobalConfig) -> BatchResult {
    let mut out = BatchResult::default();
    let ids: Vec<String> = session.roles.keys().cloned().collect();
    for id in ids {
        match launcher::close_role(session, store, cfg, &id).await {
            Ok(()) => out.ok += 1,
            Err(e) => out.push_error(e),
        }
    }
    out
}