import { ref } from "vue";
import type { AppStateView, RoleView, System } from "../types/api";
import { tauri } from "./useTauri";

/// 全局应用状态（角色 / 系统 / 沙箱 / 快照等）。
/// 单例 ref，启动时由 App 加载一次，跨组件共享。
export const appState = ref<AppStateView>({
  roles: [],
  systems: [],
  sandboxes: [],
  snapshots: [],
  browser_path: null,
  browser_candidates: [],
  data_root: "",
});

export async function loadAppState(): Promise<void> {
  try {
    appState.value = await tauri.getState();
  } catch {
    // 启动时后端未就绪 → 保持空，不抛错。
  }
}

/// 按 system_id 分组角色。返回 Map<systemId | "__ungrouped__", RoleView[]>。
export function groupRolesBySystem(
  roles: RoleView[],
): Map<string, RoleView[]> {
  const map = new Map<string, RoleView[]>();
  for (const r of roles) {
    const key = r.system_id ?? "__ungrouped__";
    const arr = map.get(key);
    if (arr) arr.push(r);
    else map.set(key, [r]);
  }
  return map;
}

/// 根据系统 ID 取得该系统下的角色列表。
export function rolesForSystem(systemId: string): RoleView[] {
  return appState.value.roles.filter((r) => r.system_id === systemId);
}

/// 取得未分组的角色列表。
export function ungroupedRoles(): RoleView[] {
  return appState.value.roles.filter((r) => r.system_id === null);
}

/// 按系统分箱：返回 [{ system, roles }] 列表，最后一条为 ungrouped（system=null）。
export function systemBuckets(): { system: System | null; roles: RoleView[] }[] {
  const result: { system: System | null; roles: RoleView[] }[] = [];
  for (const sys of appState.value.systems) {
    result.push({ system: sys, roles: rolesForSystem(sys.id) });
  }
  const ungrouped = ungroupedRoles();
  if (ungrouped.length > 0) {
    result.push({ system: null, roles: ungrouped });
  }
  return result;
}
