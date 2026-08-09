import type {
  CustomEvent,
  LogLine,
  MemorySample,
  RequestMeta,
  RequestSnapshot,
} from "./types";

export async function fetchSnap(
  api: string,
  id: string,
): Promise<RequestSnapshot | null> {
  if (!id) return null;
  const r = await fetch(`${api}/requests/${encodeURIComponent(id)}`);
  if (!r.ok) return null;
  return (await r.json()) as RequestSnapshot;
}

export async function fetchTimeline(api: string): Promise<RequestMeta[]> {
  const r = await fetch(`${api}/requests`);
  if (!r.ok) return [];
  return (await r.json()) as RequestMeta[];
}

export async function fetchConfig(api: string): Promise<unknown> {
  const r = await fetch(`${api}/config`);
  if (!r.ok) return {};
  return r.json();
}

export async function fetchLogs(api: string): Promise<LogLine[]> {
  const r = await fetch(`${api}/logs`);
  if (!r.ok) return [];
  return (await r.json()) as LogLine[];
}

export async function fetchCustomEvents(api: string): Promise<CustomEvent[]> {
  const r = await fetch(`${api}/events/custom`);
  if (!r.ok) return [];
  return (await r.json()) as CustomEvent[];
}

export async function fetchMemory(api: string): Promise<MemorySample[]> {
  const r = await fetch(`${api}/memory`);
  if (!r.ok) return [];
  return (await r.json()) as MemorySample[];
}
