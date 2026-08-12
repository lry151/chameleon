<template>
  <n-modal
    :show="show"
    preset="card"
    :title="editing ? '编辑系统' : '新建系统'"
    :style="{ width: '480px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <n-form label-placement="top" class="system-form">
      <n-form-item label="系统名">
        <n-input
          v-model:value="name"
          placeholder="输入系统名称"
          clearable
          @keydown.enter="handleSave"
        />
      </n-form-item>

      <n-form-item v-if="editing && systemId" label="系统级 Quick Links">
        <n-button secondary @click="emit('manageLinks', systemId!)">
          管理预设
        </n-button>
      </n-form-item>
    </n-form>

    <template #footer>
      <div class="form-actions">
        <n-button @click="onCancel">取消</n-button>
        <n-button
          type="primary"
          :disabled="!canSave"
          :loading="saving"
          @click="handleSave"
        >
          保存
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import { appState, loadAppState } from "../composables/useAppState";
import { notifyBackendError } from "../composables/useErrorToast";
import { tauri } from "../composables/useTauri";

const props = defineProps<{
  show: boolean;
  /// null = 新建模式；非 null = 编辑模式。
  systemId: string | null;
  systemName: string;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "saved"): void;
  (e: "manageLinks", systemId: string): void;
}>();
const message = useMessage();


const editing = computed(() => props.systemId !== null);
const name = ref("");

watch(
  () => props.show,
  (visible) => {
    if (visible) name.value = props.systemName;
  },
);

const canSave = computed(() => name.value.trim().length > 0);
const saving = ref(false);

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

function onCancel() {
  emit("update:show", false);
}

async function handleSave() {
  const trimmed = name.value.trim();
  if (!trimmed) return;
  saving.value = true;
  try {
    if (editing.value && props.systemId) {
      const existing = appState.value.systems.find((s) => s.id === props.systemId);
      await tauri.updateSystem({
        id: props.systemId,
        name: trimmed,
        quick_links: existing?.quick_links ?? [],
      });
    } else {
      await tauri.createSystem(trimmed);
    }
    await loadAppState();
    message.success("系统已保存");
    emit("saved");
    emit("update:show", false);
  } catch (err) {
    notifyBackendError(message, err, "保存系统失败");
  } finally {
    saving.value = false;
  }
}
</script>

<style scoped>
.system-form {
  display: flex;
  flex-direction: column;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
