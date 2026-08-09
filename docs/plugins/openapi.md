---
title: openapi
editLink: false
---

# `openapi`

**OpenAPI 3.1 document + Scalar UI at mount path**

| | |
|--|--|
| Crate | [`sova-openapi`](https://docs.rs/sova-openapi/0.1.1) `0.1.1` |
| Plugin id | `openapi` |
| Category | Content |

## Install

```bash
cargo add sova --features openapi
```

## Features

| Feature | What you get |
|---------|-------------|
| `openapi` | OpenAPI 3.1 document + Scalar UI. |

## Overview

**When:** OpenAPI 3.1 + Scalar UI for APIs.

**Does:**
- Document from routes / vld schemas
- UI at mount path

### Example

```rust
app.install(OpenApi::new().mount("/docs"));
```

## Quick start

**`App::api()`** already mounts OpenAPI + Scalar at `/docs` (JSON at `/docs/openapi.json`). Your job is schemas + `.doc(...)` on routes — not `OpenApi::new(...)`.

```rust
use sova::prelude::*;
use sova::vld;
use sova::{
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
use sova::{
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

## Examples

- `examples/api/api_preset`

## Related

[`vld`](/plugins/vld) · [`meta`](/plugins/meta)
