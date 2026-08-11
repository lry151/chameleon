//! Hybrid 主题 vibrancy 策略。
//!
//! 把「决定做什么」与「调用 window-vibrancy」拆开：
//! - [`plan_vibrancy`] 是纯函数，易测试，覆盖 4 种 (OS, 有效主题) 组合。
//! - [`apply_vibrancy_for_theme`] 组合 detect_os + system_is_light + plan + apply，
//!   供启动与 `set_ui_preferences` 调用。

use chameleon_core::ThemeMode;

/// 经 OS 检测后的分类。Mica 需要 Win11 (build 22000+)；更低的 Win10 走 Acrylic。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Win11Plus/Win10 仅 cfg(windows) 命中；单元测试覆盖全部枚举。
pub enum OsFlavor {
    Win11Plus,
    Win10,
    Unsupported,
}

/// Vibrancy 计划：调用端按枚举分支调用对应的 window-vibrancy 函数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrancyPlan {
    /// Win11 深色：Mica 背景（tint 由系统决定）。
    Mica,
    /// Win10 深色：Acrylic + 深色 tint（精心调校的 RGBA）。
    AcrylicDark,
    /// 浅色主题：关闭 vibrancy，前端实色背景填充。
    Clear,
}

/// 决定 vibrancy 计划。纯函数，便于单元测试。
///
/// - `theme == System` 时由 `system_is_light` 决定有效明暗。
/// - 有效主题 = 浅色 → `Clear`（不论 OS）。
/// - 有效主题 = 深色 + Win11 → `Mica`。
/// - 有效主题 = 深色 + Win10/其他 → `AcrylicDark`。
pub fn plan_vibrancy(
    theme: ThemeMode,
    system_is_light: bool,
    os: OsFlavor,
) -> VibrancyPlan {
    let effective_dark = match theme {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => !system_is_light,
    };
    if !effective_dark {
        return VibrancyPlan::Clear;
    }
    match os {
        OsFlavor::Win11Plus => VibrancyPlan::Mica,
        _ => VibrancyPlan::AcrylicDark,
    }
}

/// 检测当前 OS 类别。非 Windows 一律 Unsupported。
pub fn detect_os() -> OsFlavor {
    #[cfg(target_os = "windows")]
    {
        let ver = windows_version::OsVersion::current();
        if ver.major >= 10 && ver.build >= 22000 {
            OsFlavor::Win11Plus
        } else if ver.major == 10 {
            OsFlavor::Win10
        } else {
            OsFlavor::Unsupported
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        OsFlavor::Unsupported
    }
}

/// 一台机器真实的背景材质能力（运行时自检，不靠假设）。
///
/// 关键事实：`apply_mica`/`apply_acrylic` 只是调 `DwmSetWindowAttribute`，
/// DWM 在「系统透明效果关闭 / 无合成器（RDP、VM）」时会**静默忽略**，API
/// 照样返回 Ok——所以「调用成功」不等于「效果渲染」。前端必须拿到这个能力
/// 信号来决定 CSS 策略，否则就是猜（历史上猜错 = 白屏 / Mica 被内容盖住）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackdropCapability {
    /// Win11 + 透明效果开 + DWM 合成可用 → 真 Mica。
    Mica,
    /// Win10 + 同上 → Acrylic。
    Acrylic,
    /// 其余（透明效果关 / RDP / 无 DWM / 旧系统）→ 纯实色，前端不透明。
    None,
}

impl BackdropCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackdropCapability::Mica => "Mica",
            BackdropCapability::Acrylic => "Acrylic",
            BackdropCapability::None => "None",
        }
    }
}

#[allow(dead_code)] // 仅 cfg(windows) 的 detect_backdrop_capability 调用；单元测试覆盖。
/// 纯决策函数：OS + 系统透明效果 + DWM 合成 三信号 → 能力。便于单元测试。
pub fn decide_backdrop(
    os: OsFlavor,
    transparency_on: bool,
    composition_on: bool,
) -> BackdropCapability {
    if !transparency_on || !composition_on {
        return BackdropCapability::None;
    }
    match os {
        OsFlavor::Win11Plus => BackdropCapability::Mica,
        OsFlavor::Win10 => BackdropCapability::Acrylic,
        OsFlavor::Unsupported => BackdropCapability::None,
    }
}

/// 检测当前机器背景材质能力。非 Windows 一律 None。
pub fn detect_backdrop_capability() -> BackdropCapability {
    #[cfg(target_os = "windows")]
    {
        decide_backdrop(
            detect_os(),
            // EnableTransparency 缺失时系统默认开启透明效果。
            read_enable_transparency().unwrap_or(true),
            dwm_composition_enabled(),
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        BackdropCapability::None
    }
}

/// 查询系统当前 app 主题是否浅色（读注册表
/// `HKCU\…\Themes\Personalize\AppsUseLightTheme`）。
///
/// 失败/缺失时默认返回 `false`（按深色处理），与历史行为一致。
pub fn system_is_light() -> bool {
    #[cfg(target_os = "windows")]
    {
        read_apps_use_light_theme().unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn read_apps_use_light_theme() -> Option<bool> {
    use windows::core::w;
    read_personalize_dword(w!("AppsUseLightTheme")).map(|v| v == 1)
}

/// 系统「透明效果」开关（`EnableTransparency`）。缺失时视为开启（系统默认）。
#[cfg(target_os = "windows")]
fn read_enable_transparency() -> Option<bool> {
    use windows::core::w;
    read_personalize_dword(w!("EnableTransparency")).map(|v| v == 1)
}

/// DWM 桌面合成是否可用。RDP 基础主题 / 部分虚拟机下为 false → 无任何半透效果。
#[cfg(target_os = "windows")]
fn dwm_composition_enabled() -> bool {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::Graphics::Dwm::DwmIsCompositionEnabled;
    let mut enabled = BOOL::default();
    unsafe { DwmIsCompositionEnabled(&mut enabled) }.is_ok() && enabled.as_bool()
}

/// 读 `HKCU\…\Themes\Personalize` 下的 DWORD 值。
#[cfg(target_os = "windows")]
fn read_personalize_dword(value_name: &windows::core::HSTRING) -> Option<u32> {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        REG_VALUE_TYPE, RRF_RT_REG_DWORD,
    };
    use windows::core::w;

    let sub_key = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

let mut key = HKEY::default();
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            sub_key,
            Some(0),
            KEY_READ,
            &mut key,
        )
    };
    if result.is_err() {
        return None;
    }

    let mut ty = REG_VALUE_TYPE::default();
    let mut data: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let result = unsafe {
        RegQueryValueExW(
            key,
            value_name,
            None,
            Some(&mut ty),
            Some(&mut data as *mut u32 as *mut u8),
            Some(&mut size),
        )
    };
    unsafe { let _ = RegCloseKey(key); }
    if result.is_err() {
        return None;
    }
    Some(data == 1)
}

