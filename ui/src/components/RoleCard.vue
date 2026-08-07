<template>
  <n-card class="role-card" :class="{ 'role-card--running': role.running }">
    <!-- 头部：标识 + 状态 -->
    <template #header>
      <div class="role-header">
        <span
          class="role-swatch"
          :style="{ backgroundColor: role.color }"
          aria-hidden="true"
        />
        <span class="role-name" :title="role.name">{{ role.name }}</span>
        <n-tag
          v-if="role.running"
          :bordered="false"
          size="small"
          round
          :style="{ backgroundColor: role.color, color: '#fff' }"
        >
          运行中
        </n-tag>
        <span class="role-port">:{{ role.cdp_port }}</span>
      </div>
    </template>

    <!-- 中部：preset chips -->
    <div class="role-links">
      <template v-if="role.quick_links.length > 0">
        <n-button
          v-for="link in role.quick_links"
          :key="link.name"
          size="tiny"
          secondary
          class="role-link-chip"
          @click="openLink(link.url)"
        >
          {{ link.name || link.url }}
        </n-button>
      </template>
      <n-text v-else depth="3" class="role-links-empty">
        暂无预设
      </n-text>
    </div>

    <!-- 底部：actions -->
    <template #action>
      <div class="role-actions">
        <n-space :size="8">
          <n-button
            v-if="!role.running"
            type="primary"
            size="small"
            :loading="busy"
            @click="handleLaunch"
          >
            启动
          </n-button>
          <n-button
            v-else
            size="small"
            :loading="busy"
            @click="handleClose"
          >
            关闭
          </n-button>
          <n-button size="small" secondary @click="$emit('presets', role)">
            预设
          </n-button>
          <n-button size="small" secondary @click="$emit('handoff', role)">
            接力
          </n-button>
        </n-space>

        <!-- 更多操作：popconfirm（删除确认）+ dropdown（编辑/克隆） -->
        <n-popconfirm
          :show="showDeletePop"
          :positive-text="'删除'"
          :negative-text="'取消'"
          @positive-click="doDelete"
          @negative-click="showDeletePop = false"
          @click-outside="showDeletePop = false"
        >
          <template #trigger>
            <n-dropdown
              trigger="click"
              :options="menuOptions"
              :keyboard="true"
              :disabled="showDeletePop"
              @select="handleMenuSelect"
            >
              <n-button
                size="small"
                quaternary
                circle
                aria-label="更多操作"
              >
                <template #icon>
                  <span aria-hidden="true">⋯</span>
                </template>
              </n-button>
            </n-dropdown>
          </template>
          确定删除角色「{{ role.name }}」？
        </n-popconfirm>
      </div>
    </template>
  </n-card>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { RoleView } from "../types/api";
import { tauri } from "../composables/useTauri";
import { loadAppState } from "../composables/useAppState";

const props = defineProps<{
  role: RoleView;
}>();

const emit = defineEmits<{
  (e: "presets", role: RoleView): void;
  (e: "handoff", role: RoleView): void;
  (e: "edit", role: RoleView): void;
  (e: "clone", role: RoleView): void;
  (e: "deleted"): void;
}>();

const busy = ref(false);
const showDeletePop = ref(false);

const menuOptions = computed(() => [
  { label: "编辑", key: "edit" },
  { label: "克隆", key: "clone" },
  { type: "divider", key: "d1" },
  { label: "删除", key: "delete" },
]);

async function handleLaunch() {
  busy.value = true;
  try {
    await tauri.launchRole(props.role.id);
    await loadAppState();
  } finally {
    busy.value = false;
  }
}

async function handleClose() {
  busy.value = true;
  try {
    await tauri.closeRole(props.role.id);
    await loadAppState();
  } finally {
    busy.value = false;
  }
}

function handleMenuSelect(key: string) {
  if (key === "edit") emit("edit", props.role);
  else if (key === "clone") emit("clone", props.role);
  else if (key === "delete") showDeletePop.value = true;
}

async function doDelete() {
  showDeletePop.value = false;
  busy.value = true;
  try {
    await tauri.deleteRole(props.role.id);
    await loadAppState();
    emit("deleted");
  } finally {
    busy.value = false;
  }
}

function openLink(url: string) {
  window.open(url, "_blank", "noopener");
}
</script>

<style scoped>
.role-header {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.role-swatch {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.role-name {
  font-weight: 600;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1 1 auto;
  min-width: 0;
}

.role-port {
  font-size: 12px;
  opacity: 0.6;
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}

.role-links {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  min-height: 24px;
}

.role-link-chip {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.role-links-empty {
  font-size: 12px;
}

.role-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
</style>
