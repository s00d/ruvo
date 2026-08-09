import type { LogLine, RequestMeta, RequestSnapshot } from "../types";

const SQL_USERS =
  "SELECT id, email, name FROM users WHERE active = 1 ORDER BY created_at DESC LIMIT 50";
const SQL_DUP =
  "SELECT * FROM sessions WHERE user_id = $1";

export const mockCurrent: RequestSnapshot = {
  id: "snap_demo_001",
  request_id: "req_7f3a9c",
  method: "GET",
  path: "/api/dashboard?range=7d",
  status: 200,
  duration_ms: 142.6,
  at_ms: Date.now() - 1000,
  queries: [
    { sql: SQL_USERS, duration_ms: 12.4, rows: 50 },
    { sql: SQL_DUP, duration_ms: 4.1, rows: 1 },
    { sql: SQL_DUP, duration_ms: 3.8, rows: 1 },
    {
      sql: "SELECT COUNT(*) FROM orders WHERE created_at > NOW() - INTERVAL '7 days'",
      duration_ms: 68.2,
      rows: 1,
    },
    {
      sql: "UPDATE products SET stock = stock - 1 WHERE id = $1 RETURNING *",
      duration_ms: 9.5,
      rows: 1,
    },
  ],
  http: [
    {
      method: "GET",
      url: "https://api.stripe.com/v1/balance",
      status: 200,
      duration_ms: 88.0,
    },
    {
      method: "POST",
      url: "https://hooks.slack.com/services/T00/B00/xxx",
      status: 404,
      duration_ms: 45.2,
      error: "channel_not_found",
    },
  ],
  logs: [
    {
      level: "INFO",
      target: "sova::http",
      message: "request started",
      request_id: "req_7f3a9c",
      at_ms: Date.now() - 140,
    },
    {
      level: "WARN",
      target: "sova_db",
      message: "slow query 68.2ms",
      request_id: "req_7f3a9c",
      at_ms: Date.now() - 80,
    },
    {
      level: "ERROR",
      target: "sova_http_client",
      message: "slack webhook failed: 404",
      request_id: "req_7f3a9c",
      at_ms: Date.now() - 40,
    },
  ],
  mail: [
    {
      to: ["ops@example.com"],
      subject: "Daily digest",
      backend: "fake",
    },
  ],
  jobs: [
    { name: "reindex_search", status: "queued", detail: "delay 5s" },
    { name: "send_digest", status: "running", detail: null },
  ],
  auth: {
    session_id: "sess_abc123",
    user_id: "42",
    session_keys: [
      ["role", "admin"],
      ["locale", "en"],
      ["csrf", "***"],
    ],
  },
};

export const mockTimeline: RequestMeta[] = [
  {
    id: "snap_demo_001",
    request_id: "req_7f3a9c",
    method: "GET",
    path: "/api/dashboard?range=7d",
    status: 200,
    duration_ms: 142.6,
    at_ms: Date.now() - 1000,
    sql_count: 5,
    log_errors: 1,
    http_count: 2,
    mail_count: 1,
  },
  {
    id: "snap_demo_002",
    request_id: "req_aa11",
    method: "POST",
    path: "/api/login",
    status: 401,
    duration_ms: 28.1,
    at_ms: Date.now() - 5000,
    sql_count: 1,
    log_errors: 0,
    http_count: 0,
    mail_count: 0,
  },
  {
    id: "snap_demo_003",
    request_id: "req_bb22",
    method: "GET",
    path: "/admin/users",
    status: 500,
    duration_ms: 410.0,
    at_ms: Date.now() - 12000,
    sql_count: 8,
    log_errors: 3,
    http_count: 1,
    mail_count: 0,
  },
  {
    id: "snap_demo_004",
    request_id: "req_cc33",
    method: "GET",
    path: "/",
    status: 200,
    duration_ms: 18.4,
    at_ms: Date.now() - 20000,
    sql_count: 0,
    log_errors: 0,
    http_count: 0,
    mail_count: 0,
  },
];

export const mockLogs: LogLine[] = mockCurrent.logs;

export const mockConfig = {
  profile: "development",
  plugins: ["devtools", "db", "session", "mail", "sse"],
};

export function mockBundle() {
  return {
    timeline: mockTimeline,
    current: mockCurrent,
    logs: mockLogs,
    config: mockConfig,
  };
}
