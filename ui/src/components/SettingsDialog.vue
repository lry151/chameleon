<template>
  <n-modal
    :show="show"
    :mask-closable="true"
    preset="card"
    title="设置"
    :bordered="false"
    :style="{ maxWidth: '480px' }"
    @update:show="onUpdateShow"
    @after-leave="onAfterLeave"
  >
    <div class="settings-body">
      <!-- 段 1：主题 -->
      <div class="settings-section">
        <n-text class="settings-label">主题</n-text>
        <n-radio-group :value="draft.theme" @update:value="setTheme">
          <n-space :size="12">
            <n-radio value="Dark">深色</n-radio>
            <n-radio value="Light">浅色</n-radio>
            <n-radio value="System">跟随系统</n-radio>
          </n-space>
        </n-radio-group>
      </div>

      <!-- 段 2：透明度 -->
      <div class="settings-section">
        <n-text class="settings-label">面板透明度</n-text>
        <div class="settings-row">
          <n-slider
            :value="draft.panel_opacity"
            :min="0.5"
            :max="1.0"
            :step="0.05"
            class="settings-slider"
            @update:value="setOpacity"
          />
          <n-text class="settings-value">{{ formatOpacity(draft.panel_opacity) }}</n-text>
        </div>
      </div>

      <!-- 段 3：Accent 颜色 -->
      <div class="settings-section">
        <n-text class="settings-label">主题色</n-text>
        <n-space :size="12">
          <button
            v-for="c in accentPalette"
            :key="c"
            class="accent-dot"
            :class="{ 'accent-dot--active': draft.accent_color === c }"
            :style="{ backgroundColor: c }"
            :aria-label="`主题色 ${c}`"
            @click="setAccent(c)"
          />
        </n-space>
      </div>

      <!-- 段 4：浏览器 -->
      <div class="settings-section">
        <n-text class="settings-label">浏览器</n-text>
        <div class="settings-row">
          <n-select
            :value="browserPath"
            :options="browserOptions"
            placeholder="选择浏览器"
            filterable
            class="settings-browser-select"
            @update:value="onSelectBrowser"
          />
          <n-button @click="handlePickBrowser">选择文件</n-button>
        </div>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { appState, loadAppState } from "../composables/useAppState";
import { tauri } from "../composables/useTauri";
import { useMessage } from "naive-ui";
import { prefs, savePrefs } from "../composables/usePrefs";
import type { ThemeMode, UiPreferences } from "../types/api";

const props = defineProps<{
  show: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();
const message = useMessage();

const browserPath = computed(() => appState.value.browser_path ?? null);
const browserOptions = computed(() =>
  appState.value.browser_candidates.map((c) => ({
    label: `${c.name} — ${c.path}`,
    value: c.path,
  })),
);

async function onSelectBrowser(path: string) {
  try {
    await tauri.setBrowserPath(path);
    await loadAppState();
    message.success("浏览器已切换");
  } catch (err) {
    message.error("切换浏览器失败");
  }
}

async function handlePickBrowser() {
  try {
    const picked = await tauri.pickBrowserPath();
    if (picked) {
      await tauri.setBrowserPath(picked);
      await loadAppState();
      message.success("浏览器已切换");
    }
  } catch (err) {
    message.error("选择浏览器失败");
  }
}

const ACCENT_PALETTE = [
  "#1abc9c", // 默认 teal
  "#0078D4", // Fluent blue
  "#e74c3c", // red
  "#f39c12", // orange
  "#9b59b6", // purple
  "#2ecc71", // green
] as const;

const accentPalette: string[] = [...ACCENT_PALETTE];

/// 本地草稿：dialog 打开时从 prefs 拷贝一份，拖动过程中只改草稿 + 即时应用，
/// 关闭时一次性 save。
const draft = reactive<UiPreferences>({ ...prefs.value });

/// dialog 打开时同步草稿。
watch(
  () => props.show,
  (visible) => {
    if (visible) Object.assign(draft, prefs.value);
  },
);

function setTheme(mode: ThemeMode) {
  draft.theme = mode;
  // 立即应用到全局 prefs（驱动 useTheme 响应）。
  prefs.value = { ...draft };
}

function setOpacity(v: number) {
  draft.panel_opacity = v;
  prefs.value = { ...draft };
}

function setAccent(color: string) {
  draft.accent_color = color;
  prefs.value = { ...draft };
}

function formatOpacity(v: number): string {
  return `${Math.round(v * 100)}%`;
}

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

/// modal 完全关闭后持久化一次。
async function onAfterLeave() {
  await savePrefs({ ...draft });
}
</script>

<style scoped>
.settings-body {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-label {
  font-weight: 600;
  font-size: 14px;
}

.settings-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.settings-slider {
  flex: 1 1 auto;
}

.settings-value {
  flex-shrink: 0;
  min-width: 40px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-size: 13px;
}

.accent-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
  transition: border-color 0.15s;
}
.accent-dot:hover {
  border-color: rgba(128, 128, 128, 0.4);
}
.accent-dot--active {
  border-color: currentColor;
  box-shadow: 0 0 0 2px rgba(128, 128, 128, 0.25);
}
</style>
