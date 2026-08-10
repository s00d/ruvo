//! CRUD with `Db::from_env` + `req.db()` (SeaORM).
//!
//! ```bash
//! cargo run -p crud -- migrate
//! cargo run -p crud
//! ```
//!
//! DB: `sova.toml` `[db] url` (sqlite). Override with `DATABASE_URL`.

use sova::vld;
mod entity;
mod migrator;

use entity::prelude::*;
use migrator::Migrator;
use sova::{
    ActiveModelTrait, App, Db, DbExt, EntityTrait, Error, Json, Request, Result, Set, ValidationExt,
};
use std::path::PathBuf;

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct CreateUser {
        pub name: String => vld::string().min(1),
        pub email: String => vld::string().email(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = sova_env::load();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut app = App::new();
    let _ = app.configure_from_path(root.join("sova.toml"));
    app.install(Db::from_env().migrations::<Migrator>());
    app.get("/users", list)
        .post("/users", create)
        .get("/users/:id", show);
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

async fn show(req: Request) -> Result<Json<Model>> {
    let id = req.param_as::<i32>("id")?;
    let u = User::find_by_id(id)
        .one(req.db())
        .await?
        .ok_or(Error::NotFound)?;
    Ok(Json(u))
}
