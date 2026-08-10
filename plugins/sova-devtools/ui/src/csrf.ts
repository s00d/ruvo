/** Laravel-style double-submit token from readable `XSRF-TOKEN` cookie. */

const XSRF_COOKIE = "XSRF-TOKEN";

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
  const token = readXsrfToken();
  if (!token) return {};
  return {
    "X-XSRF-TOKEN": token,
    "X-CSRF-Token": token,
  };
}

/** Prime `XSRF-TOKEN` via a safe GET when the iframe loaded without one. */
export async function ensureCsrfHeaders(
  api: string,
): Promise<Record<string, string>> {
  let headers = csrfHeaders();
  if (Object.keys(headers).length || !api) return headers;
  try {
    await fetch(`${api}/config`, { credentials: "same-origin" });
  } catch {
    /* ignore */
  }
  headers = csrfHeaders();
  return headers;
}
