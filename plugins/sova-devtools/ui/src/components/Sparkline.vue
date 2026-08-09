<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    values: number[];
    width?: number;
    height?: number;
    stroke?: string;
  }>(),
  {
    width: 96,
    height: 22,
    stroke: "var(--dt-accent)",
  },
);

const path = computed(() => {
  const vals = props.values.length ? props.values : [0];
  const max = Math.max(...vals, 1);
  const min = Math.min(...vals, 0);
  const span = Math.max(max - min, 1e-6);
  const n = vals.length;
  const w = props.width;
  const h = props.height;
  const pad = 2;
  return vals
    .map((v, i) => {
      const x = n === 1 ? w / 2 : (i / (n - 1)) * (w - pad * 2) + pad;
      const y = h - pad - ((v - min) / span) * (h - pad * 2);
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
});
</script>

<template>
  <svg
    :width="width"
    :height="height"
    :viewBox="`0 0 ${width} ${height}`"
    class="block overflow-visible"
    aria-hidden="true"
  >
    <path
      class="dt-spark-path"
      :d="path"
      fill="none"
      :stroke="stroke"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</template>
