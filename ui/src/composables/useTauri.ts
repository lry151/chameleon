import { invoke } from "@tauri-apps/api/core";
import type {
  AppStateView,
  BatchResult,
  HandoffMode,
  QuickLinkLogin,
  Role,
  System,
  UiPreferences,
} from "../types/api";

/// 薄包装：统一 Tauri 命令调用的类型入口。
/// 后续工单扩展更多命令时只在此处加方法，不散落 invoke 字符串。
export const tauri = {
  // —— 状态 ——
  getState: () => invoke<AppStateView>("get_state"),

  // —— 角色管理 ——
  createRole: (name: string, color: string) =>
    invoke<Role>("create_role", { name, color }),
  updateRole: (role: Role) =>
    invoke<void>("update_role", { role }),
  deleteRole: (id: string) =>
    invoke<void>("delete_role", { id }),

  // —— 系统管理 ——
  createSystem: (name: string) =>
    invoke<System>("create_system", { name }),
  updateSystem: (system: System) =>
    invoke<void>("update_system", { system }),
  deleteSystem: (id: string) =>
    invoke<void>("delete_system", { id }),
  deleteSystemWithRoles: (id: string) =>
    invoke<number>("delete_system_with_roles", { id }),

  // —— 启动 / 关闭 ——
  launchRole: (id: string) =>
    invoke<void>("launch_role_cmd", { id }),
  closeRole: (id: string) =>
    invoke<void>("close_role_cmd", { id }),
  launchAll: () =>
    invoke<BatchResult>("launch_all"),
  closeAll: () =>
    invoke<BatchResult>("close_all"),
  launchSystem: (systemId: string) =>
    invoke<BatchResult>("launch_system", { systemId }),
  closeSystem: (systemId: string) =>
    invoke<BatchResult>("close_system", { systemId }),
  // —— 常用 URL 预设（角色级） ——
  addQuickLink: (
    roleId: string,
    name: string,
    url: string,
    autoOpen: boolean,
    login: QuickLinkLogin | null,
  ) =>
    invoke<void>("add_quick_link", {
      roleId,
      name,
      url,
      autoOpen,
      login,
    }),
  editQuickLink: (
    roleId: string,
    oldName: string,
    name: string,
    url: string,
    autoOpen: boolean,
    login: QuickLinkLogin | null,
  ) =>
    invoke<void>("edit_quick_link", {
      roleId,
      oldName,
      name,
      url,
      autoOpen,
      login,
    }),
  removeQuickLink: (roleId: string, name: string) =>
    invoke<void>("remove_quick_link", { roleId, name }),
  openQuickLink: (roleId: string, name: string) =>
    invoke<string>("open_quick_link", { roleId, name }),

  // —— 常用 URL 预设（系统级） ——
  addSystemQuickLink: (
    systemId: string,
    name: string,
    url: string,
    autoOpen: boolean,
  ) =>
    invoke<void>("add_system_quick_link", {
      systemId,
      name,
      url,
      autoOpen,
    }),
  editSystemQuickLink: (
    systemId: string,
    oldName: string,
    name: string,
    url: string,
    autoOpen: boolean,
  ) =>
    invoke<void>("edit_system_quick_link", {
      systemId,
      oldName,
      name,
      url,
      autoOpen,
    }),
  removeSystemQuickLink: (systemId: string, name: string) =>
    invoke<void>("remove_system_quick_link", { systemId, name }),

  // —— 接力 ——
  handoff: (sourceId: string, targetId: string, mode: HandoffMode) =>
    invoke<string>("handoff_cmd", { sourceId, targetId, mode }),

  // —— 沙箱 ——
  launchSandbox: () =>
    invoke<{ id: string; dir: string }>("launch_sandbox"),
  closeSandbox: (id: string) =>
    invoke<void>("close_sandbox", { id }),
  cleanupTemp: () =>
    invoke<number>("cleanup_temp"),

  // —— 快照 ——
  saveSnapshot: (name: string) =>
    invoke<void>("save_snapshot", { name }),
  restoreSnapshot: (name: string) =>
    invoke<void>("restore_snapshot", { name }),
  deleteSnapshot: (name: string) =>
    invoke<void>("delete_snapshot", { name }),

  // —— 浏览器 ——
  pickBrowserPath: () =>
    invoke<string | null>("pick_browser_path"),
  setBrowserPath: (path: string) =>
    invoke<void>("set_browser_path", { path }),

  // —— 配置导出 / 导入 ——
  exportConfig: () =>
    invoke<string | null>("export_config_cmd"),
  importConfig: () =>
    invoke<number>("import_config_cmd"),

  // —— 偏好 ——
  getUiPreferences: () =>
    invoke<UiPreferences>("get_ui_preferences"),
  setUiPreferences: (prefs: UiPreferences) =>
    invoke<void>("set_ui_preferences", { prefs }),

  // —— 窗口 ——
  appMinimize: () => invoke<void>("app_minimize"),
  appMaximize: () => invoke<void>("app_maximize"),
  appHide: () => invoke<void>("app_hide"),

  // —— 日志 ——
  openLogFolder: () => invoke<void>("open_log_folder"),
};
