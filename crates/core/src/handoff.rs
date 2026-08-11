//! Handoff 会话接力：把源角色当前激活标签页 URL 传递到目标角色新标签页并聚焦。
//!
//! 并行模式保留源窗口；接力模式 CDP 优雅关闭源窗口。目标角色未启动先拉起。

use crate::config::ConfigStore;
use crate::error::{ChameleonError, Result};
use crate::launcher;
use crate::model::GlobalConfig;
use crate::session::Session;

/// 接力模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandoffMode {
    /// 并行模式：保留源角色窗口，双窗口并排对比权限差异。
    Parallel,
    /// 接力模式：CDP 优雅关闭源角色窗口，仅保留目标角色窗口。
    Relay,
}

/// 执行接力：读源角色激活标签 URL → 拉起目标角色（如未启动）→
/// （接力模式优雅关闭源）→ 目标新标签页打开并聚焦。返回传递的 URL。
pub async fn handoff(
    session: &mut Session,
    store: &ConfigStore,
    cfg: &mut GlobalConfig,
    source_id: &str,
    target_id: &str,
    mode: HandoffMode,
) -> Result<String> {
    let url = launcher::read_active_tab(session, source_id).await?;
    let target = cfg
        .roles
        .iter()
        .find(|r| r.id == target_id)
        .ok_or_else(|| ChameleonError::RoleNotFound {
            id: target_id.into(),
        })?
        .clone();
    if !session.is_role_running(target_id) {
        launcher::launch_role(session, cfg, &target, true).await?;
    }
    if mode == HandoffMode::Relay {
        launcher::close_role(session, store, cfg, source_id).await?;
    }
    launcher::open_tab(session, cfg, target_id, &url).await?;
    Ok(url)
}
