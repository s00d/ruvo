<script setup lang="ts">
import { computed } from "vue";

export type BarItem = {
  label: string;
  value: number;
  hint?: string;
  tone?: "ok" | "warn" | "err" | "default";
};

const props = withDefaults(
  defineProps<{
    items: BarItem[];
    unit?: string;
    max?: number;
  }>(),
  { unit: "ms" },
);

const maxVal = computed(() => props.max ?? Math.max(...props.items.map((i) => i.value), 1));
</script>

<template>
  <div v-if="!items.length" class="text-[11px] text-[var(--dt-faint)]">—</div>
  <ul v-else class="m-0 flex list-none flex-col gap-2 p-0">
    <li v-for="(it, i) in items" :key="i" class="flex flex-col gap-1">
      <div class="flex items-baseline justify-between gap-2">
        <span class="min-w-0 truncate text-[11px] text-[var(--dt-muted)]" :title="it.label">{{
          it.label
        }}</span>
        <span
          class="dt-mono shrink-0 text-[11px]"
          :class="{
            'text-[var(--dt-text)]': !it.tone || it.tone === 'default',
            'text-[var(--dt-ok)]': it.tone === 'ok',
            'text-[var(--dt-warn)]': it.tone === 'warn',
            'text-[var(--dt-err)]': it.tone === 'err',
          }"
          >{{ it.value.toFixed(1) }}{{ unit }}</span
        >
      </div>
      <div class="h-1.5 overflow-hidden rounded-sm bg-[var(--dt-bg)]">
        <div
          class="h-full rounded-sm transition-[width] duration-300"
          :class="{
            'bg-[var(--dt-accent)]': !it.tone || it.tone === 'default' || it.tone === 'ok',
            'bg-[var(--dt-warn)]': it.tone === 'warn',
            'bg-[var(--dt-err)]': it.tone === 'err',
          }"
          :style="{ width: Math.min(100, (it.value / maxVal) * 100) + '%' }"
        />
      </div>
      <span v-if="it.hint" class="truncate text-[10px] text-[var(--dt-faint)]">{{
        it.hint
      }}</span>
    </li>
  </ul>
</template>
