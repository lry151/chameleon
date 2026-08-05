//! 单实例锁：工具启动时在应用目录获取排他文件锁，二次启动被拦截并提示中文文案。

use crate::error::{ChameleonError, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

/// 持有锁即代表当前实例独占；进程退出或 drop 时自动释放。
pub struct InstanceLock {
    _file: File,
}

impl InstanceLock {
    /// 获取单实例锁；已有实例在运行则返回 [`ChameleonError::AlreadyRunning`]。
    pub fn acquire(app_dir: &Path) -> Result<InstanceLock> {
        let path = app_dir.join(".chameleon.lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| ChameleonError::Io { detail: e.to_string() })?;
        file.try_lock_exclusive()
            .map_err(|_| ChameleonError::AlreadyRunning)?;
        Ok(InstanceLock { _file: file })
    }
}