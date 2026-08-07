<template>
  <div class="main-view">
    <div v-if="loading" class="main-loading">
      <n-spin size="medium" />
    </div>
    <div v-else-if="buckets.length === 0" class="main-empty">
      <n-empty description="暂无角色">
        <template #extra>
          <n-text depth="3">通过顶栏「新建角色」开始</n-text>
        </template>
      </n-empty>
    </div>
    <div v-else class="system-list">
      <SystemBox
        v-for="bucket in buckets"
        :key="bucket.system?.id ?? '__ungrouped__'"
        :system="bucket.system"
        :roles="bucket.roles"
        @presets="onPresets"
        @handoff="onHandoff"
        @edit-role="$emit('editRole', $event)"
        @clone-role="$emit('cloneRole', $event)"
        @edit-system="$emit('editSystem', $event)"
        @presets-system="onPresetsSystem"
        @role-deleted="refresh"
        @system-deleted="refresh"
      />
    </div>

    <!-- LinksDialog（角色 / 系统级 Quick Links 管理） -->
    <LinksDialog
      v-model:show="showLinks"
      :owner-id="linksOwnerId"
      :owner-kind="linksOwnerKind"
    />

    <!-- HandoffDialog（接力） -->
    <HandoffDialog
      v-if="handoffSource"
      v-model:show="showHandoff"
      :source="handoffSource"
      @done="refresh"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { RoleView, System } from "../types/api";
import { loadAppState, systemBuckets } from "../composables/useAppState";
import SystemBox from "../components/SystemBox.vue";
import LinksDialog from "../components/LinksDialog.vue";
import HandoffDialog from "../components/HandoffDialog.vue";
defineEmits<{
  (e: "editRole", role: RoleView): void;
  (e: "cloneRole", role: RoleView): void;
  (e: "editSystem", system: System): void;
}>();

const loading = ref(true);

const buckets = computed(() => systemBuckets());

onMounted(async () => {
  await loadAppState();
  loading.value = false;
});

async function refresh() {
  await loadAppState();
}

// —— LinksDialog ——
const showLinks = ref(false);
const linksOwnerId = ref("");
const linksOwnerKind = ref<"role" | "system">("role");

function onPresets(role: RoleView) {
  linksOwnerId.value = role.id;
  linksOwnerKind.value = "role";
  showLinks.value = true;
}

function onPresetsSystem(systemId: string) {
  linksOwnerId.value = systemId;
  linksOwnerKind.value = "system";
  showLinks.value = true;
}

// —— HandoffDialog ——
const showHandoff = ref(false);
const handoffSource = ref<RoleView | null>(null);

function onHandoff(role: RoleView) {
  handoffSource.value = role;
  showHandoff.value = true;
}
</script>

<style scoped>
.main-view {
  min-height: 100%;
}

.main-loading {
  display: flex;
  justify-content: center;
  padding: 32px 0;
}

.main-empty {
  display: flex;
  justify-content: center;
  padding: 48px 0;
}

.system-list {
  display: flex;
  flex-direction: column;
  gap: 24px;
  max-width: 1200px;
  margin: 0 auto;
}
</style>
