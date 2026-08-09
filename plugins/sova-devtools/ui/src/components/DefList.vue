<script setup lang="ts">
export type DefItem = { label: string; value: string; mono?: boolean };

defineProps<{
  items: DefItem[];
}>();

async function copy(v: string) {
  try {
    await navigator.clipboard.writeText(v);
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <dl
    class="m-0 overflow-hidden rounded-[var(--dt-radius)] border border-[var(--dt-border)]"
  >
    <div
      v-for="(it, i) in items"
      :key="i"
      class="grid grid-cols-[7.5rem_1fr] border-b border-[var(--dt-border)] last:border-b-0 sm:grid-cols-[9rem_1fr]"
    >
      <dt
        class="border-r border-[var(--dt-border)] bg-[var(--dt-surface-2)] px-3 py-2 text-[11px] text-[var(--dt-faint)]"
      >
        {{ it.label }}
      </dt>
      <dd
        class="m-0 flex min-w-0 items-center gap-2 px-3 py-2 text-[12px]"
        :class="it.mono !== false ? 'dt-mono' : ''"
        :title="it.value"
      >
        <span class="min-w-0 flex-1 truncate">{{ it.value }}</span>
        <button
          type="button"
          class="shrink-0 cursor-pointer rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-1.5 py-0.5 text-[10px] text-[var(--dt-muted)] hover:text-[var(--dt-text)]"
          title="Copy"
          @click="copy(it.value)"
        >
          copy
        </button>
      </dd>
    </div>
  </dl>
</template>
