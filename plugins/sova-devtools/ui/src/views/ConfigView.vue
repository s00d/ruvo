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
      mounts?: Record<string, Record<string, unknown>>;
    } | null,
);
const profile = computed(() => cfg.value?.profile ?? "—");
const features = computed(() => cfg.value?.features ?? []);
const hasI18n = computed(() => features.value.includes("i18n"));
const mounts = computed(() => cfg.value?.mounts ?? {});
const graphqlMount = computed(
  () => mounts.value.graphql as Record<string, unknown> | undefined,
);
const grpcMount = computed(
  () => mounts.value.grpc as Record<string, unknown> | undefined,
);
const rabbitMount = computed(
  () => mounts.value.rabbit as Record<string, unknown> | undefined,
);
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
      <Pane
        v-if="hasI18n"
        title="Internationalization"
        :icon="FileJson"
        hint="sova-i18n middleware"
      >
        <p class="m-0 text-[11px] leading-relaxed text-[var(--dt-muted)]">
          When <code class="dt-mono">devtools-i18n</code> is enabled, resolved
          <code class="dt-mono">locale</code> is captured on each request snapshot
          (Request tab → locale).
        </p>
      </Pane>
      <Pane
        v-if="graphqlMount"
        title="GraphQL server"
        :icon="FileJson"
        hint="Mounted paths from sova-graphql"
      >
        <div class="mb-2 flex flex-wrap gap-2">
          <Chip
            v-for="(val, key) in graphqlMount"
            :key="key"
            :label="String(key)"
            :value="val == null ? 'off' : String(val)"
            tone="info"
          />
        </div>
        <p class="m-0 text-[11px] leading-relaxed text-[var(--dt-muted)]">
          Open GraphiQL in the browser for ad-hoc queries; operations on POST
          <code class="dt-mono">{{ graphqlMount.api ?? '/graphql' }}</code>
          show up in the GraphQL tab.
        </p>
      </Pane>
      <Pane
        v-if="grpcMount"
        title="gRPC client"
        :icon="FileJson"
        hint="Connect-JSON from sova-grpc"
      >
        <div class="mb-2 flex flex-wrap gap-2">
          <Chip
            v-for="(val, key) in grpcMount"
            :key="key"
            :label="String(key)"
            :value="Array.isArray(val) ? val.join(', ') : val == null ? 'off' : String(val)"
            tone="info"
          />
        </div>
      </Pane>
      <Pane
        v-if="rabbitMount"
        title="RabbitMQ"
        :icon="FileJson"
        hint="sova-rabbit broker"
      >
        <div class="mb-2 flex flex-wrap gap-2">
          <Chip
            v-for="(val, key) in rabbitMount"
            :key="key"
            :label="String(key)"
            :value="val == null ? 'off' : String(val)"
            tone="info"
          />
        </div>
      </Pane>
      <CodeBlock :code="raw" title="config" language="json" />
    </Pane>
  </div>
</template>
