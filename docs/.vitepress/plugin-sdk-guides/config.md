Plugins should accept **builder overrides** and fill gaps from `sova.toml` / env. Convention across cors, csrf, shield, session, static, http, mail:

### Unset-fill from `config_doc`

```rust
if let Some(doc) = app.config_doc() {
    if let Some(section) = doc.section("cors") {
        // only fill fields the builder left unset
        self.apply_toml(section);
    }
}
```

Explicit builder values win over toml. Document keys on the [plugin page](/plugins/).

### Env precedence

Common order (db/redis style):

1. Builder / pin URL
2. Env (`DATABASE_URL`, `REDIS_URL`, …)
3. Toml section
4. Empty → **fail on startup** (see [Errors](/plugin-sdk/errors)), not silent no-op

### Human-readable sizes & durations

```rust
use sova_core::extend::{parse_bytes, parse_duration};

let ttl = parse_duration("7d")?;      // session
let max = parse_bytes("10mb")?;       // http / uploads
```

Used heavily by session, static (`max_age`), http, tasks schedules.

### Feature-gated backends

Cargo features select optional deps (session-sql, store-redis, mail-templates). Facade crate `sova` maps features in `Cargo.toml` + documents them via `doc_features.rs` for the catalog generator.

Plugin authors:

- Keep default lean
- Gate optional code with `#[cfg(feature = "…")]`
- Document facade feature names on the plugin guide

### CLI vs server config

`app.cli_mode()` — skip starting background workers unless `service_in_cli` (tasks pattern). Config parsing still runs so `myapp migrate` sees the same toml.
