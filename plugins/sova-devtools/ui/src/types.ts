import type { TabId } from "./persist";

export type { TabId };

export interface RequestMeta {
  id: string;
  request_id: string;
  method: string;
  path: string;
  status: number;
  duration_ms: number;
  at_ms: number;
  sql_count: number;
  log_errors: number;
  http_count: number;
  mail_count: number;
  cache_count?: number;
  job_count?: number;
}

export interface QueryLine {
  sql: string;
  duration_ms?: number | null;
  rows?: number | null;
}

export interface LogLine {
  level: string;
  target: string;
  message: string;
  request_id?: string | null;
  at_ms?: number;
}

export interface HttpLine {
  method: string;
  url: string;
  status?: number | null;
  duration_ms?: number | null;
  error?: string | null;
}

export interface MailLine {
  to: string[];
  subject: string;
  backend: string;
}

export interface JobLine {
  name: string;
  status: string;
  detail?: string | null;
  duration_ms?: number | null;
}

export interface CacheLine {
  op: string;
  key: string;
  hit?: boolean | null;
  bytes?: number | null;
  duration_ms?: number | null;
  backend: string;
  ok?: boolean | null;
}

export interface RouteSnap {
  path: string;
  pattern?: string | null;
  captures?: [string, string][];
}

export interface RateLimitSnap {
  limit?: number | null;
  remaining?: number | null;
  reset?: number | null;
}

export interface AuthSnap {
  session_id?: string | null;
  user_id?: string | null;
  email?: string | null;
  roles?: string[];
  session_keys?: [string, string][];
}

export interface RequestSnapshot {
  id: string;
  request_id: string;
  method: string;
  path: string;
  status: number;
  duration_ms: number;
  at_ms: number;
  queries: QueryLine[];
  logs: LogLine[];
  http: HttpLine[];
  mail: MailLine[];
  jobs: JobLine[];
  cache?: CacheLine[];
  auth?: AuthSnap | null;
  route?: RouteSnap | null;
  locale?: string | null;
  csrf?: boolean | null;
  rate_limit?: RateLimitSnap | null;
  encoding?: string | null;
}

export interface MountAttrs {
  snap: string;
  status: number;
  statusClass: "ok" | "warn" | "err";
  ms: number;
  sql: number;
  errors: number;
  api: string;
  events: string;
  shell: boolean;
  embed: boolean;
}

export interface CustomEvent {
  id: string;
  kind: string;
  payload: unknown;
  ts_ms: number;
}

export interface MemorySample {
  ts_ms: number;
  rss_bytes: number | null;
  rss_peak_bytes?: number | null;
  available_bytes?: number | null;
}

export interface MemorySummary {
  samples: MemorySample[];
  current: number | null;
  peak: number | null;
  min: number | null;
}

export function statusTone(status: number): "ok" | "warn" | "err" {
  if (status >= 500) return "err";
  if (status >= 400) return "warn";
  return "ok";
}

export function fmtMs(v: number | null | undefined): string {
  if (v == null || Number.isNaN(v)) return "—";
  return `${v.toFixed(1)}ms`;
}

export function sumMs(
  items: { duration_ms?: number | null }[] | undefined,
): number {
  if (!items?.length) return 0;
  return items.reduce((a, x) => a + (x.duration_ms ?? 0), 0);
}
