//! 配置管理：config.json 读写往返（原子写）、角色增删改、端口分配持久化。

use crate::error::{ChameleonError, Result};
use crate::model::{GlobalConfig, Role};
use crate::ports;
use crate::safety;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 配置仓库：config.json 为唯一配置源（明文 JSON，人工可改）。
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 加载配置；文件不存在时返回默认配置（不落盘，首次保存时落盘）。
    pub fn load(&self) -> Result<GlobalConfig> {
        if !self.path.exists() {
            return Ok(GlobalConfig::default());
        }
        let raw = fs::read_to_string(&self.path)
            .map_err(|e| ChameleonError::ConfigRead { detail: e.to_string() })?;
        let cfg: GlobalConfig = serde_json::from_str(&raw)
            .map_err(|e| ChameleonError::ConfigInvalid { detail: e.to_string() })?;
        safety::validate_config(&cfg)?;
        Ok(cfg)
    }

    /// 原子保存：先写临时文件再改名，避免写一半损坏配置。
    pub fn save(&self, cfg: &GlobalConfig) -> Result<()> {
        safety::validate_config(cfg)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
        }
        let raw = serde_json::to_string_pretty(cfg)
            .map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)
                .map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
            f.write_all(raw.as_bytes())
                .and_then(|_| f.sync_all())
                .map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
        }
        fs::rename(&tmp, &self.path)
            .map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
        Ok(())
    }

    /// 创建角色：分配空闲端口、校验冲突、持久化。
    /// 数据目录默认落在数据根目录下 `data_root/<name>`。
    pub fn create_role(&self, cfg: &mut GlobalConfig, name: String, color: String) -> Result<Role> {
        if name.trim().is_empty() {
            return Err(ChameleonError::ConfigInvalid { detail: "角色名称不能为空".into() });
        }
        if cfg.roles.iter().any(|r| r.name == name) {
            return Err(ChameleonError::DuplicateName { name });
        }
        let used: Vec<u16> = cfg.roles.iter().map(|r| r.cdp_port).collect();
        let port = ports::pick_role_port(&used)?;
        let profile_dir = cfg.data_root.join(sanitize_dir_name(&name));
        let role = Role::new(name, color, profile_dir, port);
        cfg.roles.push(role.clone());
        self.save(cfg)?;
        Ok(role)
    }

    pub fn update_role(&self, cfg: &mut GlobalConfig, role: Role) -> Result<()> {
        match cfg.roles.iter_mut().find(|r| r.id == role.id) {
            Some(slot) => *slot = role,
            None => return Err(ChameleonError::RoleNotFound { id: role.id }),
        }
        self.save(cfg)
    }

    pub fn delete_role(&self, cfg: &mut GlobalConfig, id: &str) -> Result<()> {
        let before = cfg.roles.len();
        cfg.roles.retain(|r| r.id != id);
        if cfg.roles.len() == before {
            return Err(ChameleonError::RoleNotFound { id: id.into() });
        }
        self.save(cfg)
    }
}

/// 目录名清洗：去掉路径分隔符等危险字符，保证 data_root 下安全落子目录。
pub fn sanitize_dir_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('-');
    if trimmed.is_empty() {
        "role".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 应用目录：exe 所在目录（便携布局：exe + config.json + 数据根一个文件夹可搬运）。
pub fn app_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// 默认数据根目录：应用目录下 `data`，与 exe 随行。
pub fn default_data_root() -> PathBuf {
    app_dir().join("data")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path().join("config.json"));
        let mut cfg = GlobalConfig::default();
        cfg.data_root = dir.path().join("data");
        let role = store.create_role(&mut cfg, "ERP-管理员".into(), "#e74c3c".into()).unwrap();
        assert_eq!(role.name, "ERP-管理员");
        drop(store);

        let store2 = ConfigStore::new(dir.path().join("config.json"));
        let loaded = store2.load().unwrap();
        assert_eq!(loaded.roles.len(), 1);
        assert_eq!(loaded.roles[0].name, "ERP-管理员");
        assert_eq!(loaded.roles[0].cdp_port, role.cdp_port); // 端口持久化，重启不变
        assert!(loaded.roles[0].profile_dir.starts_with(&cfg.data_root));
    }

    #[test]
    fn duplicate_name_rejected() {
        let dir = tempdir().unwrap();
        let store = ConfigStore::new(dir.path().join("config.json"));
        let mut cfg = GlobalConfig::default();
        cfg.data_root = dir.path().join("data");
        store.create_role(&mut cfg, "管理员".into(), "#fff".into()).unwrap();
        assert!(matches!(
            store.create_role(&mut cfg, "管理员".into(), "#000".into()),
            Err(ChameleonError::DuplicateName { .. })
        ));
    }

    #[test]
    fn roles_persist_after_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        {
            let store = ConfigStore::new(&path);
            let mut cfg = GlobalConfig::default();
            cfg.data_root = dir.path().join("data");
            store.create_role(&mut cfg, "审计员".into(), "#3498db".into()).unwrap();
        }
        let store = ConfigStore::new(&path);
        let cfg = store.load().unwrap();
        assert_eq!(cfg.roles.len(), 1);
        assert_eq!(cfg.roles[0].name, "审计员");
    }

    #[test]
    fn sanitize_removes_path_separators() {
        assert_eq!(sanitize_dir_name("ERP/管理员"), "ERP-管理员");
        assert_eq!(sanitize_dir_name(".."), "role");
    }
}