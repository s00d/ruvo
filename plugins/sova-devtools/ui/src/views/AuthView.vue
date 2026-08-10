<script setup lang="ts">
import { computed } from "vue";
import { KeyRound } from "@lucide/vue";
import DefList from "../components/DefList.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SessionConsole from "../components/SessionConsole.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const auth = computed(() => store.current?.auth ?? null);
const keys = computed(() =>
  (auth.value?.session_keys ?? []).map(([k, v]) => ({
    label: k,
    value: v,
  })),
);
const roles = computed(() => (auth.value?.roles ?? []).join(", ") || "—");

const hasConsole = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-session") ?? false;
});
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasConsole" title="Session console" :icon="KeyRound">
      <SessionConsole />
    </Pane>

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request to see session/auth state from that request."
      :icon="KeyRound"
    />
    <template v-else>
      <Pane title="Identity (snapshot)" :icon="KeyRound">
        <DefList
          :items="[
            { label: 'session', value: auth?.session_id || '—' },
            { label: 'user', value: auth?.user_id || '—' },
            { label: 'email', value: auth?.email || '—' },
            { label: 'roles', value: roles },
          ]"
        />
      </Pane>
      <EmptyState
        v-if="!keys.length"
        title="No session keys in snapshot"
        hint="Session bag was empty or not captured for this request."
        :icon="KeyRound"
      />
      <Pane v-else title="Session keys (snapshot)" hint="redacted · read-only">
        <DefList :items="keys" />
      </Pane>
    </template>
  </div>
</template>
