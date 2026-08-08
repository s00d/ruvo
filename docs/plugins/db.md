---
title: db
editLink: false
---

# `db`

**SeaORM pool, migrate CLI, optional seed CLI** · crate `sova-db` · id `db`

```bash
cargo add sova --features db,db-mysql,db-sqlite
```

| Feature | What you get |
|---------|-------------|
| `db` | SeaORM pool (`sova-db`; postgres by default). |
| `db-mysql` | MySQL backend for `sova-db`. |
| `db-sqlite` | SQLite backend for `sova-db`. |

SeaORM database plugin for Sova (postgres / sqlite / mysql via Cargo features).

```rust
 app.install(Db::from_env().migrations::<Migrator>());
 let u = User::find_by_id(id).one(req.db()).await?;
 ```

## Usage

Install Db **on top of** a preset (or cabinet-style app). Migrations run through `app.run()` / `cargo sovax db`.

```rust
use sova::prelude::*;
use sova::{
    ActiveModelTrait, Db, DbExt, EntityTrait, Json, Parser, Request, ServerArgs, Set,
    ValidationExt,
};
use sova::vld;

mod entity;
mod migrator;

use entity::prelude::*;
use migrator::Migrator;

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct CreateUser {
        pub name: String => vld::string().min(1),
        pub email: String => vld::string().email(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::api().title("Users").version("1.0").into_app();
    app.install(Db::from_env().migrations::<Migrator>());

    app.get("/users", list).post("/users", create);
    app.run().await
}

async fn list(req: Request) -> Result<Json<Vec<Model>>> {
    Ok(Json(User::find().all(req.db()).await?))
}

async fn create(mut req: Request) -> Result<(u16, Json<Model>)> {
    let body: CreateUser = req.validate().await?;
    let row = ActiveModel {
        name: Set(body.name),
        email: Set(body.email),
        ..Default::default()
    }
    .insert(req.db())
    .await?;
    Ok((201, Json(row)))
}
```

```bash
DATABASE_URL=postgres://postgres@localhost/sova cargo run -p crud -- migrate
cargo run -p crud
```

Compose migrators in one `MigratorTrait` (see cabinet). Features: `db-sqlite`, `db-mysql`.
