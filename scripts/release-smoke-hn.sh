#!/usr/bin/env bash
# Release / crates.io smoke for the HN-shaped stack (web + db-sqlite + auth).
#
# Default: exercise the in-repo `hackernews` example (workspace path deps).
# With SOVA_SMOKE_CRATES=1: scaffold a tiny clone under /tmp using crates.io `sova`.
set -euo pipefail

export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:${PATH:-}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> workspace: cargo test -p hackernews"
cargo test -p hackernews -- --nocapture

if [[ "${SOVA_SMOKE_CRATES:-}" != "1" ]]; then
  echo "OK (workspace). Set SOVA_SMOKE_CRATES=1 for crates.io consumer smoke."
  exit 0
fi

DIR="$(mktemp -d /tmp/sova-hn-smoke.XXXXXX)"
echo "==> crates.io consumer under $DIR"
cleanup() { rm -rf "$DIR"; }
trap cleanup EXIT

cd "$DIR"
cargo new hn-smoke --bin --name hn_smoke >/dev/null
cd hn-smoke

# Pin features that the HN demo needs; versions resolve from crates.io.
cargo add sova --features "web,db-sqlite,auth,auth-vld,vld-form,vld-flash,env,testing"
cargo add sea-orm --no-default-features --features "runtime-tokio-rustls,macros,sqlx-sqlite,with-chrono"
cargo add sea-orm-migration --no-default-features --features "runtime-tokio-rustls,sqlx-sqlite"
cargo add async-trait serde serde_json chrono --features chrono/clock,chrono/serde
cargo add tokio --features "macros,rt-multi-thread"
cargo add --dev tempfile http

# Minimal app: Fortify Registration-only (no Mail) + one protected page.
cat > src/main.rs <<'RS'
use sova::prelude::*;
use sova::{AuthFeature, AuthMigrator, Db, Fortify, Html, Parser, ServerArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();
    let mut app = App::web()
        .site("HN Smoke")
        .public_url("http://127.0.0.1:3000")
        .into_app();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(
        Fortify::new()
            .features([AuthFeature::Registration])
            .web_forms(true)
            .no_api()
            .home("/")
            .login_redirect("/login")
            .secret(std::env::var("FORTIFY_SECRET").unwrap_or_else(|_| "smoke-secret".into())),
    );
    let mut r = Router::new();
    r.use_middleware(Fortify::guard());
    r.get("/", || async { Html("<h1>ok</h1>".into()) });
    app.mount("/me", r);
    app.get("/", || async { Html("<a href=/me>me</a>".into()) });
    app.run().await
}
RS

DB="$DIR/smoke.db"
export DATABASE_URL="sqlite:${DB}?mode=rwc"
export FORTIFY_SECRET="smoke-secret-change-me"
export SOVA_LOG=off

echo "==> migrate"
cargo run --quiet -- migrate
echo "==> build"
cargo build --quiet
echo "==> register+guard via TestClient"
cat > tests/smoke.rs <<'RS'
use sova::{AuthFeature, AuthMigrator, Db, Fortify, Html, ResponseAssert, Router, TestClient};
use sova::prelude::*;
use tempfile::TempDir;

fn csrf_from_html(body: &str) -> String {
    let marker = "name=\"csrf\" value=\"";
    let start = body.find(marker).map(|i| i + marker.len()).expect("csrf");
    let rest = &body[start..];
    rest[..rest.find('"').unwrap()].to_string()
}

#[tokio::test]
async fn registration_only_without_mail() {
    let dir = TempDir::new().unwrap();
    let url = format!("sqlite://{}?mode=rwc", dir.path().join("t.db").display());
    std::env::set_var("DATABASE_URL", &url);
    std::env::set_var("FORTIFY_SECRET", "smoke-secret-change-me");
    std::env::set_var("SOVA_LOG", "off");

    // Apply auth migrations the same way sova-testing does (file stem collisions).
    {
        use sea_orm::{Database, Statement};
        use sea_orm_migration::{MigratorTrait, SchemaManager};
        let conn = Database::connect(&url).await.unwrap();
        let schema = SchemaManager::new(&conn);
        for m in AuthMigrator::migrations() {
            m.up(&schema).await.unwrap();
        }
        let _ = conn.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1".into(),
        )).await;
    }

    let mut app = App::web()
        .site("HN Smoke")
        .public_url("http://127.0.0.1:3000")
        .into_app();
    app.install(Db::from_env().migrations::<AuthMigrator>());
    app.install(
        Fortify::new()
            .features([AuthFeature::Registration])
            .web_forms(true)
            .no_api()
            .home("/")
            .login_redirect("/login")
            .secret("smoke-secret-change-me"),
    );
    let mut r = Router::new();
    r.use_middleware(Fortify::guard());
    r.get("/", || async { Html("<h1>ok</h1>".into()) });
    app.mount("/me", r);
    app.get("/", || async { Html("<a href=/me>me</a>".into()) });

    let c = TestClient::boot(app.into()).await.unwrap();
    assert_eq!(c.get("/me").await.status_code().as_u16(), 303);

    let reg = c.get("/register").await;
    reg.assert_status(200);
    let csrf = csrf_from_html(&String::from_utf8_lossy(reg.body_bytes().unwrap()));
    let done = c
        .post("/register")
        .form(&[
            ("csrf", csrf.as_str()),
            ("name", "Ada"),
            ("email", "ada@example.com"),
            ("password", "secret123"),
            ("password_confirmation", "secret123"),
        ])
        .await;
    assert_eq!(done.status_code().as_u16(), 303);

    let me = c.get("/me").await;
    me.assert_status(200);
    assert!(String::from_utf8_lossy(me.body_bytes().unwrap()).contains("ok"));
}
RS

cargo test --test smoke -- --nocapture
echo "OK crates.io HN-shaped smoke ($DIR)"
