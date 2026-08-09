# Docker template for Sova apps

These files are a **starting point for applications that use Sova**, not an image of the framework monorepo.

1. Create your app (`cargo sovax new myapp --web` / `--api`, or any binary crate with `sova` in `Cargo.toml`).
2. Copy this folder’s files into the **app repository root** (next to `Cargo.toml`):

```bash
cp deploy/Dockerfile deploy/docker-compose.yml deploy/sova.toml deploy/.env.example deploy/.dockerignore /path/to/myapp/
cd /path/to/myapp
mv .env.example .env
```

3. Set `APP_BIN` in `docker-compose.yml` to your binary name (usually the package name).
4. In `main`, load config so the mounted `sova.toml` applies:

```rust
let mut app = App::new(); // or App::web() / App::api() — presets already configure()
let _ = app.configure();  // picks up ./sova.toml
```

5. Run:

```bash
docker compose up --build -d
curl -sS http://127.0.0.1:3000/
docker compose down
```

Uncomment the `db` service in compose when you need Postgres; set `DATABASE_URL` in `.env`.

Full guide: [Production / Docker](https://s00d.github.io/sova/guide/production.html).
