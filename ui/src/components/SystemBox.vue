<template>
  <div
    class="system-box"
    :class="{ 'system-box--ungrouped': !system }"
  >
    <!-- 头部：标识 + actions -->
    <div class="sys-head">
      <div class="sys-title">
        <span class="sys-name">{{ system ? system.name : "未分组" }}</span>
        <n-text depth="3" class="sys-count">
          ({{ roles.length }} 个角色)
        </n-text>
      </div>
      <n-space :size="8">
        <n-button
          v-if="system"
          type="primary"
          size="small"
          :loading="busyLaunch"
          @click="handleLaunchSystem"
        >
          启动组
        </n-button>
        <n-dropdown
          v-if="system"
          trigger="click"
          :options="systemMenuOptions"
          :keyboard="true"
          @select="handleSystemMenuSelect"
        >
          <n-button size="small" quaternary circle aria-label="系统操作">
            <template #icon>
              <span aria-hidden="true">⋯</span>
            </template>
          </n-button>
        </n-dropdown>
      </n-space>
    </div>

    <!-- 中部：sys-links -->
    <div v-if="system && system.quick_links.length > 0" class="sys-links">
      <n-button
        v-for="link in system.quick_links"
        :key="link.name"
        size="tiny"
        secondary
        class="sys-link-chip"
        @click="openLink(link.url)"
      >
        {{ link.name || link.url }}
      </n-button>
    </div>

    <!-- 底部：role-grid -->
    <div class="role-grid">
      <RoleCard
        v-for="role in roles"
        :key="role.id"
        :role="role"
        @presets="(r) => $emit('presets', r)"
        @handoff="(r) => $emit('handoff', r)"
        @edit="(r) => $emit('edit-role', r)"
        @clone="(r) => $emit('clone-role', r)"
        @deleted="$emit('role-deleted')"
      />
    </div>

    <!-- 删除系统 + 角色 确认 dialog -->
    <DeleteConfirm
      v-model:show="showDeleteWithRoles"
      :title="'删除系统 + 角色'"
      :message="`确定删除系统「${system?.name}」及其所有角色？此操作不可撤销。`"
      confirm-text="确认删除"
      @confirm="doDeleteSystemWithRoles"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { RoleView, System } from "../types/api";
import { tauri } from "../composables/useTauri";
import { loadAppState } from "../composables/useAppState";
import RoleCard from "./RoleCard.vue";
import DeleteConfirm from "./DeleteConfirm.vue";

const props = defineProps<{
  /// null = ungrouped bucket (roles without a system).
  system: System | null;
  roles: RoleView[];
}>();

const emit = defineEmits<{
  (e: "presets", role: RoleView): void;
  (e: "handoff", role: RoleView): void;
  (e: "edit-role", role: RoleView): void;
  (e: "clone-role", role: RoleView): void;
  (e: "edit-system", system: System): void;
  (e: "presets-system", systemId: string): void;
  (e: "role-deleted"): void;
  (e: "system-deleted"): void;
}>();

const busyLaunch = ref(false);
const showDeleteWithRoles = ref(false);

const systemMenuOptions = computed(() => [
  { label: "编辑", key: "edit" },
  { label: "管理预设", key: "presets" },
  { type: "divider", key: "d1" },
  { label: "删除系统", key: "delete" },
  { label: "删除系统 + 角色", key: "delete-with-roles", props: { type: "error" as const } },
]);

async function handleLaunchSystem() {
  if (!props.system) return;
  busyLaunch.value = true;
  try {
    await tauri.launchSystem(props.system.id);
    await loadAppState();
  } finally {
    busyLaunch.value = false;
  }
}

function handleSystemMenuSelect(key: string) {
  if (!props.system) return;
  if (key === "edit") emit("edit-system", props.system);
  else if (key === "presets") emit("presets-system", props.system.id);
  else if (key === "delete") doDeleteSystem();
  else if (key === "delete-with-roles") showDeleteWithRoles.value = true;
}

async function doDeleteSystem() {
  if (!props.system) return;
  await tauri.deleteSystem(props.system.id);
  await loadAppState();
  emit("system-deleted");
}

async function doDeleteSystemWithRoles() {
  if (!props.system) return;
  await tauri.deleteSystemWithRoles(props.system.id);
  await loadAppState();
  emit("system-deleted");
}

function openLink(url: string) {
  window.open(url, "_blank", "noopener");
}
</script>

<style scoped>
.system-box {
  border: 1px solid rgba(128, 128, 128, 0.2);
  border-radius: 8px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.system-box--ungrouped {
  background: rgba(128, 128, 128, 0.04);
}

.sys-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.sys-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}

.sys-name {
  font-weight: 600;
  font-size: 16px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sys-count {
  font-size: 12px;
  flex-shrink: 0;
}

.sys-links {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.sys-link-chip {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.role-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
}
</style>
