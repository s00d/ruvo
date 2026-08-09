<script setup lang="ts">
import { computed, type Component } from "vue";
import { Inbox } from "@lucide/vue";
import Icon from "./Icon.vue";

const props = defineProps<{
  title: string;
  hint?: string;
  icon?: Component;
}>();

const resolvedIcon = computed(() => props.icon ?? Inbox);
</script>

<template>
  <div
    class="flex flex-col items-center justify-center gap-2 px-4 py-10 text-center"
  >
    <div
      class="flex h-10 w-10 items-center justify-center rounded-full border border-[var(--dt-border)] bg-[var(--dt-surface)] text-[var(--dt-faint)]"
    >
      <Icon :icon="resolvedIcon" :size="18" />
    </div>
    <p class="m-0 text-[13px] font-medium text-[var(--dt-muted)]">{{ title }}</p>
    <p v-if="hint" class="m-0 max-w-sm text-[11px] text-[var(--dt-faint)]">
      {{ hint }}
    </p>
  </div>
</template>
