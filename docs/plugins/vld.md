---
title: vld
editLink: false
---

# `vld`

**Request validation hooks and coverage check** · crate `sova-vld` `0.1.0` · id `vld`

```bash
cargo add sova --features vld,vld-flash,vld-flash-templates,vld-form,vld-i18n,vld-openapi
```

| Feature | What you get |
|---------|-------------|
| `vld` | Request validation (`sova_vld` + `vld`). |
| `vld-flash` | Validation errors into session flash. |
| `vld-flash-templates` | Flash helpers in MiniJinja. |
| `vld-form` | Bind validation to multipart/form input. |
| `vld-i18n` | Localized validation messages. |
| `vld-openapi` | Validation ↔ OpenAPI schema sugar. |

Request validation via [`vld`].

## Usage

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
