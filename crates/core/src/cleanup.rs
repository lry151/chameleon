//! 数据目录清理：一键清理所有测试用临时数据目录（沙箱目录），释放磁盘空间。
//! 角色正式数据目录与配置不受影响。

use crate::error::Result;
use crate::model::GlobalConfig;
use crate::sandbox;
use crate::session::Session;

/// 一键清理：先 CDP 优雅关闭所有运行中的沙箱（关闭即删目录），再扫描删除残留目录。
pub async fn cleanup_all(session: &mut Session, cfg: &GlobalConfig) -> Result<usize> {
    let ids: Vec<String> = session.sandboxes.keys().cloned().collect();
    for id in ids {
        crate::warn_err(sandbox::close(session, &id).await, "清理沙箱关闭失败");
    }
    sandbox::cleanup_orphans(&[], cfg)
}