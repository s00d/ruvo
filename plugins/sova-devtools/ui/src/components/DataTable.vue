<script setup lang="ts">
defineProps<{
  headers: string[];
  rows: unknown[][];
  monoCols?: number[];
}>();

async function copyCell(v: unknown) {
  try {
    await navigator.clipboard.writeText(String(v ?? ""));
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <p v-if="!rows.length" class="m-0 text-[var(--dt-faint)]">— empty —</p>
  <div v-else class="overflow-auto rounded-[var(--dt-radius)] border border-[var(--dt-border)]">
    <table class="w-full border-collapse text-left">
      <thead>
        <tr class="bg-[var(--dt-surface-2)]">
          <th
            v-for="h in headers"
            :key="h"
            class="sticky top-0 px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--dt-faint)]"
          >
            {{ h }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="(row, i) in rows"
          :key="i"
          class="border-t border-[var(--dt-border)] transition-colors hover:bg-[var(--dt-surface)]"
          :class="i % 2 === 1 ? 'bg-[color-mix(in_srgb,var(--dt-surface)_45%,transparent)]' : ''"
        >
          <td
            v-for="(cell, j) in row"
            :key="j"
            class="max-w-[28rem] truncate px-2.5 py-1.5 align-top text-[12px]"
            :class="monoCols?.includes(j) ? 'dt-mono text-[11px]' : ''"
            :title="String(cell ?? '')"
            @dblclick="copyCell(cell)"
          >
            <slot name="cell" :cell="cell" :col="j" :row="row">{{ cell }}</slot>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
