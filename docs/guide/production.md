# Production

How to ship **your** Sova application — release binary and Docker Compose.  
The [`deploy/`](https://github.com/s00d/sova/tree/master/deploy) folder is a **template for app repos**, not a container of the framework itself.

## Local release

```bash
export SOVA_ENV=production
cargo build --release -p your_app
./target/release/your_app

# or
cargo sovax serve -p your_app
```

Use `[production.*]` in `sova.toml` ([Configuration](/guide/configuration)). Secrets stay in env (`DATABASE_URL`, SMTP, …), not in git.

- DevTools off in production (default for release / `SOVA_PROFILE=production`)
- Behind a reverse proxy: `trust_proxy = true`
- `app.listen(port)` binds `0.0.0.0`

---

## Docker Compose (app template)

Copy the template into **your application repository** (the crate that depends on `sova`), then build that app:

```bash
# from the Sova repo (or download the raw files)
cp deploy/Dockerfile deploy/docker-compose.yml deploy/sova.toml \
   deploy/.env.example deploy/.dockerignore /path/to/myapp/

cd /path/to/myapp
mv .env.example .env
# edit docker-compose.yml → build.args.APP_BIN = your binary name
docker compose up --build -d
curl -sS http://127.0.0.1:3000/
docker compose logs --tail=50
docker compose down
```

| File | In your app |
|------|-------------|
| [`Dockerfile`](https://github.com/s00d/sova/blob/master/deploy/Dockerfile) | multi-stage `cargo build --release` of **your** crate |
| [`docker-compose.yml`](https://github.com/s00d/sova/blob/master/deploy/docker-compose.yml) | `app` service (+ optional Postgres, commented) |
| [`sova.toml`](https://github.com/s00d/sova/blob/master/deploy/sova.toml) | mounted at `/app/sova.toml` |
| [`.env.example`](https://github.com/s00d/sova/blob/master/deploy/.env.example) | copy to `.env` for secrets |
| [`.dockerignore`](https://github.com/s00d/sova/blob/master/deploy/.dockerignore) | keeps `target/` out of the build context |

### App checklist

1. Binary crate with `Cargo.toml` + `src/` (what `cargo sovax new` scaffolds).
2. `APP_BIN` in compose matches `[[bin]]` / package name.
3. Load config in `main`:

```rust
let mut app = App::new();
let _ = app.configure(); // reads ./sova.toml (in the image: /app/sova.toml)
app.listen(3000).await?;
```

Presets `App::web()` / `App::api()` already call `configure()`.

4. Uncomment the `db` block in compose when you need Postgres; set `DATABASE_URL` in `.env`.
5. Publish **your** image from **your** CI (`docker build` / GHCR) — Sova does not publish a framework runtime image for you to `FROM`.

### Image layout (what the template builds)

1. **builder** — `rust:*-bookworm`, `cargo build --release` in the app context  
2. **runtime** — slim Debian, non-root user, binary + `sova.toml`, `EXPOSE 3000`
