<script setup lang="ts">
import type { Component } from "vue";
import Icon from "./Icon.vue";

defineProps<{
  label: string;
  value: string;
  hint?: string;
  tone?: "default" | "ok" | "warn" | "err" | "info";
  icon?: Component;
}>();
</script>

<template>
  <div
    class="flex min-w-0 flex-col gap-1 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] px-3 py-2.5"
  >
    <div class="flex items-center gap-1.5 text-[11px] text-[var(--dt-muted)]">
      <Icon v-if="icon" :icon="icon" :size="12" />
      <span>{{ label }}</span>
    </div>
    <div
      class="dt-mono truncate text-[15px] font-medium tracking-tight"
      :class="{
        'text-[var(--dt-text)]': !tone || tone === 'default',
        'text-[var(--dt-ok)]': tone === 'ok',
        'text-[var(--dt-warn)]': tone === 'warn',
        'text-[var(--dt-err)]': tone === 'err',
        'text-[var(--dt-info)]': tone === 'info',
      }"
    >
      {{ value }}
    </div>
    <div v-if="hint" class="truncate text-[10px] text-[var(--dt-faint)]">
      {{ hint }}
    </div>
  </div>
</template>
