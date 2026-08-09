/**
 * Host-page bridge: Symfony-style chip toolbar + dock iframe.
 * "New tab" opens SPA once — no mode persistence.
 */

import {
  CHANNEL,
  loadPanelH,
  loadSpark,
  loadUi,
  patchUi,
  pushSpark,
  savePanelH,
  clampPanelH,
  PANEL_H_MSG,
  type BusMsg,
  type PageMsg,
  type UiState,
} from "./persist";

const FRAME_ID = "sova-dt-frame";
const BAR_ID = "sova-dt-bar";
const PANEL_URL = "/_devtools/app?embed=1";
const SHELL_URL = "/_devtools/app";
const BAR_H = 36;

const C = {
  bg: "#0b0d12",
  surface: "#141820",
  surface2: "#1a1f2a",
  border: "#2c3340",
  borderStrong: "#454d5e",
  text: "#eef0f4",
  muted: "#9aa3b2",
  faint: "#6b7382",
  accent: "#3dd68c",
  ok: "#3dd68c",
  warn: "#e6b35a",
  err: "#f07178",
  info: "#6cb6ff",
  mono: '"IBM Plex Mono", ui-monospace, Menlo, Consolas, monospace',
  sans: '"IBM Plex Sans", ui-sans-serif, system-ui, sans-serif',
};

function host(): HTMLElement | null {
  return document.getElementById("sova-devtools");
}

function readPage(): PageMsg | null {
  const el = host();
  if (!el) return null;
  const ds = el.dataset;
  const sc = ds.statusClass;
  return {
    type: "page",
    snap: ds.snap || "",
    status: Number(ds.status || 0),
    statusClass: sc === "warn" || sc === "err" ? sc : "ok",
    ms: Number(ds.ms || 0),
    sql: Number(ds.sql || 0),
    errors: Number(ds.errors || 0),
    path: location.pathname,
  };
}

function channel(): BroadcastChannel | null {
  try {
    return new BroadcastChannel(CHANNEL);
  } catch {
    return null;
  }
}

function post(msg: BusMsg) {
  channel()?.postMessage(msg);
}

function chip(
  label: string,
  value: string,
  left: string,
  valueColor = C.text,
): string {
  return `<button type="button" data-dt-toggle style="
    display:inline-flex;align-items:center;gap:6px;min-height:32px;
    padding:4px 10px;border:1px solid ${C.border};border-left:3px solid ${left};
    border-radius:6px;background:${C.surface};color:${C.text};cursor:pointer;
    font-family:${C.sans};font-size:12px;white-space:nowrap;
  "><span style="font-size:9px;text-transform:uppercase;letter-spacing:.06em;color:${C.faint}">${label}</span><span style="font-family:${C.mono};font-weight:500;color:${valueColor}">${value}</span></button>`;
}

