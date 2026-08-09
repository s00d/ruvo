import {
  createMemoryHistory,
  createRouter,
  type RouteRecordRaw,
} from "vue-router";
import type { TabId } from "./types";
import { TAB_META } from "./tabs";
import RequestView from "./views/RequestView.vue";
import TimelineView from "./views/TimelineView.vue";
import DbView from "./views/DbView.vue";
import CacheView from "./views/CacheView.vue";
import LogsView from "./views/LogsView.vue";
import HttpView from "./views/HttpView.vue";
import MailView from "./views/MailView.vue";
import JobsView from "./views/JobsView.vue";
import AuthView from "./views/AuthView.vue";
import ConfigView from "./views/ConfigView.vue";
import EventsView from "./views/EventsView.vue";
import MemoryView from "./views/MemoryView.vue";

/** @deprecated use TAB_META */
export const TAB_ORDER = TAB_META.map((t) => ({ id: t.id, label: t.label }));

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/request" },
  { path: "/request", name: "request", component: RequestView },
  { path: "/timeline", name: "timeline", component: TimelineView },
  { path: "/db", name: "db", component: DbView },
  { path: "/cache", name: "cache", component: CacheView },
  { path: "/logs", name: "logs", component: LogsView },
  { path: "/http", name: "http", component: HttpView },
  { path: "/mail", name: "mail", component: MailView },
  { path: "/jobs", name: "jobs", component: JobsView },
  { path: "/auth", name: "auth", component: AuthView },
  { path: "/events", name: "events", component: EventsView },
  { path: "/memory", name: "memory", component: MemoryView },
  { path: "/config", name: "config", component: ConfigView },
];

export const router = createRouter({
  history: createMemoryHistory(),
  routes,
});

export function isTabId(name: unknown): name is TabId {
  return typeof name === "string" && TAB_META.some((t) => t.id === name);
}
