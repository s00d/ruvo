<script setup lang="ts">
import { computed } from "vue";
import { FileJson } from "@lucide/vue";
import CodeBlock from "../components/CodeBlock.vue";
import Chip from "../components/Chip.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const cfg = computed(
  () =>
    store.config as {
      profile?: string;
      plugins?: string[];
      features?: string[];
    } | null,
);
const profile = computed(() => cfg.value?.profile ?? "—");
const features = computed(() => cfg.value?.features ?? []);
const raw = computed(() => JSON.stringify(store.config ?? {}, null, 2));
</script>

<template>
  <EmptyState
    v-if="store.config == null && !store.isPlayground"
    title="No config payload"
    hint="Hub config endpoint returned empty."
    :icon="FileJson"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane title="Runtime" :icon="FileJson" :hint="profile">
      <div class="mb-3 flex flex-wrap gap-2">
        <Chip label="profile" :value="profile" tone="info" />
        <Chip
          v-for="f in features"
          :key="f"
          label="feature"
          :value="f"
          tone="ok"
        />
        <span
          v-if="!features.length"
          class="text-[12px] text-[var(--dt-faint)]"
          >no optional DevTools features compiled in</span
        >
      </div>
      <CodeBlock :code="raw" title="config" language="json" />
    </Pane>
  </div>
</template>
