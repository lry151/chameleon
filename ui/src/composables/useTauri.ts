import { invoke } from "@tauri-apps/api/core";
import type {
  AppStateView,
  BatchResult,
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

  // —— 偏好 ——
  getUiPreferences: () =>
    invoke<UiPreferences>("get_ui_preferences"),
  setUiPreferences: (prefs: UiPreferences) =>
    invoke<void>("set_ui_preferences", { prefs }),

  // —— 窗口 ——
  appMinimize: () => invoke<void>("app_minimize"),
  appMaximize: () => invoke<void>("app_maximize"),
  appHide: () => invoke<void>("app_hide"),
};
