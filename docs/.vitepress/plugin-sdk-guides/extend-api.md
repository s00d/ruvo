Authoritative import path for plugin crates: `sova_core::extend::…` (facade re-exports many names as `sova::extend` when using the `sova` crate).

### Always-on plugin surface

| Symbol | Role | Used in-tree |
|--------|------|----------------|
| `PluginMeta`, `PLUGIN_SDK_VERSION`, `PluginSdkVersion`, `check_plugin_sdk`, `SdkCompat`, `InstalledPlugin` | Identity / SDK | nearly all plugins |
| `named` | Label MW | cookies, session, csrf, cors, shield, … |
| `with_leaked` | `'static` config MW | cors, shield, compress |
| `MwEntry`, `IntoMwEntry`, `IntoMiddleware`, `Middleware` | Composable MW | auth, passport, db tx, rate-limit |
| `BoxFuture` | Async plugin internals | session, http, tasks, … |
| `parse_duration`, `parse_bytes` | Human config | session, static, http, tasks |
| `StateMap` | Startup/CLI/service state | db, tasks, meta, quic/udp |
| `wait_shutdown` | Service loops | tasks, quic, udp |
| `ErrorResponse` | Typed HTTP errors | http, db, storage, vld |
| `normalize_path`, `to_brace_path`, `join_paths` | Paths | static, openapi |
| `RouteTable`, `RouteEntry`, `RouteValue`, `MetaMap`, `Needs`, `BuildCtx` | Introspection | openapi, vld, meta, i18n, session |
| `MatchedRoute`, `MatchedRouteCapture`, `MatchedMeta`, `MatchedMetaCapture` | Post-match | meta, observability, devtools |
| `Extensions`, `TypeMap` | Low-level maps | advanced |
| `Body`, `BoxError`, `HttpBody`, `ResponseBody` | Streaming bodies | sse, compress |
| `HtmlInject`, `HtmlAnchor`, `inject*` | HTML rewrite | meta/devtools patterns |
| `logger_skip_path`, `logger_skip_paths` | Quiet routes | **devtools** |
| `add_log_event_hook`, `LogRecord`, … | Log sinks | **devtools** |
| `request_id`, `RequestId`, `ensure_request_id` | Correlation | core MW; plugins read via extensions / `current_request_id` |
| `Deadline`, `MaxBody`, `RequestTimeout`, `tighten_deadline` | Limits | http tests / clients |
| `Bind` | Listen helpers | tests / advanced bind |
| `Cell`, `Slot` | Cross-task share (`sova_core::share`) | `share_demo`, in-memory APIs |
| `after`, `before`, `around`, `map_html` | Response MW helpers | available; plugins often custom-inject |
| `RawHandler`, `IntoRawHandler` | Escape hatch handlers | niche |
| `FormData`, `RequestBuilder`, `Upload`, `UploadRules` | Request building | uploads / tests |
| `Handler`, `IntoHandler`, `FallibleHandler`, … | Handler traits | framework glue |

### Crate-root companions (not only under `extend`)

| Symbol | Role |
|--------|------|
| `App`, `Plugin`, `Request`, `Response`, `Html`, `Json` | Daily drivers |
| `with_state` | Arc MW (also used heavily by plugins) |
| `logger_skip_path` | Same as extend (devtools) |
| `current_request_id` | Correlate store/redis/tasks traces |
| `TestClient` (feature `testing`) | Integration tests |

### Unused ≠ forbidden

`Cell` / `Slot` / `map_html` are part of the supported SDK even if few plugins use them today. Prefer documented helpers over inventing parallel utilities.

### Share handles (`Cell` / `Slot`)

Re-exported from the crate root (`sova::Cell`, `sova::Slot`) and `sova_core::extend`.

- **`Cell<T: Clone>`** — shared value + `changed()` watch (counters, flags).
- **`Slot<T>`** — single-item ownership transfer (`put` / `take`); ideal for handing a `TcpStream` from a `BackgroundService` to an HTTP handler.

Guide: [Concepts → Share](/guide/concepts#share-cell-slot). Example: [`share_demo`](https://github.com/s00d/sova/tree/master/examples/misc/share_demo).

### App-author core (guide)

Features below are documented for application developers in [Concepts](/guide/concepts), not only in this SDK table:

| Area | Concepts section |
|------|------------------|
| SSE / file streaming | [Streaming responses](/guide/concepts#streaming-responses) |
| Upgrades / WS budget | [HTTP upgrades](/guide/concepts#http-upgrades-websocket) |
| `EventBus` | [In-process events](/guide/concepts#in-process-events) |
| Problem Details / Accept | [Errors & content negotiation](/guide/concepts#errors-content-negotiation) |
| `MaxBody` / timeouts | [Route metadata & limits](/guide/concepts#route-metadata-limits) |
| `AppDispatch` | [In-process dispatch](/guide/concepts#in-process-dispatch) |
| Checks / CLI hooks | [Checks, audits & custom CLI](/guide/concepts#checks-audits-custom-cli) |
| Server knobs | [Server tuning](/guide/concepts#server-tuning) |
