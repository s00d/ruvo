export function hashSql(sql: string): string {
  const n = sql.replace(/\s+/g, " ").trim().toLowerCase();
  let h = 0;
  for (let i = 0; i < n.length; i++) h = (h * 31 + n.charCodeAt(i)) | 0;
  return String(h);
}

export function groupDuplicates(sqls: string[]): Map<string, number> {
  const m = new Map<string, number>();
  for (const s of sqls) {
    const k = hashSql(s);
    m.set(k, (m.get(k) || 0) + 1);
  }
  return m;
}

export const SLOW_SQL_MS = 50;
