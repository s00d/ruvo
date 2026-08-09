# Production

Build and run a Sova app for production — locally and with Docker.

## Local release

```bash
# profile: SOVA_ENV / SOVA_PROFILE → production (aliases: release)
export SOVA_ENV=production
cargo build --release -p your_app
./target/release/your_app

# or
cargo sovax serve -p your_app   # sets SOVA_ENV=production
```

Use `[production.*]` sections in `sova.toml` (see [Configuration](/guide/configuration)). Keep secrets in the environment (`DATABASE_URL`, SMTP, …), not in git.

Checklist:

- DevTools off in production (default for release / `SOVA_PROFILE=production`)
- Behind a reverse proxy: `trust_proxy = true` when needed
- Bind is `0.0.0.0` via `app.listen(port)` (all interfaces)

## Docker

Base configs live in [`deploy/`](https://github.com/s00d/sova/tree/master/deploy): multi-stage `Dockerfile`, `docker-compose.yml`, `sova.production.toml`, `.env.example`.

Default image builds the lightweight `hello` example:

```bash
cd deploy
cp .env.example .env   # optional
docker compose up --build -d
curl -sS http://127.0.0.1:3000/
docker compose logs --tail=50
docker compose down
```

Override the package:

```bash
docker compose build --build-arg EXAMPLE_PKG=fs_demo --build-arg EXAMPLE_BIN=fs_demo
# or set ARG in Dockerfile when copying into your app
```

Compose sets `SOVA_ENV=production`. The image includes `/app/sova.toml` (from `sova.production.toml`) for copy-paste; there is no `SOVA_CONFIG` env — load it in code with `configure()` / `configure_from_path`. Default `hello` does not load toml. Uncomment the compose volume to override the file for apps that do.

Copy `deploy/Dockerfile` into your app and change `EXAMPLE_*` / `COPY` paths to match your crate.

## Image layout

1. **builder** — `rust:1-bookworm`, `cargo build --release -p <pkg>`
2. **runtime** — `debian:bookworm-slim`, binary + baked-in toml, `EXPOSE 3000`, `CMD` the binary

For apps with Postgres/Redis, add services next to `app` in compose (cabinet-style); keep secrets in `.env`, not in the image.
