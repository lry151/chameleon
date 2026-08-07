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
      <n-button type="primary" size="medium" @mousedown.stop>新建角色</n-button>
      <n-button size="medium" @mousedown.stop>新建系统</n-button>
    </n-space>

    <!-- 批量区 -->
    <n-space :size="8" class="topbar-group">
      <n-button size="medium" @mousedown.stop>启动所有</n-button>
      <n-button size="medium" @mousedown.stop>关闭所有</n-button>
      <n-button size="medium" @mousedown.stop>沙箱</n-button>
      <n-button size="medium" @mousedown.stop>清理</n-button>
    </n-space>

    <!-- 工具区 -->
    <n-space :size="8" class="topbar-group topbar-tools">
      <n-button size="medium" @mousedown.stop>导出</n-button>
      <n-button size="medium" @mousedown.stop>导入</n-button>
      <n-button quaternary circle size="medium" @mousedown.stop @click="$emit('openSettings')" aria-label="设置">
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
  </header>
</template>

<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tauri } from "../composables/useTauri";

defineEmits<{
  (e: "openSettings"): void;
}>();

function onDrag(e: MouseEvent) {
  // 仅左键触发；按钮已通过 @mousedown.stop 阻止冒泡。
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
</script>

<style scoped>
.topbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 12px;
  /* 深色：轻微暗 tint，避免内容直接贴在 Mica 上难以聚焦；
     浅色：实色浅色（由 --topbar-bg 控制，随 isDark + panel_opacity 变化）。 */
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
