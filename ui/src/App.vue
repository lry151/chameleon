<template>
  <n-config-provider
    :theme="naiveTheme"
    :theme-overrides="fluentOverride"
  >
    <n-loading-bar-provider>
      <n-dialog-provider>
        <n-message-provider>
          <div class="app-shell">
            <Topbar />
            <main class="app-main">
              <n-p depth="3">Hello chameleon</n-p>
            </main>
          </div>
        </n-message-provider>
      </n-dialog-provider>
    </n-loading-bar-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import Topbar from "./components/Topbar.vue";
import { useTheme } from "./composables/useTheme";
import { loadPrefs } from "./composables/usePrefs";

const { naiveTheme, fluentOverride } = useTheme();

onMounted(() => {
  // 启动时加载偏好（主题 / accent / opacity）。
  loadPrefs();
});
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