function sparkSvg(values: number[]): string {
  const vals = values.length ? values : [0];
  const w = 72;
  const h = 16;
  const pad = 2;
  const max = Math.max(...vals, 1);
  const min = Math.min(...vals, 0);
  const span = Math.max(max - min, 1e-6);
  const n = vals.length;
  const d = vals
    .map((v, i) => {
      const x = n === 1 ? w / 2 : (i / (n - 1)) * (w - pad * 2) + pad;
      const y = h - pad - ((v - min) / span) * (h - pad * 2);
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return `<svg width="${w}" height="${h}" viewBox="0 0 ${w} ${h}" aria-hidden="true" style="display:block"><path d="${d}" fill="none" stroke="${C.accent}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
}

function ensureBar(page: PageMsg) {
  let bar = document.getElementById(BAR_ID);
  if (!bar) {
    bar = document.createElement("div");
    bar.id = BAR_ID;
    bar.setAttribute("role", "toolbar");
    Object.assign(bar.style, {
      position: "fixed",
      left: "0",
      right: "0",
      bottom: "0",
      height: "36px",
      display: "flex",
      alignItems: "center",
      gap: "6px",
      padding: "4px 8px",
      background: C.bg,
      borderTop: `1px solid ${C.borderStrong}`,
      zIndex: "2147483001",
      boxSizing: "border-box",
      overflowX: "auto",
    } as CSSStyleDeclaration);
    document.documentElement.appendChild(bar);

    bar.addEventListener("click", (e) => {
      const t = e.target as HTMLElement;
      if (t.closest("[data-dt-tab]")) {
        e.stopPropagation();
        window.open(SHELL_URL, "_blank", "noopener,noreferrer");
        const page = readPage();
        if (page) post(page);
        return;
      }
      if (t.closest("[data-dt-toggle]") || t.closest("[data-dt-brand]")) {
        toggle();
      }
    });

    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        const ui = loadUi();
        if (ui.open) {
          const next = patchUi({ open: false });
          ensureFrame(false);
          announce(next);
        }
      }
    });
  }

  const tone =
    page.statusClass === "err"
      ? C.err
      : page.statusClass === "warn"
        ? C.warn
        : C.ok;

  bar.innerHTML = `
    <button type="button" data-dt-brand style="
      min-height:32px;padding:0 10px;border:1px solid ${C.border};border-radius:6px;
      background:${C.surface};color:${C.accent};font-weight:700;font-size:13px;
      font-family:${C.sans};cursor:pointer;letter-spacing:.02em;
    ">Sova</button>
    ${chip("status", String(page.status), tone, tone)}
    ${chip("time", page.ms.toFixed(1) + "ms", C.borderStrong)}
    ${chip("sql", String(page.sql), C.info, C.info)}
    ${chip("err", String(page.errors), page.errors ? C.err : C.borderStrong, page.errors ? C.err : C.text)}
    <span style="display:inline-flex;align-items:center;margin-left:4px">${sparkSvg(loadSpark())}</span>
    <span style="margin-left:auto;display:flex;gap:6px;align-items:center">
      <button type="button" data-dt-tab style="
        min-height:32px;padding:0 10px;border:1px solid ${C.border};border-radius:6px;
        background:${C.surface};color:${C.muted};font-size:11px;font-family:${C.sans};cursor:pointer;
      ">New tab</button>
    </span>
  `;
}

function applyFrameHeight(frame: HTMLIFrameElement, h: number) {
  const height = clampPanelH(h);
  frame.style.height = `${height}px`;
  savePanelH(height);
}

function ensureFrame(show: boolean) {
  let frame = document.getElementById(FRAME_ID) as HTMLIFrameElement | null;
  if (!show) {
    if (frame) frame.hidden = true;
    return;
  }
  if (!frame) {
    frame = document.createElement("iframe");
    frame.id = FRAME_ID;
    frame.title = "Sova DevTools";
    frame.src = PANEL_URL;
    Object.assign(frame.style, {
      position: "fixed",
      left: "0",
      right: "0",
      bottom: `${BAR_H}px`,
      height: `${loadPanelH()}px`,
      width: "100%",
      border: "0",
      zIndex: "2147483001",
      background: C.bg,
      boxShadow: "0 -12px 40px rgba(0,0,0,.45)",
    } as CSSStyleDeclaration);
    document.documentElement.appendChild(frame);
  } else {
    applyFrameHeight(frame, loadPanelH());
  }
  frame.hidden = false;
}

function announce(ui: UiState) {
  post({ type: "ui", open: ui.open, tab: ui.tab });
}

function toggle() {
  const open = !loadUi().open;
  const ui = patchUi({ open });
  ensureFrame(open);
  announce(ui);
}

function boot() {
  const page = readPage();
  if (!page) return;
  if (page.ms > 0) pushSpark(page.ms);
  ensureBar(page);
  post(page);
  ensureFrame(loadUi().open);

  channel()?.addEventListener("message", (ev: MessageEvent<BusMsg>) => {
    const msg = ev.data;
    if (!msg || typeof msg !== "object" || msg.type !== "ui") return;
    const next = patchUi({
      open: msg.open,
      ...(msg.tab ? { tab: msg.tab } : {}),
    });
    ensureFrame(next.open);
  });

  window.addEventListener("message", (ev: MessageEvent) => {
    const data = ev.data;
    if (!data || typeof data !== "object") return;
    if (data.type !== PANEL_H_MSG) return;
    const frame = document.getElementById(FRAME_ID) as HTMLIFrameElement | null;
    if (!frame || typeof data.height !== "number") return;
    applyFrameHeight(frame, data.height);
  });

  // Back/forward cache restores the document without a network request — force
  // a real navigation so the server records a DevTools snapshot again.
  window.addEventListener("pageshow", (ev) => {
    const pe = ev as PageTransitionEvent;
    if (pe.persisted) {
      location.reload();
    }
  });
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
