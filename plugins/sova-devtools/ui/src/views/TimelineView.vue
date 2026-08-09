<script setup lang="ts">
import { ListTree } from "@lucide/vue";
import Pane from "../components/Pane.vue";
import Sparkline from "../components/Sparkline.vue";
import StatusDonut from "../components/StatusDonut.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="grid gap-3 sm:grid-cols-2">
      <Pane title="Latency sparkline" :icon="ListTree" hint="recent">
        <div class="flex items-center gap-3">
          <Sparkline :values="store.sparkFromTimeline" :width="200" :height="40" />
          <span class="text-[11px] text-[var(--dt-faint)]"
            >{{ store.timeline.length }} samples</span
          >
        </div>
      </Pane>
      <Pane title="Status mix" hint="last 50">
        <StatusDonut
          :ok="store.statusBuckets.ok"
          :warn="store.statusBuckets.warn"
          :err="store.statusBuckets.err"
        />
      </Pane>
    </div>
    <Pane title="Hint">
      <p class="m-0 text-[12px] text-[var(--dt-muted)]">
        Use the request list on the left to open a snapshot. Live SSE updates
        appear there in the running app.
      </p>
    </Pane>
  </div>
</template>
