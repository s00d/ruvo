<script setup lang="ts">
import { Search } from "@lucide/vue";
import Icon from "./Icon.vue";

const model = defineModel<string>({ default: "" });

defineProps<{
  placeholder?: string;
}>();
</script>

<template>
  <label
    class="flex min-h-[var(--dt-touch)] items-center gap-2 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2.5 focus-within:border-[var(--dt-accent)]"
  >
    <Icon :icon="Search" :size="14" class="text-[var(--dt-faint)]" />
    <input
      v-model="model"
      type="search"
      class="min-w-0 flex-1 border-0 bg-transparent py-2 text-[12px] text-[var(--dt-text)] outline-none placeholder:text-[var(--dt-faint)]"
      :placeholder="placeholder || 'Filter…'"
    />
  </label>
</template>
