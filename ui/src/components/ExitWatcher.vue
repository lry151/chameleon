<template>
  <!-- 不渲染内容；仅作为 Tauri 事件监听 + 非阻塞提示的载体。 -->
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { useMessage } from "naive-ui";
import { appState, loadAppState } from "../composables/useAppState";

const message = useMessage();
let unlistenRole: (() => void) | null = null;
let unlistenSandbox: (() => void) | null = null;

onMounted(async () => {
  // 查名字放在 loadAppState 之前——否则 role 已被从 session 移除，刷新后 appState 里没有。
  unlistenRole = await listen<{ id: string }>("role-exited", (e) => {
    const role = appState.value.roles.find((r) => r.id === e.payload.id);
    const name = role?.name ?? e.payload.id;
    message.info(`「${name}」窗口已关闭`, { duration: 3000 });
    void loadAppState();
  });

  // 沙箱无名，直接提示。
  unlistenSandbox = await listen<{ id: string }>("sandbox-exited", () => {
    message.info("沙箱窗口已关闭", { duration: 3000 });
    void loadAppState();
  });
});

onUnmounted(() => {
  unlistenRole?.();
  unlistenSandbox?.();
});
</script>
