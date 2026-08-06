//! JSON REST skeleton.
use ruvo::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Db {
    items: Arc<Mutex<Vec<String>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.state(Db::default());

    app.get("/items", list);
    app.post("/items", create);
    app.get("/items/:id", show);
    app.listen(3001).await
}

async fn list(req: Request) -> Json<Vec<String>> {
    let db = req.state::<Db>();
    let items = db.items.lock().unwrap().clone();
    Json(items)
}

async fn create(mut req: Request) -> Result<(u16, Json<serde_json::Value>)> {
    #[derive(serde::Deserialize)]
    struct Body {
        name: String,
    }
    let body: Body = req.json().await?;
    let db = req.state::<Db>();
    let mut items = db.items.lock().unwrap();
    items.push(body.name);
    let n = items.len();
    Ok((201, Json(serde_json::json!({ "ok": true, "n": n }))))
}

async fn show(req: Request) -> Option<Json<serde_json::Value>> {
    let id: usize = req.param_as("id").unwrap_or(0);
    let db = req.state::<Db>();
    let items = db.items.lock().unwrap();
    items
        .get(id)
        .map(|name| Json(serde_json::json!({ "id": id, "name": name })))
}
