<script setup lang="ts">
import { computed } from "vue";
import { KeyRound } from "@lucide/vue";
import DefList from "../components/DefList.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const auth = computed(() => store.current?.auth ?? null);
const keys = computed(() =>
  (auth.value?.session_keys ?? []).map(([k, v]) => ({
    label: k,
    value: v,
  })),
);
</script>

<template>
  <EmptyState
    v-if="!store.current"
    title="No snapshot"
    hint="Select a request first."
    :icon="KeyRound"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane title="Identity" :icon="KeyRound">
      <DefList
        :items="[
          { label: 'session', value: auth?.session_id || '—' },
          { label: 'user', value: auth?.user_id || '—' },
        ]"
      />
    </Pane>
    <EmptyState
      v-if="!keys.length"
      title="No session keys"
      hint="Session bag empty or redacted."
      :icon="KeyRound"
    />
    <Pane v-else title="Session keys" hint="redacted values">
      <DefList :items="keys" />
    </Pane>
  </div>
</template>
