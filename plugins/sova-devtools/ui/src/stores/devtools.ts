import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { fetchConfig, fetchLogs, fetchSnap, fetchTimeline } from "../api";
import {
  CHANNEL,
  loadPanelH,
  loadSpark,
  loadUi,
  patchUi,
  pushSpark,
  savePanelH,
  PANEL_H_MSG,
  type BusMsg,
  type PageMsg,
} from "../persist";
import type {
  CustomEvent,
  LogLine,
  MemorySample,
  MountAttrs,
  RequestMeta,
  RequestSnapshot,
  TabId,
} from "../types";
import { statusTone, sumMs } from "../types";

const FS_KEY = "sova.devtools.fs";

function readMount(): MountAttrs {
  const el = document.getElementById("sova-devtools");
  const ds = el?.dataset ?? {};
  const statusClass = (ds.statusClass as MountAttrs["statusClass"]) || "ok";
  const params = new URLSearchParams(location.search);
  const embed = params.get("embed") === "1" || window !== window.top;
  const shell =
    ds.shell === "1" ||
    location.pathname.startsWith("/_devtools/app") ||
    ds.playground === "1";
  return {
    snap: ds.snap || "",
    status: Number(ds.status || 0),
    statusClass:
      statusClass === "warn" || statusClass === "err" ? statusClass : "ok",
    ms: Number(ds.ms || 0),
    sql: Number(ds.sql || 0),
    errors: Number(ds.errors || 0),
    api: ds.api || "/_devtools",
    events: ds.events || "/_devtools/events",
    shell,
    embed,
  };
}

function applyPage(
  page: PageMsg,
  mount: { value: MountAttrs },
  snapId: { value: string },
  spark: { value: number[] },
) {
  mount.value = {
    ...mount.value,
    snap: page.snap,
    status: page.status,
    statusClass: page.statusClass,
    ms: page.ms,
    sql: page.sql,
    errors: page.errors,
  };
  if (page.snap) snapId.value = page.snap;
  if (page.ms > 0) spark.value = pushSpark(page.ms);
}

