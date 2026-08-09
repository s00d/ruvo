<script setup lang="ts">
import type { Component } from "vue";
import Icon from "./Icon.vue";

defineProps<{
  label?: string;
  value: string;
  icon?: Component;
  tone?: "default" | "ok" | "warn" | "err" | "info";
  active?: boolean;
  compact?: boolean;
}>();

defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <button
    type="button"
    class="inline-flex min-h-[var(--dt-touch)] items-center gap-1.5 border border-solid px-2.5 text-left transition-colors"
    :class="[
      compact ? 'rounded-[4px] py-1' : 'rounded-[var(--dt-radius)] py-1.5',
      active
        ? 'border-[var(--dt-accent)] bg-[var(--dt-accent-dim)]'
        : 'border-[var(--dt-border)] bg-[var(--dt-surface)] hover:border-[var(--dt-border-strong)] hover:bg-[var(--dt-surface-2)]',
    ]"
    :style="{
      borderLeftWidth: '3px',
      borderLeftColor:
        tone === 'ok'
          ? 'var(--dt-ok)'
          : tone === 'warn'
            ? 'var(--dt-warn)'
            : tone === 'err'
              ? 'var(--dt-err)'
              : tone === 'info'
                ? 'var(--dt-info)'
                : 'var(--dt-border-strong)',
    }"
    @click="$emit('click', $event)"
  >
    <Icon v-if="icon" :icon="icon" :size="13" class="text-[var(--dt-muted)]" />
    <span
      v-if="label"
      class="text-[10px] uppercase tracking-wider text-[var(--dt-faint)]"
      >{{ label }}</span
    >
    <span
      class="dt-mono text-[12px] font-medium"
      :class="{
        'text-[var(--dt-text)]': !tone || tone === 'default',
        'text-[var(--dt-ok)]': tone === 'ok',
        'text-[var(--dt-warn)]': tone === 'warn',
        'text-[var(--dt-err)]': tone === 'err',
        'text-[var(--dt-info)]': tone === 'info',
      }"
      >{{ value }}</span
    >
  </button>
</template>
