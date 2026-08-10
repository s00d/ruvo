import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { postAction, type ActionResponse } from "../api";
import { statusTone } from "../types";

export type BodyMode = "none" | "json" | "raw";
export type HttpTarget = "app" | "external";

export interface KeyValue {
  key: string;
  value: string;
  enabled: boolean;
}

export interface HttpHistoryEntry {
  method: string;
  path: string;
  target: HttpTarget;
  at_ms: number;
}

export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body: string;
  truncated: boolean;
  duration_ms: number;
}

const STORAGE_KEY = "sova.devtools.playground.v1";
const HISTORY_KEY = "sova.devtools.playground.history";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

function emptyRow(): KeyValue {
  return { key: "", value: "", enabled: true };
}

function loadState() {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    return null;
  }
}

function loadHistory(): HttpHistoryEntry[] {
  try {
    const raw = sessionStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw) as HttpHistoryEntry[];
    return Array.isArray(arr) ? arr.slice(0, 10) : [];
  } catch {
    return [];
  }
}

function rowsFromRecord(rec: Record<string, string> | undefined): KeyValue[] {
  if (!rec || !Object.keys(rec).length) return [emptyRow()];
  return Object.entries(rec).map(([key, value]) => ({
    key,
    value,
    enabled: true,
  }));
}

function recordFromRows(rows: KeyValue[]): Record<string, string> {
  const out: Record<string, string> = {};
  for (const r of rows) {
    if (!r.enabled || !r.key.trim()) continue;
    out[r.key.trim()] = r.value;
  }
  return out;
}

function splitPathQuery(path: string): { path: string; query: KeyValue[] } {
  const q = path.indexOf("?");
  if (q < 0) return { path, query: [emptyRow()] };
  const base = path.slice(0, q);
  const qs = path.slice(q + 1);
  const params = qs.split("&").map((pair) => {
    const [k, ...rest] = pair.split("=");
    return {
      key: decodeURIComponent(k ?? ""),
      value: decodeURIComponent(rest.join("=")),
      enabled: true,
    };
  });
  return { path: base || "/", query: params.length ? params : [emptyRow()] };
}

function buildPath(base: string, query: KeyValue[]): string {
  const pairs = query
    .filter((r) => r.enabled && r.key.trim())
    .map(
      (r) =>
        `${encodeURIComponent(r.key.trim())}=${encodeURIComponent(r.value)}`,
    );
  if (!pairs.length) return base;
  return `${base}?${pairs.join("&")}`;
}

