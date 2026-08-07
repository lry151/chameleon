// Rust 端类型镜像。与 crates/core/src/model.rs 保持同步。

/// 主题模式（对应 Rust `ThemeMode`）。
/// serde 默认 PascalCase 序列化，无 rename_all。
export type ThemeMode = "Dark" | "Light" | "System";

export interface UiPreferences {
  theme: ThemeMode;
  /// 面板透明度 0.5–1.0。
  panel_opacity: number;
  /// Accent 颜色，十六进制如 "#1abc9c"。
  accent_color: string;
}

export const DEFAULT_PREFS: UiPreferences = {
  theme: "Dark",
  panel_opacity: 0.72,
  accent_color: "#1abc9c",
};
