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
}

export interface AuthSnap {
  session_id?: string | null;
  user_id?: string | null;
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
  auth?: AuthSnap | null;
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
