<script setup lang="ts">
import { ref } from "vue";
import { Check, Copy } from "@lucide/vue";
import Icon from "./Icon.vue";

const props = withDefaults(
  defineProps<{
    code: string;
    title?: string;
    language?: string;
  }>(),
  { title: "code", language: "text" },
);

const copied = ref(false);

async function copy() {
  try {
    await navigator.clipboard.writeText(props.code);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 1200);
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <div
    class="overflow-hidden rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)]"
  >
    <div
      class="flex items-center gap-2 border-b border-[var(--dt-border)] bg-[var(--dt-surface-2)] px-3 py-1.5"
    >
      <span class="text-[10px] uppercase tracking-wider text-[var(--dt-faint)]">{{
        title
      }}</span>
      <span class="dt-mono text-[10px] text-[var(--dt-faint)]">{{ language }}</span>
      <button
        type="button"
        class="ml-auto inline-flex min-h-7 cursor-pointer items-center gap-1 rounded border border-[var(--dt-border)] bg-[var(--dt-surface)] px-2 text-[10px] text-[var(--dt-muted)] hover:text-[var(--dt-text)]"
        @click="copy"
      >
        <Icon :icon="copied ? Check : Copy" :size="12" />
        {{ copied ? "copied" : "copy" }}
      </button>
    </div>
    <pre
      class="dt-scroll m-0 max-h-48 overflow-auto p-3 dt-mono text-[11px] leading-relaxed text-[var(--dt-muted)] whitespace-pre-wrap break-all"
      >{{ code }}</pre
    >
  </div>
</template>
