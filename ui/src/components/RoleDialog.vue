<template>
  <n-modal
    :show="show"
    preset="card"
    :title="dialogTitle"
    :style="{ width: '480px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <n-form label-placement="top" class="role-form">
      <n-form-item label="角色名">
        <n-input
          v-model:value="formName"
          placeholder="输入角色名称"
          clearable
        />
      </n-form-item>

      <n-form-item label="角色颜色">
        <div class="color-row">
          <button
            v-for="c in COLOR_PALETTE"
            :key="c"
            class="color-dot"
            :class="{ 'color-dot--active': formColor === c }"
            :style="{ backgroundColor: c }"
            :aria-label="`颜色 ${c}`"
            @click="formColor = c"
          />
        </div>
      </n-form-item>

      <n-form-item label="归属系统">
        <n-select
          v-model:value="formSystemId"
          :options="systemOptions"
          placeholder="不归属任何系统"
          clearable
        />
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
import type { Role, RoleView } from "../types/api";
import { tauri } from "../composables/useTauri";
import { appState, loadAppState } from "../composables/useAppState";
import { notifyBackendError } from "../composables/useErrorToast";

const props = defineProps<{
  show: boolean;
  /// null = 新建模式；非 null = 编辑 / 克隆模式。
  role: RoleView | null;
  /// true = 克隆模式（预填字段但创建新角色）。
  clone: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "saved"): void;
}>();
const message = useMessage();


const COLOR_PALETTE: string[] = [
  "#e74c3c",
  "#e67e22",
  "#f1c40f",
  "#2ecc71",
  "#1abc9c",
  "#3498db",
  "#9b59b6",
  "#34495e",
];

const editing = computed(() => props.role !== null && !props.clone);
const dialogTitle = computed(() => {
  if (props.clone) return "克隆角色";
  if (props.role) return "编辑角色";
  return "新建角色";
});

const formName = ref("");
const formColor = ref(COLOR_PALETTE[0]);
const formSystemId = ref<string | null>(null);

const systemOptions = computed(() =>
  appState.value.systems.map((s) => ({ label: s.name, value: s.id })),
);

watch(
  () => props.show,
  (visible) => {
    if (visible && props.role) {
      formName.value = props.clone ? `${props.role.name} (副本)` : props.role.name;
      formColor.value = props.role.color;
      formSystemId.value = props.role.system_id;
    } else if (visible) {
      formName.value = "";
      formColor.value = COLOR_PALETTE[0];
      formSystemId.value = null;
    }
  },
);

const canSave = computed(() => formName.value.trim().length > 0);
const saving = ref(false);

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

function onCancel() {
  emit("update:show", false);
}

async function handleSave() {
  const trimmed = formName.value.trim();
  if (!trimmed) return;
  saving.value = true;
  try {
    if (editing.value && props.role) {
      // 编辑：更新现有角色。
      const updated: Role = {
        ...props.role,
        name: trimmed,
        color: formColor.value,
        system_id: formSystemId.value,
      };
      await tauri.updateRole(updated);
    } else {
      // 新建或克隆：先创建，再补全字段。
      const created = await tauri.createRole(trimmed, formColor.value);
      if (formSystemId.value || (props.clone && props.role)) {
        const sourceLinks = props.clone && props.role ? props.role.quick_links : [];
        await tauri.updateRole({
          ...created,
          system_id: formSystemId.value,
          quick_links: sourceLinks,
        });
      }
    }
    await loadAppState();
    message.success("角色已保存");
    emit("saved");
    emit("update:show", false);
  } catch (err) {
    notifyBackendError(message, err, "保存角色失败");
  } finally {
    saving.value = false;
  }
}
</script>

<style scoped>
.role-form {
  display: flex;
  flex-direction: column;
}

.color-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.color-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
  transition: border-color 0.15s;
}
.color-dot:hover {
  border-color: rgba(128, 128, 128, 0.4);
}
.color-dot--active {
  border-color: currentColor;
  box-shadow: 0 0 0 2px rgba(128, 128, 128, 0.25);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
