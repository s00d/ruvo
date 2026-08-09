<script setup lang="ts">
withDefaults(
  defineProps<{
    rows?: number;
    cols?: number;
  }>(),
  { rows: 4, cols: 1 },
);
</script>

<template>
  <div class="flex flex-col gap-2 p-1">
    <div
      v-for="r in rows"
      :key="r"
      class="grid gap-2"
      :style="{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }"
    >
      <div
        v-for="c in cols"
        :key="c"
        class="h-7 animate-pulse rounded-[var(--dt-radius)] bg-[var(--dt-surface-2)]"
      />
    </div>
  </div>
</template>
