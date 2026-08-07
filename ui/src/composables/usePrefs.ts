import { ref } from "vue";
import { tauri } from "./useTauri";
import { DEFAULT_PREFS, type UiPreferences } from "../types/api";

/// 全局 UI 偏好（主题 / 透明度 / 主色）。
/// 单例 ref，启动时由 App 加载一次，跨组件共享。
export const prefs = ref<UiPreferences>({ ...DEFAULT_PREFS });

export async function loadPrefs(): Promise<void> {
  try {
    const p = await tauri.getUiPreferences();
    prefs.value = { ...DEFAULT_PREFS, ...p };
  } catch {
    // 旧版本或读取失败 → 保持默认，不抛错。
  }
}

export async function savePrefs(next: UiPreferences): Promise<void> {
  prefs.value = { ...next };
  await tauri.setUiPreferences(next);
}
