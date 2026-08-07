<template>
  <n-modal
    :show="show"
    preset="card"
    title="管理预设"
    :style="{ width: '520px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <n-form
      :model="formState"
      label-placement="top"
      class="links-form"
    >
      <!-- URL 字段 -->
      <n-form-item label="URL">
        <n-input
          v-model:value="formState.url"
          type="text"
          placeholder="https://example.com"
          clearable
        />
      </n-form-item>

      <!-- 启动时自动打开 -->
      <n-form-item>
        <n-checkbox v-model:checked="formState.autoOpen">
          启动时自动打开
        </n-checkbox>
      </n-form-item>

      <!-- 含登录辅助（仅角色级） -->
      <n-form-item v-if="isRole">
        <n-checkbox v-model:checked="formState.hasLogin">
          含登录辅助
        </n-checkbox>
      </n-form-item>

      <!-- 登录辅助字段（条件展开，缩进 16px） -->
      <div v-if="isRole && formState.hasLogin" class="login-assist-fields">
        <n-form-item label="用户名">
          <n-input
            v-model:value="formState.username"
            type="text"
            placeholder="登录用户名"
            clearable
          />
        </n-form-item>
        <n-form-item label="输入框选择器">
          <n-input
            v-model:value="formState.usernameSelector"
            type="text"
            placeholder="CSS 选择器，如 #username 或 input[name=email]"
            clearable
          />
        </n-form-item>
        <n-button
          :disabled="!canTestLogin"
          :loading="testingLogin"
          @click="handleTestLogin"
        >
          测试登录
        </n-button>
      </div>

      <!-- 添加 / 保存按钮 -->
      <div class="form-submit">
        <n-button
          type="primary"
          :disabled="!canSubmit"
          :loading="submitting"
          @click="handleSubmit"
        >
          {{ editingName ? "保存" : "添加预设" }}
        </n-button>
      </div>
    </n-form>

    <n-divider />

    <!-- 已有预设列表 -->
    <div class="preset-list">
      <n-text v-if="currentLinks.length === 0" depth="3">
        暂无预设
      </n-text>
      <div
        v-for="link in currentLinks"
        :key="link.name"
        class="preset-row"
      >
        <span class="preset-url" :title="link.url">
          {{ link.name || link.url }}
        </span>
        <n-checkbox
          :checked="link.auto_open"
          disabled
          size="small"
        >
          启动
        </n-checkbox>
        <n-button size="tiny" quaternary @click="startEdit(link)">
          编辑
        </n-button>
        <n-popconfirm
          :show="deleteTarget === link.name"
          positive-text="删除"
          negative-text="取消"
          @positive-click="doRemove(link.name)"
          @negative-click="deleteTarget = null"
          @click-outside="deleteTarget = null"
        >
          <template #trigger>
            <n-button
              size="tiny"
              quaternary
              type="error"
              @click="deleteTarget = link.name"
            >
              ×
            </n-button>
          </template>
          确定删除预设「{{ link.name || link.url }}」？
        </n-popconfirm>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { QuickLink, QuickLinkLogin } from "../types/api";
import { tauri } from "../composables/useTauri";
import { appState, loadAppState } from "../composables/useAppState";

