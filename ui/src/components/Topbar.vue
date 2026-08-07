<template>
  <header class="topbar" @mousedown.left="onDrag">
    <!-- Brand -->
    <div class="topbar-brand">
      <span class="topbar-logo" aria-hidden="true">🦎</span>
      <div class="topbar-title">
        <span class="topbar-name">chameleon</span>
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

    <!-- 批量区 -->
    <n-space :size="8" class="topbar-group">
      <n-button
        size="medium"
        :loading="busyLaunchAll"
        @mousedown.stop="handleLaunchAll"
      >
        启动所有
      </n-button>
      <n-button
        size="medium"
        :loading="busyCloseAll"
        @mousedown.stop="handleCloseAll"
      >
        关闭所有
      </n-button>
      <n-button size="medium" @mousedown.stop="showSandboxes = true">
        沙箱
      </n-button>
      <n-button size="medium" @mousedown.stop="showSnapshots = true">
        快照
      </n-button>
      <n-popconfirm
        :show="showCleanupPop"
        positive-text="清理"
        negative-text="取消"
        @positive-click="handleCleanup"
        @negative-click="showCleanupPop = false"
        @click-outside="showCleanupPop = false"
      >
        <template #trigger>
          <n-button size="medium" @mousedown.stop="showCleanupPop = true">
            清理
          </n-button>
        </template>
        确定清理全部临时数据？
      </n-popconfirm>
    </n-space>

    <!-- 工具区 -->
    <n-space :size="8" class="topbar-group topbar-tools">
      <n-button size="medium" @mousedown.stop="handleExport">导出</n-button>
      <n-button size="medium" @mousedown.stop="handleImport">导入</n-button>
      <n-button
        quaternary
        circle
        size="medium"
        @mousedown.stop
        @click="$emit('openSettings')"
        aria-label="设置"
      >
        <template #icon>
          <span aria-hidden="true">⚙</span>
        </template>
      </n-button>
    </n-space>

    <!-- 弹簧占位：把窗口控制推到最右 -->
    <span class="topbar-spacer" />

    <!-- 窗口控制 -->
    <div class="topbar-window-controls">
      <button
        class="topbar-ctrl"
        aria-label="最小化"
        @mousedown.stop
        @click="minimize"
      >─</button>
      <button
        class="topbar-ctrl"
        aria-label="最大化"
        @mousedown.stop
        @click="maximize"
      >□</button>
      <button
        class="topbar-ctrl topbar-ctrl-close"
        aria-label="关闭"
        @mousedown.stop
        @click="hide"
      >×</button>
    </div>

    <!-- 沙箱面板 -->
    <SandboxesPanel v-model:show="showSandboxes" />

    <!-- 快照面板 -->
    <SnapshotsPanel v-model:show="showSnapshots" />
  </header>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tauri } from "../composables/useTauri";
import { loadAppState } from "../composables/useAppState";
import SandboxesPanel from "./SandboxesPanel.vue";
import SnapshotsPanel from "./SnapshotsPanel.vue";

defineEmits<{
  (e: "openSettings"): void;
  (e: "newRole"): void;
  (e: "newSystem"): void;
}>();

const showSandboxes = ref(false);
const showSnapshots = ref(false);
const showCleanupPop = ref(false);
const busyLaunchAll = ref(false);
const busyCloseAll = ref(false);

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
  } catch (err) {
    console.error("Failed to launch all:", err);
  } finally {
    busyLaunchAll.value = false;
  }
}

async function handleCloseAll() {
  busyCloseAll.value = true;
  try {
    await tauri.closeAll();
    await loadAppState();
  } catch (err) {
    console.error("Failed to close all:", err);
  } finally {
    busyCloseAll.value = false;
  }
}

async function handleCleanup() {
  showCleanupPop.value = false;
  try {
    await tauri.cleanupTemp();
    await loadAppState();
  } catch (err) {
    console.error("Failed to cleanup:", err);
  }
}

async function handleExport() {
  try {
    await tauri.exportConfig();
  } catch (err) {
    console.error("Failed to export:", err);
  }
}

async function handleImport() {
  try {
    await tauri.importConfig();
    await loadAppState();
  } catch (err) {
    console.error("Failed to import:", err);
  }
}
</script>

<style scoped>
.topbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 12px;
  background: var(--topbar-bg);
  user-select: none;
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
}

.topbar-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-right: 8px;
  border-right: 1px solid rgba(128, 128, 128, 0.2);
  margin-right: 4px;
}

.topbar-logo {
  font-size: 22px;
  line-height: 1;
}

.topbar-title {
  display: flex;
  flex-direction: column;
  line-height: 1.1;
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

.topbar-tools {
  margin-left: 4px;
}

.topbar-spacer {
  flex: 1 1 auto;
}

.topbar-window-controls {
  display: flex;
  flex-shrink: 0;
  margin-left: 4px;
}

.topbar-ctrl {
  width: 40px;
  height: 28px;
  border: none;
  background: transparent;
  color: inherit;
  font-size: 12px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.topbar-ctrl:hover {
  background: rgba(128, 128, 128, 0.2);
}
.topbar-ctrl-close:hover {
  background: #e81123;
  color: #fff;
}
</style>
