**When:** SeaORM pool + migrate/seed CLI.

**Does:**
- `Db::from_env().migrations::<Migrator>()`
- `req.db()` in handlers
- postgres (default) / mysql / sqlite features
- `cargo sovax db migrate|seed`

### Example

```rust
app.install(Db::from_env().migrations::<Migrator>());
let u = User::find_by_id(id).one(req.db()).await?;
```

### Config

```bash
DATABASE_URL=postgres://postgres@localhost/sova
```