export const useDevToolsStore = defineStore("devtools", () => {
  const saved = loadUi();
  const mount = ref(readMount());
  const playground = ref(
    document.getElementById("sova-devtools")?.dataset.playground === "1",
  );
  const open = ref(
    mount.value.embed || playground.value
      ? true
      : saved.open || mount.value.shell,
  );
  const tab = ref<TabId>(saved.tab);
  const snapId = ref(mount.value.snap);
  const current = ref<RequestSnapshot | null>(null);
  const timeline = ref<RequestMeta[]>([]);
  const globalLogs = ref<LogLine[]>([]);
  const config = ref<unknown>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const spark = ref<number[]>(loadSpark());
  const panelH = ref(loadPanelH());
  const fullscreen = ref(
    (() => {
      try {
        return sessionStorage.getItem(FS_KEY) === "1";
      } catch {
        return false;
      }
    })(),
  );

  const api = computed(() => mount.value.api);
  const isEmbed = computed(() => mount.value.embed);
  const isShell = computed(() => mount.value.shell);
  const isPlayground = computed(() => playground.value);

  const sqlTotalMs = computed(() => sumMs(current.value?.queries));
  const httpTotalMs = computed(() => sumMs(current.value?.http));
  const cacheTotalMs = computed(() => sumMs(current.value?.cache));
  const logErrorCount = computed(
    () =>
      current.value?.logs.filter((l) =>
        String(l.level).toUpperCase().includes("ERROR"),
      ).length ?? 0,
  );

  const statusBuckets = computed(() => {
    let ok = 0;
    let warn = 0;
    let err = 0;
    for (const m of timeline.value.slice(0, 50)) {
      const t = statusTone(m.status);
      if (t === "ok") ok++;
      else if (t === "warn") warn++;
      else err++;
    }
    return { ok, warn, err };
  });

  const sparkFromTimeline = computed(() => {
    const fromTl = timeline.value
      .slice(0, 40)
      .map((m) => m.duration_ms)
      .reverse();
    return fromTl.length ? fromTl : spark.value;
  });

  const tabBadges = computed(() => {
    const c = current.value;
    return {
      request: 0,
      timeline: timeline.value.length,
      db: c?.queries.length ?? 0,
      cache: c?.cache?.length ?? 0,
      logs: c?.logs.length || globalLogs.value.length,
      http: c?.http.length ?? 0,
      mail: c?.mail.length ?? 0,
      jobs: c?.jobs.length ?? 0,
      auth: c?.auth?.session_keys?.length ?? 0,
      config: 0,
    } as Record<TabId, number>;
  });

  const effectivePanelH = computed(() => {
    if (fullscreen.value) return Math.max(320, window.innerHeight - 40);
    return panelH.value;
  });

  function persist() {
    if (playground.value) return;
    patchUi({ open: open.value, tab: tab.value });
  }

  watch([open, tab], persist);

  function setPanelH(h: number) {
    const clamped = Math.min(800, Math.max(220, h));
    panelH.value = clamped;
    savePanelH(clamped);
    if (isEmbed.value && window.parent !== window) {
      try {
        window.parent.postMessage(
          { type: PANEL_H_MSG, height: clamped },
          "*",
        );
      } catch {
        /* ignore */
      }
    }
  }

  function toggleFullscreen() {
    fullscreen.value = !fullscreen.value;
    try {
      sessionStorage.setItem(FS_KEY, fullscreen.value ? "1" : "0");
    } catch {
      /* ignore */
    }
  }

  function toggle() {
    if (isEmbed.value || playground.value) return;
    open.value = !open.value;
    if (open.value) void refresh();
  }

  function close() {
    if (isEmbed.value || playground.value) return;
    open.value = false;
  }

  function setTab(next: TabId) {
    tab.value = next;
    void refresh();
  }

  function openSnap(id: string) {
    snapId.value = id;
    tab.value = "request";
    void refresh();
  }

  function pushTimelineMeta(meta: RequestMeta) {
    timeline.value = [meta, ...timeline.value].slice(0, 100);
    spark.value = pushSpark(meta.duration_ms);
  }

  function loadMock(data: {
    timeline: RequestMeta[];
    current: RequestSnapshot;
    logs: LogLine[];
    config: unknown;
  }) {
    timeline.value = data.timeline;
    current.value = data.current;
    snapId.value = data.current.id;
    globalLogs.value = data.logs;
    config.value = data.config;
    mount.value = {
      ...mount.value,
      snap: data.current.id,
      status: data.current.status,
      statusClass: statusTone(data.current.status),
      ms: data.current.duration_ms,
      sql: data.current.queries.length,
      errors: data.current.logs.filter((l) =>
        String(l.level).toUpperCase().includes("ERROR"),
      ).length,
    };
    spark.value = data.timeline.map((m) => m.duration_ms).reverse();
    loading.value = false;
  }

  async function refresh() {
    if (playground.value) return;
    if (!open.value && !isEmbed.value) return;
    loading.value = true;
    error.value = null;
    try {
      if (tab.value === "timeline") {
        timeline.value = await fetchTimeline(api.value);
      } else if (tab.value === "config") {
        config.value = await fetchConfig(api.value);
      } else if (tab.value === "logs" && !snapId.value) {
        globalLogs.value = await fetchLogs(api.value);
      } else if (snapId.value) {
        current.value = await fetchSnap(api.value, snapId.value);
      } else {
        const list = await fetchTimeline(api.value);
        timeline.value = list;
        if (list[0]) {
          snapId.value = list[0].id;
          current.value = await fetchSnap(api.value, list[0].id);
        }
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  function connectBus() {
    if (playground.value) return;
    try {
      const ch = new BroadcastChannel(CHANNEL);
      ch.addEventListener("message", (ev: MessageEvent<BusMsg>) => {
        const msg = ev.data;
        if (!msg || typeof msg !== "object") return;
        if (msg.type === "page") {
          applyPage(msg, mount, snapId, spark);
          if (open.value || isEmbed.value) void refresh();
        } else if (msg.type === "ui" && !isEmbed.value) {
          open.value = msg.open;
          if (msg.tab) tab.value = msg.tab;
          if (open.value) void refresh();
        }
      });
    } catch {
      /* ignore */
    }
  }

  const onCustomEvent = ref<((e: CustomEvent) => void) | null>(null);
  const onMemorySample = ref<((s: MemorySample) => void) | null>(null);

  let refreshTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleRefresh() {
    if (refreshTimer != null) return;
    refreshTimer = setTimeout(() => {
      refreshTimer = null;
      if (open.value || isEmbed.value) void refresh();
    }, 250);
  }

  function connectSse() {
    if (playground.value) return;
    try {
      const es = new EventSource(mount.value.events);
      es.addEventListener("request.finished", (ev) => {
        try {
          const msg = JSON.parse((ev as MessageEvent).data) as {
            meta?: RequestMeta;
          };
          if (msg.meta) {
            pushTimelineMeta(msg.meta);
            if ((open.value || isEmbed.value) && tab.value === "timeline") {
              scheduleRefresh();
            }
          }
        } catch {
          /* ignore */
        }
      });
      es.addEventListener("custom", (ev) => {
        try {
          const msg = JSON.parse((ev as MessageEvent).data) as {
            event?: CustomEvent;
          };
          if (msg.event) onCustomEvent.value?.(msg.event);
        } catch {
          /* ignore */
        }
      });
      es.addEventListener("memory.sample", (ev) => {
        try {
          const msg = JSON.parse((ev as MessageEvent).data) as {
            sample?: MemorySample;
          };
          if (msg.sample) onMemorySample.value?.(msg.sample);
        } catch {
          /* ignore */
        }
      });
    } catch {
      /* ignore */
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") close();
  }

  function boot() {
    if (mount.value.ms > 0 && !spark.value.length) {
      spark.value = pushSpark(mount.value.ms);
    }
    connectBus();
    connectSse();
    window.addEventListener("keydown", onKeydown);
    if (!playground.value) void refresh();
  }

  return {
    mount,
    open,
    tab,
    snapId,
    current,
    timeline,
    globalLogs,
    config,
    loading,
    error,
    spark,
    sparkFromTimeline,
    panelH,
    effectivePanelH,
    fullscreen,
    isEmbed,
    isShell,
    isPlayground,
    sqlTotalMs,
    httpTotalMs,
    cacheTotalMs,
    logErrorCount,
    statusBuckets,
    tabBadges,
    toggle,
    close,
    setTab,
    openSnap,
    refresh,
    setPanelH,
    toggleFullscreen,
    loadMock,
    boot,
    onCustomEvent,
    onMemorySample,
  };
});
