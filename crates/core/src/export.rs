//! 配置导出 / 导入：导出角色配置为明文 JSON；导入校验默认目录拒绝与端口冲突，
//! 冲突时给出中文提示且不破坏现有配置。新人入职直接导入即可复用整套角色。

use crate::config::{role_profile_dir, ConfigStore};
use crate::error::{ChameleonError, Result};
use crate::model::{GlobalConfig, Role};
use crate::safety;
use std::fs;
use std::path::{Path, PathBuf};

/// 导出：把当前角色配置写成明文 JSON 文件（含名称/颜色/预设/端口）。
pub fn export_config(cfg: &GlobalConfig, dest: &Path) -> Result<()> {
    let data = serde_json::to_string_pretty(cfg)
        .map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
    fs::write(dest, data).map_err(|e| ChameleonError::ConfigWrite { detail: e.to_string() })?;
    Ok(())
}

/// 导入角色在本机数据根的落位目录：按导入文件里的系统名解析（与 create_role 同规则）。
fn imported_role_dir(cfg: &GlobalConfig, imported: &GlobalConfig, role: &Role) -> PathBuf {
    let system_name = role.system_id.as_deref()
        .and_then(|id| imported.systems.iter().find(|s| s.id == id))
        .map(|s| s.name.as_str());
    role_profile_dir(&cfg.data_root, system_name, &role.name)
}

