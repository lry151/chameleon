<template>
  <n-modal
    :show="show"
    preset="card"
    title="管理预设"
    :style="{ width: '640px' }"
    :mask-closable="true"
    @update:show="onUpdateShow"
  >
    <div class="preset-list">
      <!-- 空态 -->
      <n-empty
        v-if="currentLinks.length === 0 && !creating"
        description="暂无预设"
        size="small"
      />

      <!-- 已有预设列表 + 新增空行（合并在同一渲染路径，编辑/新增共用一套行内表单） -->
      <div
        v-for="link in displayRows"
        :key="link.id"
        class="preset-row"
        :class="{ 'preset-row--open': expandedId === link.id }"
      >
        <!-- 折叠态：一行摘要 -->
        <template v-if="expandedId !== link.id">
          <div class="preset-text" @click="expand(link.id)">
            <span class="preset-name" :title="link.name || link.url">
              {{ link.name || '未命名' }}
            </span>
            <span class="preset-url" :title="link.url">
              {{ link.url }}
            </span>
          </div>
          <n-checkbox :checked="link.auto_open" disabled size="small">
            启动
          </n-checkbox>
          <n-button size="tiny" quaternary @click="expand(link.id)">
            编辑
          </n-button>
          <n-popconfirm
            :show="deleteTarget === link.id"
            positive-text="删除"
            negative-text="取消"
            @positive-click="doRemove(link.id)"
            @negative-click="deleteTarget = null"
            @click-outside="deleteTarget = null"
          >
            <template #trigger>
              <n-button
                size="tiny"
                quaternary
                type="error"
                @click="deleteTarget = link.id"
              >
                ×
              </n-button>
            </template>
            确定删除预设「{{ link.name || link.url }}」？
          </n-popconfirm>
        </template>

        <!-- 展开态：行内编辑 -->
        <template v-else>
          <n-form label-placement="top" class="preset-edit-form">
            <n-form-item label="名称">
              <n-input
                v-model:value="draft.name"
                type="text"
                placeholder="给这个预设取个名字（可留空，用 URL 代替）"
                clearable
              />
            </n-form-item>
            <n-form-item label="URL">
              <n-input
                v-model:value="draft.url"
                type="text"
                placeholder="https://example.com"
                clearable
              />
            </n-form-item>
            <n-form-item>
              <n-checkbox v-model:checked="draft.autoOpen">
                启动时自动打开
              </n-checkbox>
            </n-form-item>

            <!-- 含登录辅助（仅角色级） -->
            <n-form-item v-if="isRole">
              <n-checkbox v-model:checked="draft.hasLogin">
                含登录辅助
              </n-checkbox>
            </n-form-item>

            <!-- 登录辅助字段：用户名 + 密码（主区必填），选择器（高级区可选，默认折叠） -->
            <div v-if="isRole && draft.hasLogin" class="login-assist-fields">
              <n-form-item label="用户名">
                <n-input
                  v-model:value="draft.username"
                  type="text"
                  placeholder="登录用户名"
                  clearable
                />
              </n-form-item>
              <n-form-item label="密码">
                <n-input
                  v-model:value="draft.password"
                  type="password"
                  show-password-on="click"
                  placeholder="登录密码"
                  clearable
                />
              </n-form-item>
              <n-button
                size="tiny"
                quaternary
                class="advanced-toggle"
                @click="advancedOpen = !advancedOpen"
              >
                {{ advancedOpen ? "收起高级选项 ▴" : "高级选项 ▾" }}
              </n-button>
              <div v-show="advancedOpen" class="login-advanced">
                <n-form-item label="用户名选择器">
                  <n-input
                    v-model:value="draft.usernameSelector"
                    type="text"
                    placeholder="CSS 选择器，如 #username"
                    clearable
                  />
                </n-form-item>
                <n-form-item label="密码选择器">
                  <n-input
                    v-model:value="draft.passwordSelector"
                    type="text"
                    placeholder="CSS 选择器，如 #password"
                    clearable
                  />
                </n-form-item>
              </div>
            </div>

            <div class="preset-edit-actions">
              <n-button
                type="primary"
                size="small"
                :disabled="!canSaveDraft"
                :loading="savingId === link.id"
                @click="saveDraft"
              >
                {{ creating ? "添加" : "保存" }}
              </n-button>
              <n-button size="small" @click="collapse">取消</n-button>
            </div>
          </n-form>
        </template>
      </div>

      <!-- 新增入口：列表底部空行 -->
      <n-button v-if="!creating" block dashed @click="startCreate">
        + 添加预设
      </n-button>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useMessage } from "naive-ui";
import type { QuickLink, QuickLinkLogin } from "../types/api";
import { tauri } from "../composables/useTauri";
import { appState, loadAppState } from "../composables/useAppState";
import { notifyBackendError } from "../composables/useErrorToast";

const props = defineProps<{
  show: boolean;
  ownerId: string;
  ownerKind: "role" | "system";
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
}>();
const message = useMessage();

const isRole = computed(() => props.ownerKind === "role");

/// 行内编辑表单状态。
interface Draft {
  name: string;
  url: string;
  autoOpen: boolean;
  hasLogin: boolean;
  username: string;
  password: string;
  usernameSelector: string;
  passwordSelector: string;
}

const draft = ref<Draft>({
  name: "",
  url: "",
  autoOpen: false,
  hasLogin: false,
  username: "",
  password: "",
  usernameSelector: "",
  passwordSelector: "",
});

