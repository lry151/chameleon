//! 会话快照 / 恢复：一键保存所有角色的打开标签页 URL 与窗口位置为 JSON 快照；
//! 一键恢复到指定快照，适合长流程回归测试多次回到同一组页面。

use crate::config::ConfigStore;
use crate::error::{ChameleonError, Result};
use crate::launcher;
use crate::model::{GlobalConfig, Snapshot, SnapshotRole};
use crate::session::Session;
use crate::window;
use chromiumoxide_cdp::cdp::browser_protocol::target::TargetId;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// 快照仓库：快照目录下每个快照一个明文 JSON 文件。
pub struct SnapshotStore {
    dir: PathBuf,
}

impl SnapshotStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.json", sanitize_name(name)))
    }

    /// 保存快照：每角色标签页 URL 列表 + 窗口位置。
    pub async fn save(
        &self,
        session: &mut Session,
        store: &ConfigStore,
        cfg: &mut GlobalConfig,
        name: &str,
    ) -> Result<()> {
        if name.trim().is_empty() {
            return Err(ChameleonError::ConfigInvalid {
                detail: "快照名称不能为空".into(),
            });
        }
        fs::create_dir_all(&self.dir).map_err(|e| ChameleonError::SnapshotWrite {
            detail: e.to_string(),
        })?;
        let mut roles = Vec::new();
        for role in &cfg.roles {
            let running = session.roles.contains_key(&role.id);
            let tabs = if running {
                launcher::list_tab_urls(session, &role.id).await
            } else {
                Vec::new()
            };
            let rect = if running {
                let run = session.roles.get(&role.id).unwrap();
                window::capture_bounds(&run.browser).await.ok()
            } else {
                role.window_rect
            };
            roles.push(SnapshotRole {
                role_id: role.id.clone(),
                role_name: role.name.clone(),
                tabs,
                window_rect: rect,
            });
        }
        let snap = Snapshot {
            name: name.to_string(),
            created_at: Utc::now().to_rfc3339(),
            roles,
        };
        let raw =
            serde_json::to_string_pretty(&snap).map_err(|e| ChameleonError::SnapshotWrite {
                detail: e.to_string(),
            })?;
        fs::write(self.path_for(name), raw).map_err(|e| ChameleonError::SnapshotWrite {
            detail: e.to_string(),
        })?;
        store.save(cfg)?;
        Ok(())
    }

    /// 列出全部快照名（按名称排序）。
    pub fn list(&self) -> Result<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.dir).map_err(|e| ChameleonError::SnapshotWrite {
            detail: e.to_string(),
        })? {
            let entry = entry.map_err(|e| ChameleonError::SnapshotWrite {
                detail: e.to_string(),
            })?;
            if entry
                .path()
                .extension()
                .map(|e| e == "json")
                .unwrap_or(false)
            {
                if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// 恢复快照：拉起角色（如未启动）→ 恢复窗口位置 → 打开快照标签 → 关闭多余旧标签。
    pub async fn restore(
        &self,
        session: &mut Session,
        store: &ConfigStore,
        cfg: &mut GlobalConfig,
        name: &str,
    ) -> Result<()> {
        let raw = fs::read_to_string(self.path_for(name))
            .map_err(|_| ChameleonError::SnapshotNotFound { name: name.into() })?;
        let snap: Snapshot =
            serde_json::from_str(&raw).map_err(|e| ChameleonError::SnapshotWrite {
                detail: e.to_string(),
            })?;
        for sr in &snap.roles {
            let role = cfg
                .roles
                .iter()
                .find(|r| r.id == sr.role_id)
                .or_else(|| cfg.roles.iter().find(|r| r.name == sr.role_name))
                .cloned();
            let Some(mut role) = role else { continue };
            if let Some(rect) = sr.window_rect {
                role.window_rect = Some(rect); // 应用到本次启动的角色
                if let Some(slot) = cfg.roles.iter_mut().find(|r| r.id == role.id) {
                    slot.window_rect = Some(rect); // 持久化
                }
            }
            if !session.is_role_running(&role.id) {
                launcher::launch_role(session, cfg, &role, false).await?;
            }
            let mut keep: Vec<TargetId> = Vec::new();
            if sr.tabs.is_empty() {
                continue;
            }
            for url in &sr.tabs {
                launcher::open_tab(session, cfg, &role.id, url).await?;
                if let Some(run) = session.roles.get(&role.id) {
                    if let Some(active) = &run.active_page {
                        keep.push(active.clone());
                    }
                }
            }
            launcher::close_other_tabs(session, &role.id, &keep).await;
        }
        store.save(cfg)?;
        Ok(())
    }

    /// 删除快照。
    pub fn delete(&self, name: &str) -> Result<()> {
        let p = self.path_for(name);
        if !p.exists() {
            return Err(ChameleonError::SnapshotNotFound { name: name.into() });
        }
        fs::remove_file(p).map_err(|e| ChameleonError::SnapshotWrite {
            detail: e.to_string(),
        })?;
        Ok(())
    }
}

/// 快照文件名清洗：去掉路径分隔符等危险字符。
fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "snapshot".to_string()
    } else {
        trimmed.to_string()
    }
}
