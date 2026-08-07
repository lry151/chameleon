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
        @edit="onEdit"
        @clone="onClone"
        @role-deleted="refresh"
        @system-deleted="refresh"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { RoleView } from "../types/api";
import { loadAppState, systemBuckets } from "../composables/useAppState";
import SystemBox from "../components/SystemBox.vue";

const loading = ref(true);

const buckets = computed(() => systemBuckets());

onMounted(async () => {
  await loadAppState();
  loading.value = false;
});

async function refresh() {
  await loadAppState();
}

// 后续工单（#7 RoleDialog 等）会接手这些事件；
// 目前仅占位，避免未处理事件警告。
function onPresets(_role: RoleView) { /* TODO: LinksDialog */ }
function onHandoff(_role: RoleView) { /* TODO: HandoffDialog */ }
function onEdit(_role: RoleView) { /* TODO: RoleDialog */ }
function onClone(_role: RoleView) { /* TODO: RoleDialog (clone mode) */ }
</script>

<style scoped>
.main-view {
  min-height: 100%;
}

.main-loading {
  display: flex;
  justify-content: center;
  padding: 48px 0;
}

.main-empty {
  display: flex;
  justify-content: center;
  padding: 64px 0;
}

.system-list {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
</style>
