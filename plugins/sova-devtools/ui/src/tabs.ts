import type { Component } from "vue";
import {
  Activity,
  Braces,
  Cpu,
  Database,
  FileJson,
  Boxes,
  HardDrive,
  Inbox,
  KeyRound,
  ListTree,
  Mail,
  Network,
  Radio,
  Archive,
  ScrollText,
  Zap,
} from "@lucide/vue";
import type { TabId } from "./types";

/** Runtime config from `GET /_devtools/config`. */
export type DevToolsConfig = {
  features?: string[];
  mounts?: Record<string, unknown>;
};

export type TabMeta = {
  id: TabId;
  label: string;
  icon: Component;
  /** Tab visible when any listed compile-time feature is present. */
  anyFeature?: string[];
  /** Tab also visible when this mount key exists (e.g. graphql server). */
  anyMount?: string;
};

export const TAB_META: TabMeta[] = [
  { id: "request", label: "Request", icon: Activity },
  { id: "timeline", label: "Timeline", icon: ListTree },
  { id: "db", label: "DB", icon: Database, anyFeature: ["db"] },
  {
    id: "cache",
    label: "Cache",
    icon: Archive,
    anyFeature: ["store", "console-store"],
  },
  {
    id: "redis",
    label: "Redis",
    icon: Boxes,
    anyFeature: ["redis", "console-redis"],
  },
  { id: "logs", label: "Logs", icon: ScrollText },
  {
    id: "http",
    label: "HTTP",
    icon: Network,
    anyFeature: ["http", "console"],
  },
  {
    id: "graphql",
    label: "GraphQL",
    icon: Braces,
    anyFeature: ["graphql", "console-graphql"],
    anyMount: "graphql",
  },
  {
    id: "grpc",
    label: "gRPC",
    icon: Radio,
    anyFeature: ["grpc", "console-grpc"],
    anyMount: "grpc",
  },
  {
    id: "rabbit",
    label: "Rabbit",
    icon: Inbox,
    anyFeature: ["rabbit", "console-rabbit"],
    anyMount: "rabbit",
  },
  { id: "mail", label: "Mail", icon: Mail, anyFeature: ["mail"] },
  { id: "jobs", label: "Jobs", icon: HardDrive, anyFeature: ["tasks"] },
  {
    id: "auth",
    label: "Auth",
    icon: KeyRound,
    anyFeature: ["session", "auth", "console-session"],
  },
  { id: "events", label: "Events", icon: Zap },
  { id: "memory", label: "Memory", icon: Cpu },
  { id: "config", label: "Config", icon: FileJson },
];

/** Tab visibility is compile-time / mount only — never hides tabs when trace data is empty. */
export function isTabVisible(
  meta: TabMeta,
  cfg: DevToolsConfig | null,
  isPlayground: boolean,
): boolean {
  if (isPlayground) return true;
  if (!meta.anyFeature?.length && !meta.anyMount) return true;
  const features = cfg?.features ?? [];
  const mounts = cfg?.mounts ?? {};
  if (meta.anyFeature?.some((f) => features.includes(f))) return true;
  if (meta.anyMount && mounts[meta.anyMount] != null) return true;
  return false;
}

export function visibleTabMeta(
  cfg: DevToolsConfig | null,
  isPlayground: boolean,
): TabMeta[] {
  return TAB_META.filter((t) => isTabVisible(t, cfg, isPlayground));
}
