import type { ThemeMode } from "../types/api";

/// 根据用户选择的模式 + 系统当前主题判定「是否深色」。
export function resolveIsDark(
  mode: ThemeMode,
  systemIsDark: boolean,
): boolean {
  if (mode === "System") return systemIsDark;
  return mode === "Dark";
}

/// Hybrid 主题背景策略：
/// - 深色：半透明暗底（Mica 生效时透出微光；Mica 失败时不露出桌面/白窗）。
///   纯 transparent 依赖 Mica 一定成功，失败则整窗透出桌面 = 白面板（本 bug 根因）。
///   0.78 保证最坏情况（白壁纸）下仍是明确深色（≈rgb(72,72,72)），同时给
///   panel_opacity 控制的半透明面板留下可透出的背衬。
/// - 浅色：body 实色 #F3F3F3（参考 Windows 11 设置应用）。
export const LIGHT_BG = "#F3F3F3";
export const DARK_BG = "rgba(20, 20, 20, 0.78)";

export function applyBodyBackground(isDark: boolean): void {
  const { style } = document.body;
  if (isDark) {
    style.backgroundColor = DARK_BG;
    style.color = "#E8E8E8";
  } else {
    style.backgroundColor = LIGHT_BG;
    style.color = "#1A1A1A";
  }
  style.colorScheme = isDark ? "dark" : "light";
}
