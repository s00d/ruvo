# Production

Release builds and Docker Compose for Sova apps.

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

## Docker Compose (recommended)

Reference stack lives in the repo under [`deploy/`](https://github.com/s00d/sova/tree/master/deploy).  
**Compose is the entrypoint** — not a file dump.

| File | Role |
|------|------|
| [`docker-compose.yml`](https://github.com/s00d/sova/tree/master/deploy/docker-compose.yml) | `app` service: image or local build, port `3000`, prod env |
| [`Dockerfile`](https://github.com/s00d/sova/tree/master/deploy/Dockerfile) | multi-stage build of the demo binary |
| [`sova.production.toml`](https://github.com/s00d/sova/tree/master/deploy/sova.production.toml) | mounted as `/app/sova.toml` |
| [`.env.example`](https://github.com/s00d/sova/tree/master/deploy/.env.example) | copy to `.env` for secrets / overrides |

### Option A — pull published image

Demo image: [`ghcr.io/s00d/sova-hello`](https://github.com/s00d/sova/pkgs/container/sova-hello) (built from `examples/basic/hello`).

```bash
git clone https://github.com/s00d/sova.git && cd sova/deploy
cp .env.example .env   # optional
docker compose pull
docker compose up -d
curl -sS http://127.0.0.1:3000/
curl -sS http://127.0.0.1:3000/healthz
docker compose logs -f --tail=50
docker compose down
```

### Option B — build from this repo

```bash
cd deploy   # or: docker compose -f deploy/docker-compose.yml … from repo root
docker compose up --build -d
curl -sS http://127.0.0.1:3000/
docker compose down
```

Build another example binary:

```bash
docker compose build --build-arg EXAMPLE_PKG=fs_demo --build-arg EXAMPLE_BIN=fs_demo
```

### What the stack does

1. Starts one `app` container on **`:3000`**
2. Sets `SOVA_ENV=production`, `RUST_LOG=info`
3. Mounts `sova.production.toml` → `/app/sova.toml` (hello calls `configure()` and picks it up)
4. `restart: unless-stopped`

Health: hello enables probes — try `/healthz` / `/ready` (see [getting started](/guide/getting-started) probes).

### Your own app

1. Copy `deploy/Dockerfile`, `deploy/docker-compose.yml`, and `sova.production.toml` into your project
2. Point compose `build.context` at your crate root; set `EXAMPLE_PKG` / `EXAMPLE_BIN` (or replace the `RUN cargo build` line)
3. In `main`: `let _ = app.configure();` or `configure_from_path("sova.toml")` so the mounted toml applies
4. Add Postgres/Redis next to `app` when you need them:

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD: sova
      POSTGRES_DB: sova
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5
  app:
    environment:
      DATABASE_URL: postgres://postgres:sova@db:5432/sova
    depends_on:
      db:
        condition: service_healthy
```

Keep credentials in `.env`, not in the image.
