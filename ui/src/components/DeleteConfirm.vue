<template>
  <n-modal
    :show="show"
    preset="dialog"
    :title="title"
    :positive-text="confirmText"
    :positive-button-props="{ type: 'error' }"
    :style="{ maxWidth: '400px' }"
    @positive-click="onConfirm"
    @negative-click="onCancel"
    @update:show="onUpdateShow"
  >
    {{ message }}
  </n-modal>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    show: boolean;
    title?: string;
    message: string;
    confirmText?: string;
  }>(),
  {
    title: "确认删除",
    confirmText: "确认删除",
  },
);

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "confirm"): void;
}>();

function onConfirm() {
  emit("confirm");
  emit("update:show", false);
}

function onCancel() {
  emit("update:show", false);
}

function onUpdateShow(value: boolean) {
  emit("update:show", value);
}
</script>
