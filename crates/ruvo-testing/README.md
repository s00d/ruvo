# ruvo-testing

Framework-style test harness for Ruvo integration tests.

## Features

| Feature | Provides |
|---------|----------|
| `sqlite` (default) | [`SqliteTestDb`], [`TestApp`], migrate helpers |
| `auth` | [`ActingAs::acting_as`], [`UserFactory`], role/permission seed |
| `notifications` | [`ActingAs::acting_as_id`] / `NotificationUser` injection |

## Quick start

```toml
[dev-dependencies]
ruvo-testing = { path = "../../crates/ruvo-testing", features = ["sqlite", "notifications", "auth"] }
ruvo-core = { path = "../../crates/ruvo-core", features = ["testing"] }
```

```rust
use ruvo_core::{ResponseAssert, TestClient};
use ruvo_testing::{ActingAs, TestApp, assert_json_snapshot};

let (_db, app) = TestApp::builder()
    .migrator::<NotificationsMigrator>()
    .install(Notifications::new().channel(...).mount("/notifications"))
    .build()
    .await;

let c = TestClient::tracked(app).unwrap();
c.acting_as_id(1);
let res = c.get("/notifications").await;
res.assert_status(200);
assert_json_snapshot!("inbox", res.json_value());
```

## Pieces

- **`SqliteTestDb`** — tempfile sqlite + [`apply_migrations`] (direct `MigrationTrait::up`; avoids `DeriveMigrationName` collisions when several migrations live in one file).
- **`TestApp`** — migrate, install `Db` with an **explicit URL** (no parallel-test `DATABASE_URL` races), plugins, `configure`, `run_startup`.
- **`TestClient::on_request` / `ActingAs`** — inject `CurrentUser` / `NotificationUser` on every request.
- **`ResponseAssert`** — `assert_status`, `json`, `json_value` (from `ruvo-core`).
- **`assert_json_snapshot!`** — insta JSON snapshots with id/timestamp redactions.
- **`UserFactory`** — register a Fortify user and return `CurrentUser`.

Facade feature `testing` stays lifecycle-only (`run_startup` / `run_shutdown`). Depend on this crate from plugin `[dev-dependencies]`.
