<script setup lang="ts">
import Icon from "./Icon.vue";
import { useDevToolsStore } from "../stores/devtools";
import type { TabId } from "../types";

const store = useDevToolsStore();

function go(id: TabId) {
  store.setTab(id);
}
</script>

<template>
  <nav
    class="dt-scroll flex h-full w-[min(100%,220px)] shrink-0 flex-col overflow-y-auto border-r border-[var(--dt-border)] bg-[var(--dt-surface)] py-2"
    role="tablist"
    aria-label="DevTools sections"
  >
    <section
      v-for="group in store.visibleTabGroups"
      :key="group.id"
      class="px-1"
    >
      <h2
        class="m-0 px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-[var(--dt-faint)]"
      >
        {{ group.label }}
      </h2>
      <button
        v-for="t in group.tabs"
        :key="t.id"
        type="button"
        role="tab"
        :aria-selected="store.tab === t.id"
        class="mb-0.5 flex w-full min-h-[var(--dt-touch)] cursor-pointer items-center gap-2 rounded-[var(--dt-radius)] border-0 px-2 text-left text-[12px] transition-colors"
        :class="
          store.tab === t.id
            ? 'bg-[var(--dt-accent-dim)] text-[var(--dt-text)]'
            : 'bg-transparent text-[var(--dt-muted)] hover:bg-[var(--dt-surface-2)] hover:text-[var(--dt-text)]'
        "
        @click.stop="go(t.id)"
      >
        <Icon
          :icon="t.icon"
          :size="14"
          :class="
            store.tab === t.id ? 'text-[var(--dt-accent)]' : 'text-current'
          "
        />
        <span class="min-w-0 flex-1 truncate">{{ t.label }}</span>
        <span
          v-if="store.tabBadges[t.id as TabId]"
          class="dt-mono shrink-0 rounded px-1 text-[10px]"
          :class="
            store.tab === t.id
              ? 'bg-[var(--dt-bg)] text-[var(--dt-accent)]'
              : 'bg-[var(--dt-surface-2)] text-[var(--dt-faint)]'
          "
          >{{ store.tabBadges[t.id as TabId] }}</span
        >
      </button>
    </section>
  </nav>
</template>
