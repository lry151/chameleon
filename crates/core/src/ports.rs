//! 空闲端口分配：创建角色时挑选空闲端口并持久化，重启不变。

use std::net::TcpListener;

/// 分配一个当前空闲的 TCP 端口（绑定 0 → 读取实际端口 → 释放）。
pub fn pick_free_port() -> crate::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| crate::ChameleonError::Io { detail: e.to_string() })?;
    let port = listener
        .local_addr()
        .map_err(|e| crate::ChameleonError::Io { detail: e.to_string() })?
        .port();
    Ok(port)
}

/// 挑选一个既空闲又未与本配置中其他角色冲突的端口。
pub fn pick_role_port(used: &[u16]) -> crate::Result<u16> {
    for _ in 0..64 {
        let port = pick_free_port()?;
        if !used.contains(&port) {
            return Ok(port);
        }
    }
    Err(crate::ChameleonError::PortConflict { port: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picked_port_is_free() {
        let port = pick_free_port().unwrap();
        // 能再次绑定说明端口确实空闲
        let l = TcpListener::bind(("127.0.0.1", port)).unwrap();
        drop(l);
    }

    #[test]
    fn picked_port_avoids_used_list() {
        let a = pick_free_port().unwrap();
        let b = pick_role_port(&[a]).unwrap();
        assert_ne!(a, b);
    }
}