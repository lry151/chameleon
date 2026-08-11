//! 窗口位置记忆：读取当前窗口位置（CDP `Browser.getWindowForTarget`），
//! 启动时通过 `--window-position` / `--window-size` 恢复上次位置与大小。

use crate::error::{ChameleonError, Result};
use crate::model::WindowRect;
use chromiumoxide::browser::Browser;
use chromiumoxide_cdp::cdp::browser_protocol::browser::GetWindowForTargetParams;

/// 读取浏览器窗口当前的位置与大小（像素）。
pub async fn capture_bounds(browser: &Browser) -> Result<WindowRect> {
    let ret = browser
        .execute(GetWindowForTargetParams::default())
        .await
        .map_err(|e| ChameleonError::CdpOperation {
            detail: e.to_string(),
        })?;
    let b = &ret.result.bounds;
    let rect = WindowRect {
        x: b.left.unwrap_or(0) as i32,
        y: b.top.unwrap_or(0) as i32,
        width: b.width.unwrap_or(0) as u32,
        height: b.height.unwrap_or(0) as u32,
    };
    if rect.width == 0 || rect.height == 0 {
        return Err(ChameleonError::CdpOperation {
            detail: "读取窗口尺寸失败".into(),
        });
    }
    Ok(rect)
}
