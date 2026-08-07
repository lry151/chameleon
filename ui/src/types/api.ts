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

/// 常用 URL 预设（与 Rust `QuickLink` 对齐）。
export interface QuickLink {
  name: string;
  url: string;
  auto_open: boolean;
  login: QuickLinkLogin | null;
}

export interface QuickLinkLogin {
  username: string;
  password: string;
  username_selector: string | null;
  password_selector: string | null;
}

/// 登录辅助（不存密码）。
export interface LoginConfig {
  login_url: string;
  username: string;
  username_selector: string | null;
  password_selector: string | null;
}

export interface WindowRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/// 角色（与 Rust `Role` 对齐）。
export interface Role {
  id: string;
  name: string;
  color: string;
  profile_dir: string;
  cdp_port: number;
  quick_links: QuickLink[];
  window_rect: WindowRect | null;
  system_id: string | null;
  login: LoginConfig | null;
}

/// 角色视图 = Role 字段平铺 + running 标记（对应 Rust `RoleView` #[serde(flatten)]）。
export interface RoleView extends Role {
  running: boolean;
}

/// 系统（与 Rust `System` 对齐）。
export interface System {
  id: string;
  name: string;
  quick_links: QuickLink[];
}

export interface BatchResult {
  succeeded: string[];
  failed: { id: string; error: string }[];
}

export interface SandboxInfo {
  id: string;
  dir: string;
}

export interface BrowserCandidate {
  name: string;
  path: string;
}

/// get_state 返回的完整应用状态。
export interface AppStateView {
  roles: RoleView[];
  systems: System[];
  sandboxes: SandboxInfo[];
  snapshots: string[];
  browser_path: string | null;
  browser_candidates: BrowserCandidate[];
  data_root: string;
}
