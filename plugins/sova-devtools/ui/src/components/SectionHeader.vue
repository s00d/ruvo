<script setup lang="ts">
import type { Component } from "vue";
import Icon from "./Icon.vue";

defineProps<{
  title: string;
  icon?: Component;
  hint?: string;
}>();
</script>

<template>
  <div class="mb-2 flex items-center gap-2">
    <Icon v-if="icon" :icon="icon" :size="14" class="text-[var(--dt-accent)]" />
    <h3 class="m-0 text-[12px] font-semibold tracking-wide text-[var(--dt-text)]">
      {{ title }}
    </h3>
    <span v-if="hint" class="text-[11px] text-[var(--dt-faint)]">{{ hint }}</span>
    <div class="ml-auto flex items-center gap-2">
      <slot name="actions" />
    </div>
  </div>
</template>
