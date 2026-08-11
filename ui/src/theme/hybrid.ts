import type { ThemeMode } from "../types/api";

/// 根据用户选择的模式 + 系统当前主题判定「是否深色」。
export function resolveIsDark(
  mode: ThemeMode,
  systemIsDark: boolean,
): boolean {
  if (mode === "System") return systemIsDark;
  return mode === "Dark";
}

/// Hybrid 主题背景策略（自适应：以后端检测的 backdrop 能力为准）。
///
/// - 深色 + Mica/Acrylic 可用：body 完全透明，让 DWM 真材质透出。
///   前提：window-vibrancy 已自检系统能渲染 Mica/Acrylic（透明效果开 + DWM 合成开
///   + Win11/10），不再靠猜。半透明面板由 panel_opacity 驱动，能看到真玻璃。
/// - 深色 + None（透明效果关 / RDP / 无 DWM / 旧系统）：body 实色暗底 #16181A，
///   面板不透明。绝不出现透明窗透出桌面 = 白屏（本类 bug 根因）。
/// - 浅色：body 实色 #F3F3F3（参考 Windows 11 设置应用）。
export const LIGHT_BG = "#F3F3F3";
export const DARK_SOLID_BG = "#16181A";

/// `backdropCapable` = 后端 detect_backdrop_capability() 是否给出 Mica/Acrylic。
export function applyBodyBackground(
  isDark: boolean,
  backdropCapable: boolean,
): void {
  const { style } = document.body;
  if (isDark) {
    style.backgroundColor = backdropCapable ? "transparent" : DARK_SOLID_BG;
    style.color = "#E8E8E8";
  } else {
    style.backgroundColor = LIGHT_BG;
    style.color = "#1A1A1A";
  }
  style.colorScheme = isDark ? "dark" : "light";
}