export const usePlaygroundStore = defineStore("playground", () => {
  const saved = loadState();

  const target = ref<HttpTarget>(
    saved?.target === "external" ? "external" : "app",
  );
  const method = ref<string>((saved?.method as string) || "GET");
  const path = ref<string>((saved?.path as string) || "/");
  const queryParams = ref<KeyValue[]>(
    rowsFromRecord(saved?.query as Record<string, string> | undefined),
  );
  const headers = ref<KeyValue[]>(
    rowsFromRecord(saved?.headers as Record<string, string> | undefined),
  );
  const bodyMode = ref<BodyMode>(
    saved?.bodyMode === "json" || saved?.bodyMode === "raw"
      ? (saved.bodyMode as BodyMode)
      : "none",
  );
  const body = ref<string>((saved?.body as string) || "");
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastAction = ref<ActionResponse | null>(null);
  const response = ref<HttpResponse | null>(
    (saved?.response as HttpResponse | null) ?? null,
  );
  const history = ref<HttpHistoryEntry[]>(loadHistory());
  const section = ref<"query" | "headers" | "body">("body");

  const responseTone = computed(() =>
    response.value ? statusTone(response.value.status) : "ok",
  );

  const responseBodyPretty = computed(() => {
    if (!response.value?.body) return "";
    if (bodyMode.value === "json" || looksJson(response.value.body)) {
      try {
        return JSON.stringify(JSON.parse(response.value.body), null, 2);
      } catch {
        /* fall through */
      }
    }
    return response.value.body;
  });

  function looksJson(s: string): boolean {
    const t = s.trim();
    return t.startsWith("{") || t.startsWith("[");
  }

  function persist() {
    try {
      sessionStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          target: target.value,
          method: method.value,
          path: path.value,
          query: recordFromRows(queryParams.value),
          headers: recordFromRows(headers.value),
          bodyMode: bodyMode.value,
          body: body.value,
          response: response.value,
        }),
      );
    } catch {
      /* ignore */
    }
  }

  function pushHistory() {
    const entry: HttpHistoryEntry = {
      method: method.value,
      path: buildPath(path.value, queryParams.value),
      target: target.value,
      at_ms: Date.now(),
    };
    history.value = [
      entry,
      ...history.value.filter(
        (h) => h.method !== entry.method || h.path !== entry.path,
      ),
    ].slice(0, 10);
    try {
      sessionStorage.setItem(HISTORY_KEY, JSON.stringify(history.value));
    } catch {
      /* ignore */
    }
  }

  function prefill(input: {
    method?: string;
    path?: string;
    target?: HttpTarget;
    body?: string;
    headers?: Record<string, string>;
  }) {
    if (input.method) method.value = input.method.toUpperCase();
    if (input.path) {
      const split = splitPathQuery(input.path);
      path.value = split.path;
      queryParams.value = split.query;
    }
    if (input.target) target.value = input.target;
    if (input.headers) headers.value = rowsFromRecord(input.headers);
    if (input.body != null) {
      body.value = input.body;
      bodyMode.value = looksJson(input.body) ? "json" : "raw";
    }
    persist();
  }

  function restoreHistory(entry: HttpHistoryEntry) {
    prefill({
      method: entry.method,
      path: entry.path,
      target: entry.target,
    });
  }

  function addHeader() {
    headers.value = [...headers.value, emptyRow()];
  }

  function addQueryParam() {
    queryParams.value = [...queryParams.value, emptyRow()];
  }

  function applyJsonBodyPreset() {
    bodyMode.value = "json";
    const hasCt = headers.value.some(
      (h) => h.enabled && h.key.toLowerCase() === "content-type",
    );
    if (!hasCt) {
      headers.value = [
        ...headers.value.filter((h) => h.key.trim()),
        { key: "content-type", value: "application/json", enabled: true },
      ];
    }
  }

  function formatJsonBody() {
    try {
      body.value = JSON.stringify(JSON.parse(body.value), null, 2);
      error.value = null;
    } catch {
      error.value = "Invalid JSON in body";
    }
  }

  async function send(api: string) {
    loading.value = true;
    error.value = null;
    persist();

    if (bodyMode.value === "json" && body.value.trim()) {
      try {
        JSON.parse(body.value);
      } catch {
        error.value = "Body must be valid JSON";
        loading.value = false;
        return;
      }
    }

    const hdrs = recordFromRows(headers.value);
    const query = recordFromRows(queryParams.value);
    const payload: Record<string, unknown> = {
      target: target.value,
      method: method.value,
      path: path.value,
      headers: hdrs,
      query,
    };
    if (bodyMode.value !== "none" && body.value) {
      payload.body = body.value;
    }

    try {
      lastAction.value = await postAction(api, "http", payload);
      if (!lastAction.value.ok) {
        error.value = lastAction.value.error ?? "request failed";
        response.value = null;
        return;
      }
      const r = lastAction.value.result as {
        status?: number;
        headers?: Record<string, string>;
        body?: string;
        truncated?: boolean;
        duration_ms?: number;
      };
      response.value = {
        status: r.status ?? 0,
        headers: r.headers ?? {},
        body: r.body ?? "",
        truncated: Boolean(r.truncated),
        duration_ms: r.duration_ms ?? lastAction.value.duration_ms,
      };
      pushHistory();
      persist();
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    methods: METHODS,
    target,
    method,
    path,
    queryParams,
    headers,
    bodyMode,
    body,
    section,
    loading,
    error,
    response,
    responseTone,
    responseBodyPretty,
    history,
    prefill,
    restoreHistory,
    addHeader,
    addQueryParam,
    applyJsonBodyPreset,
    formatJsonBody,
    send,
    persist,
  };
});
