<template>
  <n-modal
    :show="show"
    preset="card"
    title="临时沙箱"
    :style="{ width: '520px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <div class="sandboxes-body">
      <!-- 顶部操作 -->
      <div class="sandboxes-actions">
        <n-button
          type="primary"
          :loading="launching"
          @click="handleLaunch"
        >
          启动新沙箱
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
            <n-button
              :disabled="sandboxes.length === 0"
              @click="showCleanupPop = true"
            >
              清理全部
            </n-button>
          </template>
          确定清理全部临时沙箱？
        </n-popconfirm>
      </div>

      <!-- 沙箱列表 -->
      <div v-if="sandboxes.length === 0" class="sandboxes-empty">
        <n-text depth="3">暂无活跃沙箱</n-text>
      </div>
      <div v-else class="sandboxes-list">
        <div
          v-for="sb in sandboxes"
          :key="sb.id"
          class="sandbox-row"
        >
          <span class="sandbox-id" :title="sb.dir">{{ shortId(sb.id) }}</span>
          <n-button
            size="small"
            :loading="closingId === sb.id"
            @click="handleClose(sb.id)"
          >
            关闭
          </n-button>
        </div>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMessage } from "naive-ui";
import { appState, loadAppState } from "../composables/useAppState";
import { tauri } from "../composables/useTauri";
defineProps<{
  show: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();

const sandboxes = computed(() => appState.value.sandboxes);
const launching = ref(false);
const closingId = ref<string | null>(null);
const showCleanupPop = ref(false);
const message = useMessage();

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

function shortId(id: string): string {
  return id.length > 8 ? id.slice(0, 8) : id;
}

async function handleLaunch() {
  launching.value = true;
  try {
    await tauri.launchSandbox();
    await loadAppState();
  } catch (err) {
    message.error("启动沙箱失败");
  } finally {
    launching.value = false;
  }
}

async function handleClose(id: string) {
  closingId.value = id;
  try {
    await tauri.closeSandbox(id);
    await loadAppState();
  } catch (err) {
    message.error("关闭沙箱失败");
  } finally {
    closingId.value = null;
  }
}

async function handleCleanup() {
  showCleanupPop.value = false;
  try {
    await tauri.cleanupTemp();
    await loadAppState();
  } catch (err) {
    message.error("清理沙箱数据失败");
  }
}
</script>

<style scoped>
.sandboxes-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.sandboxes-actions {
  display: flex;
  gap: 8px;
}

.sandboxes-empty {
  padding: 24px 0;
  text-align: center;
}

.sandboxes-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sandbox-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  border-radius: 4px;
}
.sandbox-row:hover {
  background: rgba(128, 128, 128, 0.12);
}

.sandbox-id {
  font-family: monospace;
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1 1 auto;
  min-width: 0;
}
</style>
