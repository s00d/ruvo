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
  /** Session CSRF token (when `devtools-csrf` / web preset). */
  csrf_token?: string;
};

export type TabGroupId = "inspect" | "storage" | "messaging" | "apis" | "runtime";

export type TabGroup = {
  id: TabGroupId;
  label: string;
  tabIds: TabId[];
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

/** Sidebar groups (shell / new-tab layout). Redis lives with brokers, not SQL/cache. */
export const TAB_GROUPS: TabGroup[] = [
  { id: "inspect", label: "Inspect", tabIds: ["request", "timeline"] },
  { id: "storage", label: "Storage", tabIds: ["db", "cache", "jobs"] },
  { id: "messaging", label: "Messaging", tabIds: ["redis", "rabbit"] },
  { id: "apis", label: "APIs", tabIds: ["http", "graphql", "grpc"] },
  {
    id: "runtime",
    label: "Runtime",
    tabIds: ["logs", "mail", "auth", "events", "memory", "config"],
  },
];

export type VisibleTabGroup = TabGroup & { tabs: TabMeta[] };

export function buildVisibleTabGroups(
  cfg: DevToolsConfig | null,
  isPlayground: boolean,
): VisibleTabGroup[] {
  const visibleIds = new Set(
    visibleTabMeta(cfg, isPlayground).map((t) => t.id),
  );
  const byId = Object.fromEntries(TAB_META.map((t) => [t.id, t])) as Record<
    TabId,
    TabMeta
  >;
  return TAB_GROUPS.map((g) => ({
    ...g,
    tabs: g.tabIds
      .filter((id) => visibleIds.has(id))
      .map((id) => byId[id])
      .filter(Boolean),
  })).filter((g) => g.tabs.length > 0);
}

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
