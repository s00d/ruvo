[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

# Hacker News–style Sova demo

Minimal news site: **stories**, **upvotes**, **comments**, session auth via Fortify
(`Registration` only — no Mail / 2FA / reset / roles).

```bash
cargo run -p hackernews
# http://127.0.0.1:3000
# Accounts (seeded): demo@sova.news / demo1234 · alice@sova.news / alice1234 · bob@sova.news / bob12345
```

Auto migrate + seed on startup (`Db::migrate_on_startup` / `seed_on_startup` + `[db]` in [`sova.toml`](./sova.toml)). Override DB with `DATABASE_URL`.

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
