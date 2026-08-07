<template>
  <n-card class="role-card" :class="{ 'role-card--running': role.running }" :style="runningCardStyle">
    <!-- 头部：标识 + 状态 -->
    <template #header>
      <div class="role-header">
        <span
          class="role-swatch"
          :style="{ backgroundColor: role.color, transform: `scale(${swatchScale})` }"
          aria-hidden="true"
        />
        <span class="role-name" :title="role.name">{{ role.name }}</span>
        <n-tag
          v-if="role.running"
          :bordered="false"
          size="small"
          round
          :style="{ backgroundColor: role.color, color: tagTextColor }"
        >
          运行中
        </n-tag>
      </div>
    </template>

    <!-- 中部：preset chips -->
    <div class="role-links">
      <!-- 角色有自己的预设 -->
      <template v-if="hasOwnLinks">
        <n-button
          v-for="link in role.quick_links"
          :key="link.name"
          size="tiny"
          secondary
          class="role-link-chip"
          @click="openLink(link.url)"
        >
          {{ link.name || link.url }}
        </n-button>
      </template>
      <!-- 角色没有预设，但系统有预设 -->
      <template v-else-if="inheritedLinks.length > 0">
        <div class="inherited-links">
          <div class="inherited-header">
            <n-text depth="3" class="inherited-label">↓ 系统预设</n-text>
            <n-button
              size="tiny"
              quaternary
              class="inherited-apply-btn"
              @click="applySystemLinks"
            >
              应用到角色
            </n-button>
          </div>
          <div class="inherited-chips">
            <n-button
              v-for="link in inheritedLinks"
              :key="link.name"
              size="tiny"
              secondary
              class="inherited-link-chip"
              @click="openLink(link.url)"
            >
              {{ link.name || link.url }}
            </n-button>
          </div>
        </div>
      </template>
      <!-- 角色和系统都没有预设 -->
      <n-text v-else depth="3" class="role-links-empty">
        暂无预设
      </n-text>
    </div>

    <!-- 底部：actions -->
    <template #action>
      <div class="role-actions">
        <n-space :size="8">
          <n-button
            v-if="!role.running"
            type="primary"
            size="small"
            :loading="busy"
            @click="handleLaunch"
          >
            启动
          </n-button>
          <n-button
            v-else
            size="small"
            :loading="busy"
            @click="handleClose"
          >
            关闭
          </n-button>
          <n-button size="small" secondary @click="$emit('presets', role)">
            预设
          </n-button>
          <n-button size="small" secondary @click="$emit('handoff', role)">
            接力
          </n-button>

        <!-- 更多操作：popconfirm（删除确认）+ dropdown（编辑/克隆） -->
        <n-popconfirm
          :show="showDeletePop"
          :positive-text="'删除'"
          :negative-text="'取消'"
          @positive-click="doDelete"
          @negative-click="showDeletePop = false"
          @click-outside="showDeletePop = false"
        >
          <template #trigger>
            <n-dropdown
              trigger="click"
              :options="menuOptions"
              :keyboard="true"
              :disabled="showDeletePop"
              @select="handleMenuSelect"
            >
              <n-button
                size="small"
                quaternary
                aria-label="更多操作"
              >
                <template #icon>
                  <span aria-hidden="true">⋯</span>
                </template>
              </n-button>
            </n-dropdown>
          </template>
          确定删除角色「{{ role.name }}」？
        </n-popconfirm>
        </n-space>
      </div>
    </template>
  </n-card>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useMessage } from "naive-ui";
import type { QuickLink, RoleView } from "../types/api";
import { tauri } from "../composables/useTauri";
import { createSpring } from "../utils/spring";
import { loadAppState } from "../composables/useAppState";

const props = defineProps<{
  role: RoleView;
  systemLinks?: QuickLink[];
}>();

const emit = defineEmits<{
  (e: "presets", role: RoleView): void;
  (e: "handoff", role: RoleView): void;
  (e: "edit", role: RoleView): void;
  (e: "clone", role: RoleView): void;
  (e: "deleted"): void;
}>();

const busy = ref(false);
const showDeletePop = ref(false);
const swatchScale = ref(1);
const message = useMessage();

