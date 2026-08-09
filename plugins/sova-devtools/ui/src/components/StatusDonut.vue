<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    ok: number;
    warn: number;
    err: number;
    size?: number;
  }>(),
  { size: 72 },
);

const total = computed(() => Math.max(props.ok + props.warn + props.err, 0));
const r = computed(() => props.size / 2 - 4);
const c = computed(() => props.size / 2);
const circ = computed(() => 2 * Math.PI * r.value);

const segs = computed(() => {
  const t = total.value || 1;
  let offset = 0;
  const mk = (n: number, color: string) => {
    const len = (n / t) * circ.value;
    const o = offset;
    offset += len;
    return { color, dash: `${len} ${circ.value - len}`, offset: -o };
  };
  return [
    mk(props.ok, "var(--dt-ok)"),
    mk(props.warn, "var(--dt-warn)"),
    mk(props.err, "var(--dt-err)"),
  ];
});
</script>

<template>
  <div class="flex items-center gap-3">
    <svg :width="size" :height="size" class="shrink-0" aria-hidden="true">
      <circle
        :cx="c"
        :cy="c"
        :r="r"
        fill="none"
        stroke="var(--dt-border)"
        stroke-width="6"
      />
      <circle
        v-for="(s, i) in segs"
        :key="i"
        :cx="c"
        :cy="c"
        :r="r"
        fill="none"
        :stroke="s.color"
        stroke-width="6"
        stroke-linecap="butt"
        :stroke-dasharray="s.dash"
        :stroke-dashoffset="s.offset"
        transform-origin="center"
        :style="{ transform: `rotate(-90deg)`, transformOrigin: `${c}px ${c}px` }"
      />
      <text
        :x="c"
        :y="c"
        text-anchor="middle"
        dominant-baseline="central"
        class="dt-mono"
        fill="var(--dt-text)"
        font-size="12"
        font-weight="500"
      >
        {{ total }}
      </text>
    </svg>
    <div class="flex flex-col gap-1 text-[11px] text-[var(--dt-muted)]">
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-sm bg-[var(--dt-ok)]" />
        2xx <span class="dt-mono text-[var(--dt-text)]">{{ ok }}</span>
      </div>
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-sm bg-[var(--dt-warn)]" />
        4xx <span class="dt-mono text-[var(--dt-text)]">{{ warn }}</span>
      </div>
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-sm bg-[var(--dt-err)]" />
        5xx <span class="dt-mono text-[var(--dt-text)]">{{ err }}</span>
      </div>
    </div>
  </div>
</template>
