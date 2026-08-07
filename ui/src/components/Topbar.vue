<template>
  <header class="topbar" @mousedown.left="onDrag">
    <!-- Brand -->
    <div class="topbar-brand">
      <span class="topbar-logo" aria-hidden="true">🦎</span>
      <div class="topbar-title">
        <span class="topbar-name" :style="{ color: logoColor }">chameleon</span>
        <span class="topbar-subtitle">Chrome 会话隔离管理</span>
      </div>
    </div>

    <!-- 主操作区 -->
    <n-space :size="8" class="topbar-group">
      <n-button type="primary" size="medium" @mousedown.stop="$emit('newRole')">
        新建角色
      </n-button>
      <n-button size="medium" @mousedown.stop="$emit('newSystem')">
        新建系统
      </n-button>
    </n-space>

    <!-- 更多操作 -->
    <n-dropdown
      trigger="click"
      :options="moreOptions"
      @select="handleMoreSelect"
    >
      <n-button quaternary size="medium" aria-label="更多操作">
        <template #icon>
          <span aria-hidden="true">⋮</span>
        </template>
      </n-button>
    </n-dropdown>

    <!-- 弹簧占位：把窗口控制推到最右 -->
    <span class="topbar-spacer" />

    <!-- 窗口控制 -->
    <div class="topbar-window-controls">
      <button
        class="topbar-ctrl"
        aria-label="最小化"
        @mousedown.stop
        @click="minimize"
      >
        <svg width="10" height="10" viewBox="0 0 10 10">
          <line x1="1" y1="5" x2="9" y2="5" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
      <button
        class="topbar-ctrl"
        aria-label="最大化"
        @mousedown.stop
        @click="maximize"
      >
        <svg width="10" height="10" viewBox="0 0 10 10">
          <rect x="1.5" y="1.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
      <button
        class="topbar-ctrl topbar-ctrl-close"
        aria-label="关闭"
        @mousedown.stop
        @click="hide"
      >
        <svg width="10" height="10" viewBox="0 0 10 10">
          <line x1="1.5" y1="1.5" x2="8.5" y2="8.5" stroke="currentColor" stroke-width="1" />
          <line x1="8.5" y1="1.5" x2="1.5" y2="8.5" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
    </div>

    <!-- 沙箱面板 -->
    <SandboxesPanel v-model:show="showSandboxes" />

    <!-- 快照面板 -->
    <SnapshotsPanel v-model:show="showSnapshots" />
  </header>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMessage, useDialog } from "naive-ui";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tauri } from "../composables/useTauri";
import { loadAppState } from "../composables/useAppState";
import { prefs } from "../composables/usePrefs";
import SandboxesPanel from "./SandboxesPanel.vue";
import SnapshotsPanel from "./SnapshotsPanel.vue";

const emit = defineEmits<{
  (e: "openSettings"): void;
  (e: "newRole"): void;
  (e: "newSystem"): void;
}>();

const showSandboxes = ref(false);
const showSnapshots = ref(false);
const busyLaunchAll = ref(false);
const busyCloseAll = ref(false);
const message = useMessage();
const dialog = useDialog();

/// 顶栏 Logo 色：跟随用户 accent，强化变色龙品牌。
const logoColor = computed(() => prefs.value.accent_color);

function onDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  getCurrentWindow().startDragging();
}

function minimize() {
  tauri.appMinimize();
}
function maximize() {
  tauri.appMaximize();
}
function hide() {
  tauri.appHide();
}

async function handleLaunchAll() {
  busyLaunchAll.value = true;
  try {
    await tauri.launchAll();
    await loadAppState();
    message.success("角色已全部启动");
  } catch (err) {
    message.error("启动角色失败，请检查浏览器路径是否正确");
  } finally {
    busyLaunchAll.value = false;
  }
}

async function handleCloseAll() {
  busyCloseAll.value = true;
  try {
    await tauri.closeAll();
    await loadAppState();
    message.success("角色已全部关闭");
  } catch (err) {
    message.error("关闭角色失败，请稍后重试");
  } finally {
    busyCloseAll.value = false;
  }
}

async function handleCleanup() {
  try {
    await tauri.cleanupTemp();
    await loadAppState();
  } catch (err) {
    message.error("清理临时数据失败");
  }
}

async function handleExport() {
  try {
    await tauri.exportConfig();
    message.success("配置已导出");
  } catch (err) {
    message.error("导出配置失败，请检查写入权限");
  }
}

async function handleImport() {
  try {
    await tauri.importConfig();
    await loadAppState();
    message.success("配置已导入");
  } catch (err: any) {
    message.error(`导入配置失败：${err?.message ?? err}`);
  }
}

const moreOptions = [
  { label: "启动所有", key: "launchAll" },
  { label: "关闭所有", key: "closeAll" },
  { type: "divider" as const, key: "d1" },
  { label: "沙箱", key: "sandbox" },
  { label: "快照", key: "snapshot" },
  { type: "divider" as const, key: "d2" },
  { label: "清理临时数据", key: "cleanup" },
  { type: "divider" as const, key: "d3" },
  { label: "导出配置", key: "export" },
  { label: "导入配置", key: "import" },
  { type: "divider" as const, key: "d4" },
  { label: "设置", key: "settings" },
];

function handleMoreSelect(key: string) {
  switch (key) {
    case "launchAll":
      handleLaunchAll();
      break;
    case "closeAll":
      dialog.warning({
        title: "确认关闭",
        content: "确定关闭全部运行中的角色窗口？",
        positiveText: "关闭全部",
        negativeText: "取消",
        onPositiveClick: handleCloseAll,
      });
      break;
    case "sandbox":
      showSandboxes.value = true;
      break;
    case "snapshot":
      showSnapshots.value = true;
      break;
    case "cleanup":
      dialog.warning({
        title: "确认清理",
        content: "确定清理全部临时数据？",
        positiveText: "清理",
        negativeText: "取消",
        onPositiveClick: handleCleanup,
      });
      break;
    case "export":
      handleExport();
      break;
    case "import":
      dialog.warning({
        title: "确认导入",
        content: "导入将合并外部配置到现有数据，冲突项会被拒绝。确认导入？",
        positiveText: "导入",
        negativeText: "取消",
        onPositiveClick: handleImport,
      });
      break;
    case "settings":
      emit("openSettings");
      break;
  }
}
</script>

<style scoped>
.topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--topbar-bg);
  user-select: none;
  border-bottom: 1px solid rgba(128, 128, 128, 0.2);
}

.topbar-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-right: 8px;
}

.topbar-logo {
  font-size: 22px;
  line-height: 1;
}

.topbar-title {
  display: flex;
  flex-direction: column;
  line-height: 1.4;
}

.topbar-name {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

.topbar-subtitle {
  font-size: 11px;
  opacity: 0.65;
}
.topbar-group {
  flex-shrink: 0;
}

.topbar-spacer {
  flex: 1 1 auto;
}

.topbar-window-controls {
  display: flex;
  flex-shrink: 0;
  margin-left: 12px;
}

.topbar-ctrl {
  width: 46px;
  height: 32px;
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background-color 0.1s;
}
.topbar-ctrl:hover {
  background: rgba(255, 255, 255, 0.1);
}
.topbar-ctrl:active {
  background: rgba(255, 255, 255, 0.05);
}
.topbar-ctrl-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