/// 展开编辑的预设 id；'__new__' = 底部新增空行。
const expandedId = ref<string | null>(null);
const creating = ref(false);
const savingId = ref<string | null>(null);
const deleteTarget = ref<string | null>(null);
/// 高级区（选择器）默认折叠。
const advancedOpen = ref(false);

/// 当前 owner 的预设列表（从 appState 派生）。
const currentLinks = computed<QuickLink[]>(() => {
  if (isRole.value) {
    const role = appState.value.roles.find((r) => r.id === props.ownerId);
    return role?.quick_links ?? [];
  }
  const sys = appState.value.systems.find((s) => s.id === props.ownerId);
  return sys?.quick_links ?? [];
});

/// 列表渲染 = 已有预设 + 新增空行（新增行恒为展开态）。
const displayRows = computed<QuickLink[]>(() => {
  if (!creating.value) return currentLinks.value;
  return [
    ...currentLinks.value,
    { id: "__new__", name: null, url: "", auto_open: false, login: null },
  ];
});

/// 保存条件：URL 必填；勾了登录辅助则用户名 + 密码必填（主区必填）。
const canSaveDraft = computed(() => {
  if (!draft.value.url.trim()) return false;
  if (isRole.value && draft.value.hasLogin) {
    return (
      draft.value.username.trim().length > 0 &&
      draft.value.password.length > 0
    );
  }
  return true;
});

/// dialog 关闭时重置。
watch(
  () => props.show,
  (visible) => {
    if (!visible) {
      collapse();
      resetDraft();
      deleteTarget.value = null;
    }
  },
);

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}

function resetDraft() {
  draft.value = {
    name: "",
    url: "",
    autoOpen: false,
    hasLogin: false,
    username: "",
    password: "",
    usernameSelector: "",
    passwordSelector: "",
  };
  advancedOpen.value = false;
}

function collapse() {
  expandedId.value = null;
  creating.value = false;
}

function expand(id: string) {
  const link = currentLinks.value.find((l) => l.id === id);
  if (!link) return;
  draft.value.name = link.name ?? "";
  draft.value.url = link.url;
  draft.value.autoOpen = link.auto_open;
  if (link.login) {
    draft.value.hasLogin = true;
    draft.value.username = link.login.username;
    draft.value.password = link.login.password;
    draft.value.usernameSelector = link.login.username_selector ?? "";
    draft.value.passwordSelector = link.login.password_selector ?? "";
  } else {
    draft.value.hasLogin = false;
    draft.value.username = "";
    draft.value.password = "";
    draft.value.usernameSelector = "";
    draft.value.passwordSelector = "";
  }
  advancedOpen.value = false;
  expandedId.value = id;
  creating.value = false;
}

function startCreate() {
  resetDraft();
  creating.value = true;
  expandedId.value = "__new__";
}

function buildLogin(): QuickLinkLogin | null {
  if (!isRole.value || !draft.value.hasLogin) return null;
  if (!draft.value.username.trim() || !draft.value.password) return null;
  return {
    username: draft.value.username.trim(),
    password: draft.value.password,
    username_selector: draft.value.usernameSelector.trim() || null,
    password_selector: draft.value.passwordSelector.trim() || null,
  };
}

async function saveDraft() {
  const url = draft.value.url.trim();
  if (!url) return;
  const name = draft.value.name.trim() || url;
  const autoOpen = draft.value.autoOpen;
  const login = buildLogin();
  const targetId = expandedId.value;
  if (!targetId) return;

  savingId.value = targetId;
  try {
    if (isRole.value) {
      if (creating.value) {
        await tauri.addQuickLink(props.ownerId, name, url, autoOpen, login);
      } else if (targetId !== "__new__") {
        await tauri.editQuickLink(
          props.ownerId,
          targetId,
          name,
          url,
          autoOpen,
          login,
        );
      }
    } else {
      if (creating.value) {
        await tauri.addSystemQuickLink(props.ownerId, name, url, autoOpen);
      } else if (targetId !== "__new__") {
        await tauri.editSystemQuickLink(
          props.ownerId,
          targetId,
          name,
          url,
          autoOpen,
        );
      }
    }
    await loadAppState();
    message.success("预设已保存");
    collapse();
  } catch (err) {
    notifyBackendError(message, err, "保存预设失败");
  } finally {
    savingId.value = null;
  }
}

async function doRemove(id: string) {
  deleteTarget.value = null;
  try {
    if (isRole.value) {
      await tauri.removeQuickLink(props.ownerId, id);
    } else {
      await tauri.removeSystemQuickLink(props.ownerId, id);
    }
    await loadAppState();
    if (expandedId.value === id) {
      collapse();
    }
  } catch (err) {
    notifyBackendError(message, err, "删除预设失败");
  }
}
</script>

<style scoped>
.preset-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.preset-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
}
.preset-row:hover {
  background: rgba(128, 128, 128, 0.12);
}

.preset-row--open {
  background: rgba(128, 128, 128, 0.06);
  align-items: stretch;
}

.preset-text {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  cursor: pointer;
}
.preset-name {
  font-size: 14px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.preset-url {
  font-size: 12px;
  opacity: 0.6;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preset-edit-form {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
}

.preset-edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

/* 登录辅助 */
.login-assist-fields {
  padding-left: 16px;
  display: flex;
  flex-direction: column;
}

.advanced-toggle {
  align-self: flex-start;
  font-size: 12px;
  margin-bottom: 8px;
}

.login-advanced {
  display: flex;
  flex-direction: column;
}
</style>
