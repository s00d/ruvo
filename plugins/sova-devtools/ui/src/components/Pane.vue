<script setup lang="ts">
import type { Component } from "vue";
import Icon from "./Icon.vue";

defineProps<{
  title?: string;
  icon?: Component;
  hint?: string;
  tone?: "default" | "warn" | "err";
}>();
</script>

<template>
  <section
    class="overflow-hidden rounded-[var(--dt-radius)] border bg-[var(--dt-surface)]"
    :class="{
      'border-[var(--dt-border)]': !tone || tone === 'default',
      'border-[var(--dt-warn)]': tone === 'warn',
      'border-[var(--dt-err)]': tone === 'err',
    }"
  >
    <header
      v-if="title || $slots.actions"
      class="flex min-h-9 items-center gap-2 border-b border-[var(--dt-border)] bg-[var(--dt-surface-2)] px-3 py-2"
    >
      <Icon v-if="icon" :icon="icon" :size="14" class="text-[var(--dt-accent)]" />
      <h3 v-if="title" class="m-0 text-[12px] font-semibold tracking-wide">
        {{ title }}
      </h3>
      <span v-if="hint" class="text-[11px] text-[var(--dt-faint)]">{{ hint }}</span>
      <div class="ml-auto flex items-center gap-2">
        <slot name="actions" />
      </div>
    </header>
    <div class="p-3">
      <slot />
    </div>
  </section>
</template>