const props = defineProps<{
  show: boolean;
  ownerId: string;
  ownerKind: "role" | "system";
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();

const isRole = computed(() => props.ownerKind === "role");

/// 表单状态。
interface FormState {
  url: string;
  autoOpen: boolean;
  hasLogin: boolean;
  username: string;
  usernameSelector: string;
}

const formState = ref<FormState>({
  url: "",
  autoOpen: false,
  hasLogin: false,
  username: "",
  usernameSelector: "",
});

/// 编辑模式：记录正在编辑的预设原始 name（即 URL）。
const editingName = ref<string | null>(null);
const submitting = ref(false);
const testingLogin = ref(false);
const deleteTarget = ref<string | null>(null);

/// 当前 owner 的预设列表（从 appState 派生）。
const currentLinks = computed<QuickLink[]>(() => {
  if (isRole.value) {
    const role = appState.value.roles.find((r) => r.id === props.ownerId);
    return role?.quick_links ?? [];
  }
  const sys = appState.value.systems.find((s) => s.id === props.ownerId);
  return sys?.quick_links ?? [];
});

const canSubmit = computed(() => formState.value.url.trim().length > 0);

const canTestLogin = computed(
  () =>
    isRole.value &&
    formState.value.hasLogin &&
    formState.value.url.trim().length > 0 &&
    formState.value.username.trim().length > 0,
);

/// dialog 关闭时重置表单。
watch(
  () => props.show,
  (visible) => {
    if (!visible) resetForm();
  },
);

function resetForm() {
  formState.value = {
    url: "",
    autoOpen: false,
    hasLogin: false,
    username: "",
    usernameSelector: "",
  };
  editingName.value = null;
  deleteTarget.value = null;
}

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

function startEdit(link: QuickLink) {
  formState.value.url = link.url;
  formState.value.autoOpen = link.auto_open;
  if (link.login) {
    formState.value.hasLogin = true;
    formState.value.username = link.login.username;
    formState.value.usernameSelector = link.login.username_selector ?? "";
  } else {
    formState.value.hasLogin = false;
    formState.value.username = "";
    formState.value.usernameSelector = "";
  }
  editingName.value = link.name;
}

function buildLogin(): QuickLinkLogin | null {
  if (!isRole.value || !formState.value.hasLogin) return null;
  if (!formState.value.username.trim()) return null;
  return {
    username: formState.value.username.trim(),
    password: "",
    username_selector: formState.value.usernameSelector.trim() || null,
    password_selector: null,
  };
}

async function handleSubmit() {
  const url = formState.value.url.trim();
  if (!url) return;
  const name = url;
  const autoOpen = formState.value.autoOpen;
  const login = buildLogin();

  submitting.value = true;
  try {
    if (isRole.value) {
      if (editingName.value) {
        await tauri.editQuickLink(
          props.ownerId,
          editingName.value,
          name,
          url,
          autoOpen,
          login,
        );
      } else {
        await tauri.addQuickLink(
          props.ownerId,
          name,
          url,
          autoOpen,
          login,
        );
      }
    } else {
      if (editingName.value) {
        await tauri.editSystemQuickLink(
          props.ownerId,
          editingName.value,
          name,
          url,
          autoOpen,
        );
      } else {
        await tauri.addSystemQuickLink(
          props.ownerId,
          name,
          url,
          autoOpen,
        );
      }
    }
    await loadAppState();
    resetForm();
  } catch (err) {
    console.error("Failed to save quick link:", err);
  } finally {
    submitting.value = false;
  }
}

async function doRemove(name: string) {
  deleteTarget.value = null;
  try {
    if (isRole.value) {
      await tauri.removeQuickLink(props.ownerId, name);
    } else {
      await tauri.removeSystemQuickLink(props.ownerId, name);
    }
    await loadAppState();
    if (editingName.value === name) {
      resetForm();
    }
  } catch (err) {
    console.error("Failed to remove quick link:", err);
  }
}

async function handleTestLogin() {
  if (!canTestLogin.value) return;
  testingLogin.value = true;
  try {
    const url = formState.value.url.trim();
    const name = url;
    const login = buildLogin();
    if (!login) return;

    // 先保存（添加或更新），再打开测试。
    if (editingName.value) {
      await tauri.editQuickLink(
        props.ownerId,
        editingName.value,
        name,
        url,
        formState.value.autoOpen,
        login,
      );
    } else {
      // 如果同名已存在则更新，否则新增。
      const existing = currentLinks.value.find((l) => l.name === name);
      if (existing) {
        await tauri.editQuickLink(
          props.ownerId,
          existing.name,
          name,
          url,
          formState.value.autoOpen,
          login,
        );
      } else {
        await tauri.addQuickLink(
          props.ownerId,
          name,
          url,
          formState.value.autoOpen,
          login,
        );
      }
    }
    await loadAppState();
    editingName.value = name;
    await tauri.openQuickLink(props.ownerId, name);
  } catch (err) {
    console.error("Failed to test login:", err);
  } finally {
    testingLogin.value = false;
  }
}
</script>

<style scoped>
.links-form {
  display: flex;
  flex-direction: column;
}

.login-assist-fields {
  padding-left: 16px;
  display: flex;
  flex-direction: column;
}

.form-submit {
  display: flex;
  justify-content: flex-end;
  margin-top: 4px;
}

.preset-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.preset-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 4px;
  border-radius: 4px;
}
.preset-row:hover {
  background: rgba(128, 128, 128, 0.08);
}

.preset-url {
  flex: 1 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}
</style>