const hasOwnLinks = computed(() => props.role.quick_links.length > 0);
const inheritedLinks = computed(() =>
  !hasOwnLinks.value ? (props.systemLinks ?? []) : [],
);

async function applySystemLinks() {
  for (const link of inheritedLinks.value) {
    await tauri.addQuickLink(
      props.role.id,
      link.name,
      link.url,
      link.auto_open,
      link.login,
    );
  }
  await loadAppState();
  message.success(`已应用 ${inheritedLinks.value.length} 个系统预设`);
}

/// 「运行中」标签文字颜色：按角色颜色亮度选深/浅，保证在浅色(如黄 #f1c40f)棋盘上可读。
const tagTextColor = computed(() => readableOn(props.role.color));
/// 运行中角色卡片：左侧色条强化色块身份。
const runningCardStyle = computed(() =>
  props.role.running ? { borderLeft: `2px solid ${props.role.color}` } : {},
);
/// 返回在给定 hex 背景上可读的近似文字色（黑或白）。
function readableOn(hex: string): string {
  const c = hex.replace("#", "");
  if (c.length !== 6) return "#fff";
  const r = parseInt(c.slice(0, 2), 16);
  const g = parseInt(c.slice(2, 4), 16);
  const b = parseInt(c.slice(4, 6), 16);
  // 感知亮度（Rec.709 luma），阈值 0.6。
  const luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luma > 0.6 ? "#1A1A1A" : "#fff";
}

/// 启动/关闭后触发脉冲动画
function triggerPulse() {
  const spring = createSpring({
    from: 1,
    to: 1,
    stiffness: 400,
    damping: 15,
  });
  // 先放大再弹回
  let phase = 0;
  spring.onUpdate = (value) => {
    if (phase === 0) {
      swatchScale.value = 1 + value * 0.3;
      if (value > 0.5) phase = 1;
    } else {
      swatchScale.value = 1.3 - (value - 0.5) * 0.6;
    }
  };
  spring.onDone = () => {
    swatchScale.value = 1;
  };
  spring.start();
}

const menuOptions = computed(() => [
  { label: "编辑", key: "edit" },
  { label: "克隆", key: "clone" },
  { type: "divider", key: "d1" },
  { label: "删除", key: "delete" },
]);

async function handleLaunch() {
  busy.value = true;
  try {
    await tauri.launchRole(props.role.id);
    triggerPulse();
    await loadAppState();
  } catch (err) {
    message.error(`启动「${props.role.name}」失败，请检查浏览器路径`);
  } finally {
    busy.value = false;
  }
}

async function handleClose() {
  busy.value = true;
  try {
    await tauri.closeRole(props.role.id);
    triggerPulse();
    await loadAppState();
  } catch (err) {
    message.error(`关闭「${props.role.name}」失败，请稍后重试`);
  } finally {
    busy.value = false;
  }
}

function handleMenuSelect(key: string) {
  if (key === "edit") emit("edit", props.role);
  else if (key === "clone") emit("clone", props.role);
  else if (key === "delete") showDeletePop.value = true;
}

async function doDelete() {
  showDeletePop.value = false;
  busy.value = true;
  try {
    await tauri.deleteRole(props.role.id);
    await loadAppState();
    emit("deleted");
  } catch (err) {
    message.error(`删除角色「${props.role.name}」失败`);
  } finally {
    busy.value = false;
  }
}

function openLink(url: string) {
  window.open(url, "_blank", "noopener");
}
</script>

<style scoped>
.role-header {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.role-swatch {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  transform-origin: center;
}

.role-name {
  font-weight: 600;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1 1 auto;
  min-width: 0;
}

.role-links {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  min-height: 24px;
}

.role-link-chip {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.role-links-empty {
  font-size: 12px;
}
/* 系统预设继承区域 */
.inherited-links {
  width: 100%;
  padding: 8px;
  background: rgba(128, 128, 128, 0.04);
  border: 1px dashed rgba(128, 128, 128, 0.2);
  border-radius: 4px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.inherited-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.inherited-label {
  font-size: 11px;
}

.inherited-apply-btn {
  font-size: 11px;
}

.inherited-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.inherited-link-chip {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  opacity: 0.7;
}

.role-actions {
  display: flex;
  align-items: center;
}
</style>
