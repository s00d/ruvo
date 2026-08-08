# cargo-sova

![cargo-sova](/banners/cargo-sova.svg)

Workspace CLI for scaffolding, watch, build, and DB helpers.

## Generate

```bash
cargo sova generate plugin hello
cargo sova generate mailer Welcome
cargo sova generate job ProcessOrder   # alias: worker
cargo sova generate migration add_status_to_posts
cargo sova generate migration create_tags --fields name:string
cargo sova generate seed DemoUsers
cargo sova generate resource post --fields name:string,body:text
cargo sova generate resource post --api
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
| `cargo sova dev -p <pkg>` | watch `.rs` / `.env*` / `sova.toml`; Vite if `frontend/`; Unix graceful overlap (`SO_REUSEPORT`) |
| `cargo sova build -p <pkg>` | frontend + `cargo build --release` |
| `cargo sova serve -p <pkg>` | release binary (`SOVA_ENV=production`) |
| `cargo sova db migrate -p <pkg>` | apply migrations |
| `cargo sova db down -p <pkg> [N]` | roll back |
| `cargo sova db status -p <pkg>` | applied/pending table |
| `cargo sova db seed -p <pkg>` | run `seed` CLI |

Optional `[frontend]` in `sova.toml` (`enabled = false` to force off).

### Hot reload

| Layer | Mechanism |
|-------|-----------|
| Templates / i18n | In-process FS watch |
| Code / `.env*` / `sova.toml` | process restart via `dev` |
| Graceful (Unix default) | new process + `/ready` + SIGTERM old |
