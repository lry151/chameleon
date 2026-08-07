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
          type="primary"
          size="small"
          :loading="busyLaunch"
          @click="handleLaunchSystem"
        >
          启动组
        </n-button>
        <n-popconfirm
          v-if="system"
          :show="showDeletePop"
          :positive-text="'删除'"
          :negative-text="'取消'"
          @positive-click="doDeleteSystem"
          @negative-click="showDeletePop = false"
          @click-outside="showDeletePop = false"
        >
          <template #trigger>
            <n-dropdown
              trigger="click"
              :options="systemMenuOptions"
              :keyboard="true"
              :disabled="showDeletePop"
              @select="handleSystemMenuSelect"
            >
              <n-button size="small" quaternary circle aria-label="系统操作">
                <template #icon>
                  <span aria-hidden="true">⋯</span>
                </template>
              </n-button>
            </n-dropdown>
          </template>
          确定删除系统「{{ system.name }}」？角色将变为未分组。
        </n-popconfirm>
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
        @edit="(r) => $emit('edit', r)"
        @clone="(r) => $emit('clone', r)"
        @deleted="$emit('role-deleted')"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { RoleView, System } from "../types/api";
import { tauri } from "../composables/useTauri";
import { loadAppState } from "../composables/useAppState";
import RoleCard from "./RoleCard.vue";

const props = defineProps<{
  /// null = ungrouped bucket (roles without a system).
  system: System | null;
  roles: RoleView[];
}>();

const emit = defineEmits<{
  (e: "presets", role: RoleView): void;
  (e: "handoff", role: RoleView): void;
  (e: "edit", role: RoleView): void;
  (e: "clone", role: RoleView): void;
  (e: "role-deleted"): void;
  (e: "system-deleted"): void;
}>();

const busyLaunch = ref(false);
const showDeletePop = ref(false);

const systemMenuOptions = computed(() => [
  { label: "编辑", key: "edit" },
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
  if (key === "delete") showDeletePop.value = true;
  else if (key === "delete-with-roles") doDeleteSystemWithRoles();
}

async function doDeleteSystem() {
  if (!props.system) return;
  showDeletePop.value = false;
  await tauri.deleteSystem(props.system.id);
  await loadAppState();
  emit("system-deleted");
}

async function doDeleteSystemWithRoles() {
  if (!props.system) return;
  // 重量确认：使用原生 confirm 做二次兜底（ADR-0010 独立确认步骤）。
  const ok = window.confirm(
    `确定删除系统「${props.system.name}」及其所有角色？此操作不可撤销。`,
  );
  if (!ok) return;
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
  border-style: dashed;
  background: transparent;
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
  font-size: 15px;
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
  gap: 6px;
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
