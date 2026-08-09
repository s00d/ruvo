/** Shared keys / message shapes for bridge ↔ panel (same origin). */

export const STORAGE_KEY = "sova.devtools.v2";
export const SPARK_KEY = "sova.devtools.spark";
export const PANEL_H_KEY = "sova.devtools.panelH";
export const CHANNEL = "sova.devtools";

export type TabId =
  | "request"
  | "timeline"
  | "db"
  | "cache"
  | "logs"
  | "http"
  | "mail"
  | "jobs"
  | "auth"
  | "config";

export type UiState = {
  open: boolean;
  tab: TabId;
};

export type PageMsg = {
  type: "page";
  snap: string;
  status: number;
  statusClass: "ok" | "warn" | "err";
  ms: number;
  sql: number;
  errors: number;
  path?: string;
};

export type UiMsg = {
  type: "ui";
  open: boolean;
  tab?: TabId;
};

/** iframe → host: resize dock panel height */
export type PanelHMsg = {
  type: "panelH";
  height: number;
};

export type BusMsg = PageMsg | UiMsg | PanelHMsg;

export const PANEL_H_MSG = "sova-dt-panel-h";

export function clampPanelH(h: number): number {
  return Math.min(800, Math.max(220, Math.round(h)));
}

export const DEFAULT_UI: UiState = {
  open: false,
  tab: "request",
};

export function loadUi(): UiState {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_UI };
    const parsed = JSON.parse(raw) as {
      open?: boolean;
      tab?: string;
    };
    return {
      open: Boolean(parsed.open),
      tab: (parsed.tab as TabId) || "request",
    };
  } catch {
    return { ...DEFAULT_UI };
  }
}

export function saveUi(state: UiState): void {
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    /* private mode */
  }
}

export function patchUi(patch: Partial<UiState>): UiState {
  const next = { ...loadUi(), ...patch };
  saveUi(next);
  return next;
}

export function loadSpark(): number[] {
  try {
    const raw = sessionStorage.getItem(SPARK_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw) as number[];
    return Array.isArray(arr) ? arr.filter((n) => typeof n === "number").slice(-40) : [];
  } catch {
    return [];
  }
}

export function pushSpark(ms: number): number[] {
  const next = [...loadSpark(), ms].slice(-40);
  try {
    sessionStorage.setItem(SPARK_KEY, JSON.stringify(next));
  } catch {
    /* ignore */
  }
  return next;
}

export function loadPanelH(): number {
  try {
    const n = Number(sessionStorage.getItem(PANEL_H_KEY));
    if (Number.isFinite(n) && n >= 220 && n <= 800) return n;
  } catch {
    /* ignore */
  }
  return 420;
}

export function savePanelH(h: number): void {
  try {
    sessionStorage.setItem(PANEL_H_KEY, String(Math.round(h)));
  } catch {
    /* ignore */
  }
}
