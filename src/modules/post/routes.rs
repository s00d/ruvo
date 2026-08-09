use super::dto::{Create, IdParams, Update};
use super::handlers;
use sova::{Doc, DocVldExt, OpenApiDocExt, Router};

pub fn routes() -> Router {
    let mut r = Router::new();
    r.get("/", handlers::list).doc(Doc::new());
    r.post("/", handlers::create)
        .doc(Doc::new().body::<Create>().created_schema(serde_json::json!({ "type": "object" })));
    r.get("/:id", handlers::show)
        .doc(Doc::new().params::<IdParams>());
    r.put("/:id", handlers::update)
        .doc(Doc::new().params::<IdParams>().body::<Update>());
    r.delete("/:id", handlers::destroy)
        .doc(Doc::new().params::<IdParams>());
    r
}
