<script setup lang="ts">
import { computed } from "vue";
import { Mail } from "@lucide/vue";
import DefList from "../components/DefList.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const items = computed(() => store.current?.mail ?? []);
</script>

<template>
  <EmptyState
    v-if="!store.current"
    title="No snapshot"
    hint="Select a request first."
    :icon="Mail"
  />
  <EmptyState
    v-else-if="!items.length"
    title="No mail recorded"
    hint="Fake/SMTP sends on this request show up here."
    :icon="Mail"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane
      v-for="(m, i) in items"
      :key="i"
      :title="m.subject || '(no subject)'"
      :icon="Mail"
      :hint="m.backend"
    >
      <DefList
        :items="[
          { label: 'to', value: (m.to || []).join(', ') || '—' },
          { label: 'backend', value: m.backend },
          { label: 'subject', value: m.subject, mono: false },
        ]"
      />
    </Pane>
  </div>
</template>
