<template>
  <n-config-provider
    :theme="naiveTheme"
    :theme-overrides="fluentOverride"
  >
    <n-loading-bar-provider>
      <n-dialog-provider>
        <n-message-provider>
          <ExitWatcher />
          <div class="app-shell">
            <Topbar
              @open-settings="showSettings = true"
              @new-role="openNewRole"
              @new-system="openNewSystem"
            />
            <main class="app-main">
              <MainView
                @edit-role="openEditRole"
                @clone-role="openCloneRole"
                @edit-system="openEditSystem"
              />
            </main>
          </div>
          <SettingsDialog v-model:show="showSettings" />
          <RoleDialog
            v-model:show="showRole"
            :role="roleDialogTarget"
            :clone="roleDialogClone"
            @saved="refresh"
            @manage-links="openRoleLinks"
          />
          <SystemDialog
            v-model:show="showSystem"
            :system-id="systemDialogId"
            :system-name="systemDialogName"
            @saved="refresh"
            @manage-links="openSystemLinks"
          />
          <LinksDialog
            v-model:show="showLinks"
            :owner-id="linksOwnerId"
            :owner-kind="linksOwnerKind"
          />
        </n-message-provider>
      </n-dialog-provider>
    </n-loading-bar-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { RoleView, System } from "./types/api";
import Topbar from "./components/Topbar.vue";
import MainView from "./views/MainView.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import RoleDialog from "./components/RoleDialog.vue";
import SystemDialog from "./components/SystemDialog.vue";
import LinksDialog from "./components/LinksDialog.vue";
import ExitWatcher from "./components/ExitWatcher.vue";
import { useTheme } from "./composables/useTheme";
import { loadPrefs } from "./composables/usePrefs";
import { loadAppState } from "./composables/useAppState";

const { naiveTheme, fluentOverride } = useTheme();

// —— 全局 dialog 状态 ——
const showSettings = ref(false);

// RoleDialog
const showRole = ref(false);
const roleDialogTarget = ref<RoleView | null>(null);
const roleDialogClone = ref(false);

function openNewRole() {
  roleDialogTarget.value = null;
  roleDialogClone.value = false;
  showRole.value = true;
}
function openEditRole(role: RoleView) {
  roleDialogTarget.value = role;
  roleDialogClone.value = false;
  showRole.value = true;
}
function openCloneRole(role: RoleView) {
  roleDialogTarget.value = role;
  roleDialogClone.value = true;
  showRole.value = true;
}

// SystemDialog
const showSystem = ref(false);
const systemDialogId = ref<string | null>(null);
const systemDialogName = ref("");

function openNewSystem() {
  systemDialogId.value = null;
  systemDialogName.value = "";
  showSystem.value = true;
}
function openEditSystem(system: System) {
  systemDialogId.value = system.id;
  systemDialogName.value = system.name;
  showSystem.value = true;
}

// LinksDialog
const showLinks = ref(false);
const linksOwnerId = ref("");
const linksOwnerKind = ref<"role" | "system">("role");

function openRoleLinks(roleId: string) {
  linksOwnerId.value = roleId;
  linksOwnerKind.value = "role";
  showLinks.value = true;
}
function openSystemLinks(systemId: string) {
  linksOwnerId.value = systemId;
  linksOwnerKind.value = "system";
  showLinks.value = true;
}

async function refresh() {
  await loadAppState();
}

// 启动时加载偏好（主题 / accent / opacity）。
loadPrefs();
</script>

<style>
/* 全局重置：窗口无默认 margin，body 背景由 hybrid 策略控制。 */
html,
body,
#app {
  margin: 0;
  padding: 0;
  height: 100%;
}
body {
  overflow: hidden;
}

/* View Transitions — 对话框弹跳入场 */
@keyframes dialogIn {
  from {
    opacity: 0;
    transform: scale(0.85);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
@keyframes dialogOut {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.95);
  }
}

/* Dropdown 菜单项错开入场 */
.n-dropdown-menu {
  animation: dropdownIn 0.2s ease-out;
}
@keyframes dropdownIn {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.n-dropdown-option-body {
  animation: dropdownItemIn 0.25s ease-out backwards;
}
.n-dropdown-option-body:nth-child(1) { animation-delay: 0.02s; }
.n-dropdown-option-body:nth-child(2) { animation-delay: 0.04s; }
.n-dropdown-option-body:nth-child(3) { animation-delay: 0.06s; }
.n-dropdown-option-body:nth-child(4) { animation-delay: 0.08s; }
.n-dropdown-option-body:nth-child(5) { animation-delay: 0.10s; }
.n-dropdown-option-body:nth-child(6) { animation-delay: 0.12s; }
.n-dropdown-option-body:nth-child(7) { animation-delay: 0.14s; }
.n-dropdown-option-body:nth-child(8) { animation-delay: 0.16s; }
.n-dropdown-option-body:nth-child(9) { animation-delay: 0.18s; }
.n-dropdown-option-body:nth-child(10) { animation-delay: 0.20s; }
@keyframes dropdownItemIn {
  from {
    opacity: 0;
    transform: translateX(-6px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}
</style>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: transparent;
}

.app-main {
  flex: 1 1 auto;
  overflow: auto;
  padding: 24px;
  background: var(--main-bg);
}
</style>
