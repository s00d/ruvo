import type {
  CustomEvent,
  LogLine,
  MemorySample,
  MemorySummary,
  RequestMeta,
  RequestSnapshot,
} from "./types";
import { ensureCsrfHeaders } from "./csrf";

const fetchInit: RequestInit = { credentials: "same-origin" };

export async function fetchSnap(
  api: string,
  id: string,
): Promise<RequestSnapshot | null> {
  if (!id) return null;
  const r = await fetch(`${api}/requests/${encodeURIComponent(id)}`, fetchInit);
  if (!r.ok) return null;
  return (await r.json()) as RequestSnapshot;
}

export async function fetchTimeline(api: string): Promise<RequestMeta[]> {
  const r = await fetch(`${api}/requests`, fetchInit);
  if (!r.ok) return [];
  return (await r.json()) as RequestMeta[];
}

export async function fetchConfig(api: string): Promise<unknown> {
  const r = await fetch(`${api}/config`, fetchInit);
  if (!r.ok) return {};
  return r.json();
}

export async function fetchLogs(api: string): Promise<LogLine[]> {
  const r = await fetch(`${api}/logs`, fetchInit);
  if (!r.ok) return [];
  return (await r.json()) as LogLine[];
}

export async function fetchCustomEvents(api: string): Promise<CustomEvent[]> {
  const r = await fetch(`${api}/events/custom`, fetchInit);
  if (!r.ok) return [];
  return (await r.json()) as CustomEvent[];
}

export async function fetchMemory(api: string): Promise<MemorySummary> {
  const empty: MemorySummary = {
    samples: [],
    current: null,
    peak: null,
    min: null,
  };
  const r = await fetch(`${api}/memory`, fetchInit);
  if (!r.ok) return empty;
  const body = await r.json();
  // Compat: older servers returned a bare sample array.
  if (Array.isArray(body)) {
    const samples = body as MemorySample[];
    const rss = samples
      .map((s) => s.rss_bytes)
      .filter((v): v is number => v != null);
    return {
      samples,
      current: rss[0] ?? null,
      peak: rss.length ? Math.max(...rss) : null,
      min: rss.length ? Math.min(...rss) : null,
    };
  }
  const s = body as MemorySummary;
  return {
    samples: s.samples ?? [],
    current: s.current ?? null,
    peak: s.peak ?? null,
    min: s.min ?? null,
  };
}

export interface ActionResponse {
  ok: boolean;
  result?: unknown;
  duration_ms: number;
  error?: string | null;
}

export async function postAction(
  api: string,
  domain: string,
  payload: unknown,
): Promise<ActionResponse> {
  const csrf = await ensureCsrfHeaders(api);
  const r = await fetch(`${api}/actions/${domain}`, {
    method: "POST",
    credentials: "same-origin",
    headers: {
      "content-type": "application/json",
      ...csrf,
    },
    body: JSON.stringify(payload),
  });
  if (!r.ok) {
    const text = await r.text();
    try {
      return JSON.parse(text) as ActionResponse;
    } catch {
      return { ok: false, duration_ms: 0, error: text || r.statusText };
    }
  }
  return (await r.json()) as ActionResponse;
}
