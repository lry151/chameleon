<template>
  <n-config-provider :theme="darkTheme">
    <n-layout style="height: 100vh">
      <n-layout-header bordered style="padding: 16px 24px">
        <n-h1 style="margin: 0">🦎 chameleon</n-h1>
      </n-layout-header>
      <n-layout-content content-style="padding: 24px;">
        <n-p>Hello chameleon</n-p>
      </n-layout-content>
    </n-layout>
  </n-config-provider>
</template>

<script setup lang="ts">
import { darkTheme } from "naive-ui";
</script>
