**When:** local debugging of HTML apps — request timeline, SQL, logs, outbound HTTP, mail, session.

**Does:**
- Injects a bottom bar into `text/html` only (not JSON/SSE/streams)
- Collects per-request snapshot (correlated via `request_id`)
- Site-wide live feed over SSE `GET /_devtools/events`
- JSON: `/_devtools/requests`, `/_devtools/requests/:id`, `/_devtools/logs`, `/_devtools/config`
- Soft hooks: session dump, FakeMail, route / rate-limit / encoding; sqlx / http / store / redis / tasks via `add_log_event_hook`
- Mirrors console/`tracing` into Logs; skips `/_devtools` access logs via `logger_skip_path`
- **Release builds:** hard-off unless `SOVA_DEVTOOLS=1`

Full guide (screenshots + tour GIF): [DevTools](/guide/devtools)

### Example

```rust
app.install(DevTools::new()); // on in debug / development
```

### Config

```toml
[development.devtools]
enabled = true

[production.devtools]
enabled = false
```

```bash
SOVA_DEVTOOLS=1   # force on (incl. release)
SOVA_DEVTOOLS=0   # force off
```

Default: on in debug + development profile; off in `--release`.

For SQL tab with SeaORM: `Db::from_env().sqlx_logging(true)` and/or `RUST_LOG=sqlx=debug`.

### Notes
- GET-only under `/_devtools/*` — not for production exposure
- Install after session/mail if you want those tabs filled
- See [`examples/web/devtools`](https://github.com/s00d/sova/tree/master/examples/web/devtools) (`devtools_demo`)
- Guide: [/guide/devtools](/guide/devtools)
