import type { Component } from "vue";
import {
  Activity,
  Database,
  FileJson,
  HardDrive,
  KeyRound,
  ListTree,
  Mail,
  Network,
  ScrollText,
} from "@lucide/vue";
import type { TabId } from "./types";

export const TAB_META: {
  id: TabId;
  label: string;
  icon: Component;
}[] = [
  { id: "request", label: "Request", icon: Activity },
  { id: "timeline", label: "Timeline", icon: ListTree },
  { id: "db", label: "DB", icon: Database },
  { id: "logs", label: "Logs", icon: ScrollText },
  { id: "http", label: "HTTP", icon: Network },
  { id: "mail", label: "Mail", icon: Mail },
  { id: "jobs", label: "Jobs", icon: HardDrive },
  { id: "auth", label: "Auth", icon: KeyRound },
  { id: "config", label: "Config", icon: FileJson },
];
