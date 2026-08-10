/** Laravel-style double-submit token from readable `XSRF-TOKEN` cookie or config API. */

const XSRF_COOKIE = "XSRF-TOKEN";

let cachedToken: string | null = null;

export function setCsrfToken(token: string | null | undefined) {
  cachedToken = token?.trim() ? token.trim() : null;
}

export function readXsrfToken(): string | null {
  if (typeof document === "undefined") return null;
  const parts = document.cookie.split(";");
  for (const part of parts) {
    const trimmed = part.trim();
    if (!trimmed.startsWith(`${XSRF_COOKIE}=`)) continue;
    const raw = trimmed.slice(XSRF_COOKIE.length + 1);
    if (!raw) return null;
    try {
      return decodeURIComponent(raw);
    } catch {
      return raw;
    }
  }
  return null;
}

export function csrfHeaders(): Record<string, string> {
  const token = readXsrfToken() ?? cachedToken;
  if (!token) return {};
  return {
    "X-XSRF-TOKEN": token,
    "X-CSRF-Token": token,
  };
}

/** Prime CSRF via GET `/_devtools/config` (sets cookie + JSON token). */
export async function ensureCsrfHeaders(
  api: string,
): Promise<Record<string, string>> {
  if (!api) return csrfHeaders();
  if (Object.keys(csrfHeaders()).length) return csrfHeaders();
  try {
    const r = await fetch(`${api}/config`, { credentials: "same-origin" });
    if (r.ok) {
      const body = (await r.json()) as { csrf_token?: string };
      if (body.csrf_token) setCsrfToken(body.csrf_token);
    }
  } catch {
    /* ignore */
  }
  return csrfHeaders();
}