/// 把 [`VibrancyPlan`] 作用到窗口。非 Windows 是 no-op（返回 Ok）。
pub fn apply(window: &tauri::WebviewWindow, plan: VibrancyPlan) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_mica, clear_vibrancy};
        // 先清空：避免旧 vibrancy 残留与新 plan 叠加。
        let _ = clear_vibrancy(window);
        match plan {
            VibrancyPlan::Mica => apply_mica(window, None)
                .map_err(|e| format!("apply_mica 失败: {e}")),
            VibrancyPlan::AcrylicDark => apply_acrylic(window, Some((33, 31, 41, 180)))
                .map_err(|e| format!("apply_acrylic 失败: {e}")),
            VibrancyPlan::Clear => Ok(()),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, plan);
        Ok(())
    }
}

/// 入口：按当前 OS + 系统主题 + 用户选择，一步完成 vibrancy 应用。
pub fn apply_vibrancy_for_theme(
    window: &tauri::WebviewWindow,
    theme: ThemeMode,
) -> Result<(), String> {
    let plan = plan_vibrancy(theme, system_is_light(), detect_os());
    apply(window, plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_dark_win11_is_mica() {
        assert_eq!(
            plan_vibrancy(ThemeMode::Dark, false, OsFlavor::Win11Plus),
            VibrancyPlan::Mica
        );
    }

    #[test]
    fn plan_dark_win10_is_acrylic() {
        assert_eq!(
            plan_vibrancy(ThemeMode::Dark, false, OsFlavor::Win10),
            VibrancyPlan::AcrylicDark
        );
    }

    #[test]
    fn plan_light_any_os_is_clear() {
        for os in [OsFlavor::Win11Plus, OsFlavor::Win10, OsFlavor::Unsupported] {
            assert_eq!(
                plan_vibrancy(ThemeMode::Light, false, os),
                VibrancyPlan::Clear
            );
        }
    }

    #[test]
    fn plan_system_follows_system_light_flag() {
        // 系统浅色 → Clear；系统深色 → 按 OS 选 Mica / AcrylicDark
        assert_eq!(
            plan_vibrancy(ThemeMode::System, true, OsFlavor::Win11Plus),
            VibrancyPlan::Clear
        );
        assert_eq!(
            plan_vibrancy(ThemeMode::System, false, OsFlavor::Win11Plus),
            VibrancyPlan::Mica
        );
        assert_eq!(
            plan_vibrancy(ThemeMode::System, false, OsFlavor::Win10),
            VibrancyPlan::AcrylicDark
        );
    }

    #[test]
    fn plan_unsupported_os_falls_back_to_acrylic_dark_when_dark() {
        // Unsupported OS（例如 Win7 / 非 Windows）在深色模式下走 AcrylicDark；
        // window-vibrancy 会自行在运行时失败，前端降级到实色背景，不影响启动。
        assert_eq!(
            plan_vibrancy(ThemeMode::Dark, false, OsFlavor::Unsupported),
            VibrancyPlan::AcrylicDark
        );
    }

    #[test]
    fn backdrop_win11_with_transparency_and_composition_is_mica() {
        assert_eq!(
            decide_backdrop(OsFlavor::Win11Plus, true, true),
            BackdropCapability::Mica
        );
    }

    #[test]
    fn backdrop_win10_with_transparency_and_composition_is_acrylic() {
        assert_eq!(
            decide_backdrop(OsFlavor::Win10, true, true),
            BackdropCapability::Acrylic
        );
    }

    #[test]
    fn backdrop_transparency_off_collapses_to_none_even_on_win11() {
        // 系统透明效果关闭 → Mica/Acrylic 不渲染 → 实色，不论 OS
        for os in [OsFlavor::Win11Plus, OsFlavor::Win10] {
            assert_eq!(
                decide_backdrop(os, false, true),
                BackdropCapability::None
            );
        }
    }

    #[test]
    fn backdrop_no_dwm_composition_collapses_to_none() {
        // RDP 基础主题 / 无合成器 → 实色
        for os in [OsFlavor::Win11Plus, OsFlavor::Win10] {
            assert_eq!(
                decide_backdrop(os, true, false),
                BackdropCapability::None
            );
        }
    }

    #[test]
    fn backdrop_unsupported_os_is_none() {
        assert_eq!(
            decide_backdrop(OsFlavor::Unsupported, true, true),
            BackdropCapability::None
        );
    }
}
