//! 常用 URL 预设：角色级 + 系统级。角色级可标记启动时自动打开。
//! 系统级预设组内所有角色共享（启动时与角色级合并打开）。
//! 预设内部以 `id` 为唯一键；name 是纯显示字段（可空、可重、可改）。

use crate::config::ConfigStore;
use crate::error::{ChameleonError, Result};
use crate::launcher;
use crate::model::{GlobalConfig, QuickLink, QuickLinkLogin};
use crate::session::Session;

/// 新增角色级预设并持久化。id 由服务端生成（uuid），name 可空可重。
pub fn add(store: &ConfigStore, cfg: &mut GlobalConfig, role_id: &str, name: Option<String>, url: &str, auto_open: bool, login: Option<QuickLinkLogin>) -> Result<()> {
    let role = cfg
        .roles
        .iter_mut()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    if url.trim().is_empty() {
        return Err(ChameleonError::ConfigInvalid { detail: "预设地址不能为空".into() });
    }
    role.quick_links.push(QuickLink::new(name, url.to_string(), auto_open, login));
    store.save(cfg)
}

/// 删除角色级预设（按 id）并持久化。
pub fn remove(store: &ConfigStore, cfg: &mut GlobalConfig, role_id: &str, id: &str) -> Result<()> {
    let role = cfg
        .roles
        .iter_mut()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    let before = role.quick_links.len();
    role.quick_links.retain(|q| q.id != id);
    if role.quick_links.len() == before {
        return Err(ChameleonError::ConfigInvalid { detail: "预设不存在".into() });
    }
    store.save(cfg)
}

/// 编辑角色级预设（改 name/改URL/改auto_open）并持久化。id 不变。
pub fn edit(store: &ConfigStore, cfg: &mut GlobalConfig, role_id: &str, id: &str, name: Option<String>, url: &str, auto_open: bool, login: Option<QuickLinkLogin>) -> Result<()> {
    let role = cfg
        .roles
        .iter_mut()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    if url.trim().is_empty() {
        return Err(ChameleonError::ConfigInvalid { detail: "预设地址不能为空".into() });
    }
    let link = role
        .quick_links
        .iter_mut()
        .find(|q| q.id == id)
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "预设不存在".into() })?;
    link.name = name;
    link.url = url.to_string();
    link.auto_open = auto_open;
    link.login = login;
    store.save(cfg)
}

/// 点击预设（按 id）：在该角色窗口新标签页打开（未启动先拉起）。返回打开的地址。
/// 先查角色级预设；若该预设挂有登录凭据则触发自动登录（填用户名+密码）。
/// 角色级没有 → 查所属系统级预设（系统级不支持登录）。
pub async fn open(session: &mut Session, cfg: &GlobalConfig, role_id: &str, id: &str) -> Result<String> {
    let role = cfg
        .roles
        .iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| ChameleonError::RoleNotFound { id: role_id.into() })?;
    // 优先查角色级预设
    let role_link = role.quick_links.iter().find(|q| q.id == id);
    if let Some(link) = role_link {
        if let Some(login) = &link.login {
            // 有登录凭据 → 自动登录（打开 URL + 填用户名密码）
            launcher::login_assist_link(session, cfg, role_id, &link.url, login).await?;
            return Ok(link.url.clone());
        }
        // 无登录 → 正常打开
        launcher::open_tab(session, cfg, role_id, &link.url).await?;
        return Ok(link.url.clone());
    }
    // 角色级没有 → 查所属系统级预设（系统级不支持登录）
    let link = role
        .system_id
        .as_ref()
        .and_then(|sid| {
            cfg.systems
                .iter()
                .find(|s| s.id == *sid)
                .and_then(|s| s.quick_links.iter().find(|q| q.id == id))
        })
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "预设不存在".into() })?;
    launcher::open_tab(session, cfg, role_id, &link.url).await?;
    Ok(link.url.clone())
}

/// 新增系统级预设。id 由服务端生成（uuid），name 可空可重。
pub fn add_system(store: &ConfigStore, cfg: &mut GlobalConfig, system_id: &str, name: Option<String>, url: &str, auto_open: bool) -> Result<()> {
    let sys = cfg
        .systems
        .iter_mut()
        .find(|s| s.id == system_id)
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "系统不存在".into() })?;
    if url.trim().is_empty() {
        return Err(ChameleonError::ConfigInvalid { detail: "预设地址不能为空".into() });
    }
    sys.quick_links.push(QuickLink::new(name, url.to_string(), auto_open, None));
    store.save(cfg)
}

/// 删除系统级预设（按 id）并持久化。
pub fn remove_system(store: &ConfigStore, cfg: &mut GlobalConfig, system_id: &str, id: &str) -> Result<()> {
    let sys = cfg
        .systems
        .iter_mut()
        .find(|s| s.id == system_id)
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "系统不存在".into() })?;
    let before = sys.quick_links.len();
    sys.quick_links.retain(|q| q.id != id);
    if sys.quick_links.len() == before {
        return Err(ChameleonError::ConfigInvalid { detail: "预设不存在".into() });
    }
    store.save(cfg)
}

/// 编辑系统级预设（改 name/改URL/改auto_open）并持久化。id 不变。
pub fn edit_system(store: &ConfigStore, cfg: &mut GlobalConfig, system_id: &str, id: &str, name: Option<String>, url: &str, auto_open: bool) -> Result<()> {
    let sys = cfg
        .systems
        .iter_mut()
        .find(|s| s.id == system_id)
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "系统不存在".into() })?;
    if url.trim().is_empty() {
        return Err(ChameleonError::ConfigInvalid { detail: "预设地址不能为空".into() });
    }
    let link = sys
        .quick_links
        .iter_mut()
        .find(|q| q.id == id)
        .ok_or_else(|| ChameleonError::ConfigInvalid { detail: "预设不存在".into() })?;
    link.name = name;
    link.url = url.to_string();
    link.auto_open = auto_open;
    store.save(cfg)
}
