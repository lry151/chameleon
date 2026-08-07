import { invoke } from "@tauri-apps/api/core";

/// 薄包装：统一 Tauri 命令调用的类型入口。
/// 后续工单扩展更多命令时只在此处加方法，不散落 invoke 字符串。
export const tauri = {
  getUiPreferences: () =>
    invoke<import("../types/api").UiPreferences>("get_ui_preferences"),
  setUiPreferences: (prefs: import("../types/api").UiPreferences) =>
    invoke<void>("set_ui_preferences", { prefs }),

  appMinimize: () => invoke<void>("app_minimize"),
  appMaximize: () => invoke<void>("app_maximize"),
  appHide: () => invoke<void>("app_hide"),
};
