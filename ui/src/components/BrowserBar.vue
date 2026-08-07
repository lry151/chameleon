<template>
  <div class="browser-bar">
    <n-select
      class="browser-select"
      :value="selectedPath"
      :options="browserOptions"
      placeholder="选择浏览器"
      filterable
      @update:value="onSelect"
    />
    <n-button @click="handlePick">选择浏览器</n-button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { appState, loadAppState } from "../composables/useAppState";
import { tauri } from "../composables/useTauri";

const selectedPath = computed(() => appState.value.browser_path ?? null);

const browserOptions = computed(() =>
  appState.value.browser_candidates.map((c) => ({
    label: `${c.name} — ${c.path}`,
    value: c.path,
  })),
);

async function onSelect(path: string) {
  try {
    await tauri.setBrowserPath(path);
    await loadAppState();
  } catch (err) {
    console.error("Failed to set browser path:", err);
  }
}

async function handlePick() {
  try {
    const picked = await tauri.pickBrowserPath();
    if (picked) {
      await tauri.setBrowserPath(picked);
      await loadAppState();
    }
  } catch (err) {
    console.error("Failed to pick browser:", err);
  }
}
</script>

<style scoped>
.browser-bar {
  display: flex;
  align-items: center;
  gap: 8px;
}

.browser-select {
  min-width: 320px;
  flex: 1 1 auto;
}
</style>