/// 导入：解析文件 → 校验（默认目录拒绝 / 端口 / 名称冲突）→ 合并进现有配置。
///
/// 导入角色数据目录重写到本机数据根目录下；任何校验失败返回中文错误且不改动现有配置。
pub fn import_config(store: &ConfigStore, cfg: &mut GlobalConfig, src: &Path) -> Result<usize> {
    let raw = fs::read_to_string(src)
        .map_err(|e| ChameleonError::ImportInvalid { detail: format!("无法读取文件：{e}") })?;
    let imported: GlobalConfig = serde_json::from_str(&raw)
        .map_err(|e| ChameleonError::ImportInvalid { detail: format!("不是有效的配置文件：{e}") })?;
    if imported.roles.is_empty() {
        return Err(ChameleonError::ImportInvalid { detail: "文件中没有角色".into() });
    }
    // 导入文件自身一致性校验（重名/重端口/重目录/默认目录）
    safety::validate_config(&imported)?;

    // 与现有配置的冲突检查：全部通过才允许导入。
    // 名称唯一性按系统作用域：导入角色随其所属系统落位数据目录，跨系统同名允许。
    for r in &imported.roles {
        if let Some(d) = safety::touches_default_dir(&r.profile_dir) {
            return Err(ChameleonError::DefaultDirRefused { dir: d });
        }
        if cfg.roles.iter().any(|o| o.cdp_port == r.cdp_port) {
            return Err(ChameleonError::PortConflict { port: r.cdp_port });
        }
        if cfg.roles.iter().any(|o| o.system_id == r.system_id && o.name == r.name) {
            return Err(ChameleonError::DuplicateName { name: r.name.clone() });
        }
        let local_dir = imported_role_dir(&cfg, &imported, r);
        if cfg.roles.iter().any(|o| o.profile_dir == local_dir) {
            return Err(ChameleonError::DuplicateDir { dir: local_dir });
        }
    }

    // 合并：重写数据目录到本机数据根，重新生成 id（避免重复导入 id 冲突）
    let systems = &imported.systems;
    let n = imported.roles.len();
    cfg.roles.extend(imported.roles.into_iter().map(|mut r| {
        r.id = uuid::Uuid::new_v4().to_string();
        let system_name = r.system_id.as_deref()
            .and_then(|id| systems.iter().find(|s| s.id == id))
            .map(|s| s.name.as_str());
        r.profile_dir = role_profile_dir(&cfg.data_root, system_name, &r.name);
        r
    }));
    store.save(cfg)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture(dir: &std::path::Path) -> (ConfigStore, GlobalConfig) {
        let store = ConfigStore::new(dir.join("config.json"));
        let mut cfg = GlobalConfig::default();
        cfg.data_root = dir.join("data");
        (store, cfg)
    }

    #[test]
    fn export_roundtrip() {
        let dir = tempdir().unwrap();
        let (store, mut cfg) = fixture(dir.path());
        store.create_role(&mut cfg, "ERP-管理员".into(), "#e74c3c".into(), None).unwrap();
        let dest = dir.path().join("export.json");
        export_config(&cfg, &dest).unwrap();
        let raw = fs::read_to_string(&dest).unwrap();
        assert!(raw.contains("ERP-管理员"));
        assert!(raw.contains("#e74c3c"));
    }

    #[test]
    fn import_merges_roles_and_preserves_ports() {
        let dir = tempdir().unwrap();
        let (store, mut cfg) = fixture(dir.path());
        let r = store.create_role(&mut cfg, "管理员".into(), "#fff".into(), None).unwrap();
        let port = r.cdp_port;

        // 构造导入文件：另一个数据根下的角色
        let mut other = GlobalConfig::default();
        other.data_root = dir.path().join("other-data");
        let (store2, mut cfg2) = (ConfigStore::new(dir.path().join("other.json")), other);
        let r2 = store2.create_role(&mut cfg2, "审计员".into(), "#3498db".into(), None).unwrap();
        let port2 = r2.cdp_port;
        let src = dir.path().join("import.json");
        export_config(&cfg2, &src).unwrap();

        let n = import_config(&store, &mut cfg, &src).unwrap();
        assert_eq!(n, 1);
        assert_eq!(cfg.roles.len(), 2);
        let imported = cfg.roles.iter().find(|x| x.name == "审计员").unwrap();
        // 端口保留
        assert_eq!(imported.cdp_port, port2);
        // 数据目录重写为本机数据根
        assert!(imported.profile_dir.starts_with(&cfg.data_root));
        assert_ne!(imported.id, r2.id);
        // 现有角色不受影响
        assert_eq!(cfg.roles.iter().find(|x| x.name == "管理员").unwrap().cdp_port, port);
    }

    #[test]
    fn import_rejects_port_conflict_without_mutation() {
        let dir = tempdir().unwrap();
        let (store, mut cfg) = fixture(dir.path());
        let r = store.create_role(&mut cfg, "管理员".into(), "#fff".into(), None).unwrap();
        let port = r.cdp_port;

        let mut other = GlobalConfig::default();
        other.data_root = dir.path().join("other");
        let (store2, mut cfg2) = (ConfigStore::new(dir.path().join("o.json")), other);
        store2.create_role(&mut cfg2, "审计员".into(), "#3498db".into(), None).unwrap();
        // 手动制造端口冲突
        cfg2.roles[0].cdp_port = port;
        let src = dir.path().join("imp.json");
        export_config(&cfg2, &src).unwrap();

        assert!(matches!(
            import_config(&store, &mut cfg, &src),
            Err(ChameleonError::PortConflict { .. })
        ));
        assert_eq!(cfg.roles.len(), 1, "现有配置不得被破坏");
    }

    #[test]
    fn import_rejects_default_dir() {
        let dir = tempdir().unwrap();
        let (store, mut cfg) = fixture(dir.path());
        let mut other = GlobalConfig::default();
        other.data_root = dir.path().join("o");
        let (store2, mut cfg2) = (ConfigStore::new(dir.path().join("o.json")), other);
        store2.create_role(&mut cfg2, "审计员".into(), "#3498db".into(), None).unwrap();
        cfg2.roles[0].profile_dir = std::path::PathBuf::from(r"C:\Users\t\AppData\Local\Google\Chrome\User Data");
        let src = dir.path().join("imp.json");
        export_config(&cfg2, &src).unwrap();

        assert!(matches!(
            import_config(&store, &mut cfg, &src),
            Err(ChameleonError::DefaultDirRefused { .. })
        ));
        assert_eq!(cfg.roles.len(), 0);
    }
}