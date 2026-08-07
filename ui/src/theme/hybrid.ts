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
/// - 深色：body 透明，让原生 Mica / Acrylic 透出。
/// - 浅色：body 实色 #F3F3F3（参考 Windows 11 设置应用）。
export const LIGHT_BG = "#F3F3F3";

export function applyBodyBackground(isDark: boolean): void {
  const { style } = document.body;
  style.backgroundColor = isDark ? "transparent" : LIGHT_BG;
  style.colorScheme = isDark ? "dark" : "light";
}
