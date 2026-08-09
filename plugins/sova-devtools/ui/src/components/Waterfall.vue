<script setup lang="ts">
import { computed } from "vue";

export type WaterfallSeg = {
  label: string;
  ms: number;
  color?: string;
};

const props = defineProps<{
  totalMs: number;
  segments: WaterfallSeg[];
}>();

const rows = computed(() => {
  const total = Math.max(props.totalMs, 0.001);
  let offset = 0;
  return props.segments.map((s) => {
    const ms = Math.max(s.ms, 0);
    const left = (offset / total) * 100;
    const width = Math.min((ms / total) * 100, 100 - left);
    offset += ms;
    return { ...s, ms, left, width };
  });
});
</script>

<template>
  <div class="flex flex-col gap-2">
    <div class="flex items-baseline justify-between">
      <span class="text-[11px] text-[var(--dt-muted)]">Request waterfall</span>
      <span class="dt-mono text-[12px] text-[var(--dt-text)]"
        >{{ totalMs.toFixed(1) }}ms total</span
      >
    </div>
    <div
      class="relative h-3 overflow-hidden rounded-sm border border-[var(--dt-border)] bg-[var(--dt-bg)]"
    >
      <div
        v-for="(r, i) in rows"
        :key="i"
        class="absolute top-0 h-full"
        :style="{
          left: r.left + '%',
          width: Math.max(r.width, r.ms > 0 ? 0.4 : 0) + '%',
          background: r.color || 'var(--dt-accent)',
        }"
        :title="`${r.label}: ${r.ms.toFixed(1)}ms`"
      />
    </div>
    <div class="flex flex-wrap gap-x-4 gap-y-1">
      <div
        v-for="(r, i) in rows"
        :key="'l' + i"
        class="flex items-center gap-1.5 text-[11px] text-[var(--dt-muted)]"
      >
        <span
          class="inline-block h-2 w-2 rounded-sm"
          :style="{ background: r.color || 'var(--dt-accent)' }"
        />
        <span>{{ r.label }}</span>
        <span class="dt-mono text-[var(--dt-text)]">{{ r.ms.toFixed(1) }}ms</span>
      </div>
    </div>
  </div>
</template>
