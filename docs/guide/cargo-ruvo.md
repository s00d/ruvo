# cargo-ruvo

![cargo-ruvo](/banners/cargo-ruvo.svg)

Workspace CLI for scaffolding, watch, build, and DB helpers.

## Generate

```bash
cargo ruvo generate plugin hello
cargo ruvo generate mailer Welcome
cargo ruvo generate job ProcessOrder   # alias: worker
cargo ruvo generate migration add_status_to_posts
cargo ruvo generate migration create_tags --fields name:string
cargo ruvo generate seed DemoUsers
cargo ruvo generate resource post --fields name:string,body:text
cargo ruvo generate resource post --api
# `generate crud <name>` → alias of `resource --api`
```

| Generate | Writes |
|----------|--------|
| `module <name>` | `src/modules/{name}.rs` + register |
| `plugin <name>` | `plugins/ruvo-{name}/` |
| `model <name> --fields …` | entity + SeaORM migration |
| `migration <name>` | `src/migrations/…` |
| `seed <Name>` | `src/seeds/…` |
| `mailer <Name>` | Mailable + `views/mail/…` |
| `job` / `worker <Name>` | `src/jobs/…` |
| `resource` / `crud` | module + routes (+ JSON with `--api`) |

## Run / build

| Command | What |
|---------|------|
| `cargo ruvo dev -p <pkg>` | watch `.rs` / `.env*` / `ruvo.toml`; Vite if `frontend/`; Unix graceful overlap (`SO_REUSEPORT`) |
| `cargo ruvo build -p <pkg>` | frontend + `cargo build --release` |
| `cargo ruvo serve -p <pkg>` | release binary (`RUVO_ENV=production`) |
| `cargo ruvo db migrate -p <pkg>` | apply migrations |
| `cargo ruvo db down -p <pkg> [N]` | roll back |
| `cargo ruvo db status -p <pkg>` | applied/pending table |
| `cargo ruvo db seed -p <pkg>` | run `seed` CLI |

Optional `[frontend]` in `ruvo.toml` (`enabled = false` to force off).

### Hot reload

| Layer | Mechanism |
|-------|-----------|
| Templates / i18n | In-process FS watch |
| Code / `.env*` / `ruvo.toml` | process restart via `dev` |
| Graceful (Unix default) | new process + `/ready` + SIGTERM old |
