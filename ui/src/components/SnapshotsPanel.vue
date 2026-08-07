<template>
  <n-modal
    :show="show"
    preset="card"
    title="会话快照"
    :style="{ width: '520px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <div class="snapshots-body">
      <!-- 顶部：保存快照 -->
      <div class="snapshots-actions">
        <n-input
          v-model:value="newName"
          placeholder="快照名称"
          clearable
          class="snapshot-name-input"
        />
        <n-button
          type="primary"
          :disabled="!newName.trim()"
          :loading="saving"
          @click="handleSave"
        >
          保存快照
        </n-button>
      </div>

      <!-- 快照列表 -->
      <div v-if="snapshots.length === 0" class="snapshots-empty">
        <n-text depth="3">暂无快照</n-text>
      </div>
      <div v-else class="snapshots-list">
        <div
          v-for="name in snapshots"
          :key="name"
          class="snapshot-row"
        >
          <span class="snapshot-name" :title="name">{{ name }}</span>
          <n-space :size="4">
            <n-popconfirm
              :show="restoreTarget === name"
              positive-text="恢复"
              negative-text="取消"
              @positive-click="handleRestore(name)"
              @negative-click="restoreTarget = null"
              @click-outside="restoreTarget = null"
            >
              <template #trigger>
                <n-button
                  size="small"
                  @click="restoreTarget = name"
                >
                  恢复
                </n-button>
              </template>
              确定恢复到快照「{{ name }}」？
            </n-popconfirm>
            <n-popconfirm
              :show="deleteTarget === name"
              positive-text="删除"
              negative-text="取消"
              @positive-click="handleDelete(name)"
              @negative-click="deleteTarget = null"
              @click-outside="deleteTarget = null"
            >
              <template #trigger>
                <n-button
                  size="small"
                  text
                  type="error"
                  @click="deleteTarget = name"
                >
                  删除
                </n-button>
              </template>
              确定删除快照「{{ name }}」？
            </n-popconfirm>
          </n-space>
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

const snapshots = computed(() => appState.value.snapshots);
const newName = ref("");
const saving = ref(false);
const restoreTarget = ref<string | null>(null);
const deleteTarget = ref<string | null>(null);
const message = useMessage();

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

async function handleSave() {
  const name = newName.value.trim();
  if (!name) return;
  saving.value = true;
  try {
    await tauri.saveSnapshot(name);
    message.success("快照已保存");
    newName.value = "";
    await loadAppState();
  } catch (err) {
    message.error("保存快照失败");
  } finally {
    saving.value = false;
  }
}

async function handleRestore(name: string) {
  restoreTarget.value = null;
  try {
    await tauri.restoreSnapshot(name);
    await loadAppState();
  } catch (err) {
    message.error("恢复快照失败");
  }
}

async function handleDelete(name: string) {
  deleteTarget.value = null;
  try {
    await tauri.deleteSnapshot(name);
    await loadAppState();
  } catch (err) {
    message.error("删除快照失败");
  }
}
</script>

<style scoped>
.snapshots-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.snapshots-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.snapshot-name-input {
  flex: 1 1 auto;
}

.snapshots-empty {
  padding: 24px 0;
  text-align: center;
}

.snapshots-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.snapshot-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  border-radius: 4px;
}
.snapshot-row:hover {
  background: rgba(128, 128, 128, 0.12);
}

.snapshot-name {
  font-size: 13px;
  font-variant-numeric: tabular-nums;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1 1 auto;
  min-width: 0;
}
</style>
