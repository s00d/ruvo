# cargo-sovax

![cargo-sovax](/banners/cargo-sovax.svg)

Install as `cargo-sovax`, run as **`cargo sovax …`** (Cargo subcommand).  
Scaffolding, watch, build, and DB helpers for Sova apps.

In-app argv helpers (`ServerArgs`, `--log-level`) are the separate crate [`sovax`](/plugins/cli) via `sova` feature `cli` — not this binary.

## Generate

```bash
cargo sovax generate plugin hello
cargo sovax generate mailer Welcome
cargo sovax generate job ProcessOrder   # alias: worker
cargo sovax generate migration add_status_to_posts
cargo sovax generate migration create_tags --fields name:string
cargo sovax generate seed DemoUsers
cargo sovax generate resource post --fields name:string,body:text
cargo sovax generate resource post --api
# `generate crud <name>` → alias of `resource --api`
```

| Generate | Writes |
|----------|--------|
| `module <name>` | `src/modules/{name}.rs` + register |
| `plugin <name>` | `plugins/sova-{name}/` |
| `model <name> --fields …` | entity + SeaORM migration |
| `migration <name>` | `src/migrations/…` |
| `seed <Name>` | `src/seeds/…` |
| `mailer <Name>` | Mailable + `views/mail/…` |
| `job` / `worker <Name>` | `src/jobs/…` |
| `resource` / `crud` | module + routes (+ JSON with `--api`) |

## Run / build

| Command | What |
|---------|------|
| `cargo sovax dev -p <pkg>` | watch `.rs` / `.env*` / `sova.toml`; Vite if `frontend/`; Unix graceful overlap (`SO_REUSEPORT`) |
| `cargo sovax build -p <pkg>` | frontend + `cargo build --release` |
| `cargo sovax serve -p <pkg>` | release binary (`SOVA_ENV=production`) |
| `cargo sovax db migrate -p <pkg>` | apply migrations |
| `cargo sovax db down -p <pkg> [N]` | roll back |
| `cargo sovax db status -p <pkg>` | applied/pending table |
| `cargo sovax db seed -p <pkg>` | run `seed` CLI |

Optional `[frontend]` in `sova.toml` (`enabled = false` to force off).

### Hot reload

| Layer | Mechanism |
|-------|-----------|
| Templates / i18n | In-process FS watch |
| Code / `.env*` / `sova.toml` | process restart via `dev` |
| Graceful (Unix default) | new process + `/ready` + SIGTERM old |
