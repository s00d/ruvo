Validation belongs next to routes on **`App::api()`** (or web + `Vld` when you need forms).

```rust
use sova::prelude::*;
use sova::vld;
use sova::{
    doc_schema, Doc, DocVldExt, Json, OpenApiDocExt, Parser, Request, ServerArgs,
    ValidationError, ValidationExt,
};

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

    let mut app = App::api().title("Users").version("1.0");
    app.post("/users", create)
        .doc(Doc::new().body::<CreateUser>().created::<CreateUser>());
    app.run().await
}

async fn create(
    mut req: Request,
) -> std::result::Result<(u16, Json<CreateUser>), ValidationError> {
    let body: CreateUser = req.validate().await?;
    Ok((201, Json(body)))
}
```

HTML forms (web preset already has session/csrf):

```rust
async fn store(mut req: Request) -> Result<Redirect> {
    let form: NoteForm = req.validate_form().await?;
    req.flash_status("Saved");
    Ok(Redirect::back(&req))
}
```

Features: `vld-openapi`, `vld-flash`, `vld-form`, `vld-i18n`.
