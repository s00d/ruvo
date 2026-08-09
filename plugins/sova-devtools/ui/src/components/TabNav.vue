<script setup lang="ts">
import { useRouter } from "vue-router";
import Icon from "./Icon.vue";
import { TAB_META } from "../tabs";
import { useDevToolsStore } from "../stores/devtools";
import type { TabId } from "../types";

const store = useDevToolsStore();
const router = useRouter();

function go(id: TabId) {
  store.setTab(id);
  void router.push({ name: id });
}
</script>

<template>
  <nav
    class="dt-scroll flex shrink-0 items-stretch gap-0 overflow-x-auto scroll-smooth border-b border-[var(--dt-border)] bg-[var(--dt-surface)] px-1 snap-x"
    role="tablist"
    aria-label="DevTools tabs"
  >
    <button
      v-for="t in TAB_META"
      :key="t.id"
      type="button"
      role="tab"
      :aria-selected="store.tab === t.id"
      class="relative inline-flex min-h-[var(--dt-touch)] snap-start cursor-pointer items-center gap-1.5 border-0 border-b-2 bg-transparent px-3 text-[12px] transition-colors"
      :class="
        store.tab === t.id
          ? 'border-[var(--dt-accent)] text-[var(--dt-text)]'
          : 'border-transparent text-[var(--dt-muted)] hover:text-[var(--dt-text)]'
      "
      @click.stop="go(t.id)"
    >
      <Icon
        :icon="t.icon"
        :size="13"
        :class="
          store.tab === t.id ? 'text-[var(--dt-accent)]' : 'text-current'
        "
      />
      <span class="whitespace-nowrap">{{ t.label }}</span>
      <span
        v-if="store.tabBadges[t.id]"
        class="dt-mono rounded px-1 text-[10px]"
        :class="
          store.tab === t.id
            ? 'bg-[var(--dt-accent-dim)] text-[var(--dt-accent)]'
            : 'bg-[var(--dt-surface-2)] text-[var(--dt-faint)]'
        "
        >{{ store.tabBadges[t.id] }}</span
      >
    </button>
  </nav>
</template>
