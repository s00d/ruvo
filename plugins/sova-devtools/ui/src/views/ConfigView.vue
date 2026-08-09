<script setup lang="ts">
import { computed } from "vue";
import { FileJson } from "@lucide/vue";
import CodeBlock from "../components/CodeBlock.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const profile = computed(() => {
  const c = store.config as { profile?: string } | null;
  return c?.profile ?? "—";
});
const raw = computed(() => JSON.stringify(store.config ?? {}, null, 2));
</script>

<template>
  <EmptyState
    v-if="store.config == null && !store.isPlayground"
    title="No config payload"
    hint="Hub config endpoint returned empty."
    :icon="FileJson"
  />
  <Pane v-else title="Runtime config" :icon="FileJson" :hint="profile">
    <CodeBlock :code="raw" title="config" language="json" />
  </Pane>
</template>
