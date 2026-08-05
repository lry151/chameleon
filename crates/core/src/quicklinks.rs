//! 常用 URL 预设：角色可配置常用测试地址（名称 + 地址），点击即在新标签页打开。

use crate::config::ConfigStore;
use crate::error::{ChameleonError, Result};
use crate::launcher;
use crate::model::{GlobalConfig, QuickLink};
use crate::session::Session;

/// 新增预设并持久化。
pub fn add(store: &ConfigStore, cfg: &mut GlobalConfig, role_id: &str, name: &str, url: &str) -> Result<()> {
    let role = cfg
        .roles
        .iter_mut()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    if name.trim().is_empty() || url.trim().is_empty() {
        return Err(ChameleonError::ConfigInvalid { detail: "预设名称与地址不能为空".into() });
    }
    if role.quick_links.iter().any(|q| q.name == name) {
        return Err(ChameleonError::ConfigInvalid { detail: format!("预设「{name}」已存在") });
    }
    role.quick_links.push(QuickLink {
        name: name.to_string(),
        url: url.to_string(),
    });
    store.save(cfg)
}

/// 删除预设并持久化。
pub fn remove(store: &ConfigStore, cfg: &mut GlobalConfig, role_id: &str, name: &str) -> Result<()> {
    let role = cfg
        .roles
        .iter_mut()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    let before = role.quick_links.len();
    role.quick_links.retain(|q| q.name != name);
    if role.quick_links.len() == before {
        return Err(ChameleonError::ConfigInvalid { detail: format!("预设「{name}」不存在") });
    }
    store.save(cfg)
}

/// 点击预设：在该角色窗口新标签页打开（未启动先拉起）。返回打开的地址。
pub async fn open(session: &mut Session, cfg: &GlobalConfig, role_id: &str, name: &str) -> Result<String> {
    let role = cfg
        .roles
        .iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    let link = role
        .quick_links
        .iter()
        .find(|q| q.name == name)
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: format!("预设「{name}」不存在") })?;
    launcher::open_tab(session, cfg, role_id, &link.url).await?;
    Ok(link.url.clone())
}