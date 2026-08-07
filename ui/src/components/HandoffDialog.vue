<template>
  <n-modal
    :show="show"
    preset="card"
    title="接力"
    :style="{ width: '480px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <!-- 线性流程：源角色已锁定 → 选目标 → 选模式 → 提交 -->
    <n-form label-placement="top" class="handoff-form">
      <!-- Step 1: 源角色（锁定显示） -->
      <n-form-item label="源角色">
        <div class="source-locked">
          <span
            class="source-swatch"
            :style="{ backgroundColor: source.color }"
            aria-hidden="true"
          />
          <n-text>{{ source.name }}</n-text>
        </div>
      </n-form-item>

      <!-- Step 2: 选择目标角色 -->
      <n-form-item label="目标角色">
        <n-select
          v-model:value="targetId"
          :options="targetOptions"
          placeholder="选择目标角色"
          filterable
        />
      </n-form-item>

      <!-- Step 3: 选择模式 -->
      <n-form-item label="接力模式">
        <n-radio-group v-model:value="mode">
          <n-space vertical :size="12">
            <n-space :size="12" align="center">
              <n-radio value="parallel">并行模式</n-radio>
              <n-text class="mode-hint" depth="3">两窗口同时打开，可对比查看</n-text>
            </n-space>
            <n-space :size="12" align="center">
              <n-radio value="relay">接力模式</n-radio>
              <n-text class="mode-hint" depth="3">URL 传给目标角色，源窗口关闭</n-text>
            </n-space>
          </n-space>
        </n-radio-group>
      </n-form-item>
    </n-form>

    <template #footer>
      <div class="form-actions">
        <n-button
          type="primary"
          :disabled="!canSubmit"
          :loading="submitting"
          @click="handleSubmit"
        >
          开始接力
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import type { HandoffMode, RoleView } from "../types/api";
import { tauri } from "../composables/useTauri";
import { appState, loadAppState } from "../composables/useAppState";

const props = defineProps<{
  show: boolean;
  source: RoleView;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "done"): void;
}>();

const targetId = ref<string | null>(null);
const mode = ref<HandoffMode>("parallel");
const submitting = ref(false);
const message = useMessage();


const targetOptions = computed(() =>
  appState.value.roles
    .filter((r) => r.id !== props.source.id)
    .map((r) => ({ label: r.name, value: r.id })),
);

const canSubmit = computed(() => targetId.value !== null);

watch(
  () => props.show,
  (visible) => {
    if (visible) {
      targetId.value = null;
      mode.value = "parallel";
    }
  },
);

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

async function handleSubmit() {
  if (!targetId.value) return;
  submitting.value = true;
  try {
    await tauri.handoff(props.source.id, targetId.value, mode.value);
    await loadAppState();
    const targetRole = appState.value.roles.find((r) => r.id === targetId.value);
    message.success(`接力完成，已打开「${targetRole?.name ?? ""}」`);
    emit("done");
    emit("update:show", false);
  } catch (err: any) {
    message.error(`接力失败：${err?.message ?? err}`);
    submitting.value = false;
  }
}
</script>

<style scoped>
.handoff-form {
  display: flex;
  flex-direction: column;
}

.source-locked {
  display: flex;
  align-items: center;
  gap: 8px;
}

.source-swatch {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.mode-hint {
  font-size: 12px;
}
</style>
