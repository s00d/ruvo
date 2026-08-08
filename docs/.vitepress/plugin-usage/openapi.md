**`App::api()`** already mounts OpenAPI + Scalar at `/docs`. Your job is schemas + `.doc(...)` on routes — not `OpenApi::new(...)`.

```rust
use ruvo::prelude::*;
use ruvo::vld;
use ruvo::{
    doc_schema, Doc, DocVldExt, Json, OpenApiDocExt, Parser, Request, ServerArgs,
    ValidationError, ValidationExt,
};

mod modules;

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct CreateUser {
        pub name: String => vld::string().min(2).max(50),
        pub email: String => vld::string().email(),
    }
}

doc_schema!(CreateUser);

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::api().title("Users API").version("1.0");
    modules::register(&mut app);
    app.run().await
}
```

```rust
// modules/mod.rs
use crate::CreateUser;
use ruvo::{
    App, Doc, DocVldExt, Json, OpenApiDocExt, Request, ValidationError, ValidationExt,
};

pub fn register(app: &mut App) {
    app.post("/users", create)
        .doc(Doc::new().body::<CreateUser>().created::<CreateUser>());
}

async fn create(
    mut req: Request,
) -> std::result::Result<(u16, Json<CreateUser>), ValidationError> {
    let body: CreateUser = req.validate().await?;
    Ok((201, Json(body)))
}
```

Runnable: `cargo run -p api_preset`. Only install `OpenApi` yourself when you intentionally skip `App::api()`.
