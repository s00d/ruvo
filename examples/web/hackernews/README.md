# Hacker News–style Sova demo

Minimal news site: **stories**, **upvotes**, **comments**, session auth via Fortify
(`Registration` only — no Mail / 2FA / reset / roles).

```bash
cd examples/web/hackernews
export DATABASE_URL="sqlite:./hn.db?mode=rwc"
export FORTIFY_SECRET="dev-hn-secret-change-me"
cargo run -p hackernews -- migrate
cargo run -p hackernews -- seed   # optional: demo@sova.news / demo1234
cargo run -p hackernews
# http://127.0.0.1:3000
```

| Path | What |
|------|------|
| `/` | Top stories (by points, then age) |
| `/newest` | Newest first |
| `/submit` | Submit a story (auth) |
| `/item/:id` | Story + comments + upvote |
| `/login` `/register` | Fortify HTML forms (`web_forms(true)`) |
| `POST /logout` | Fortify logout |

Layout:

```
src/app.rs          App::web + Db + Fortify
src/modules/        feed / submit / item
src/entity/         stories, votes, comments
src/db.rs           query helpers (uses find_user_by_id)
src/migrate.rs      AuthMigrator + HN tables
src/seed.rs         demo user + sample stories
views/ + public/    MiniJinja + hn.css
tests/smoke.rs      register → submit → vote → comment
```

```bash
cargo test -p hackernews
./scripts/release-smoke-hn.sh
```
