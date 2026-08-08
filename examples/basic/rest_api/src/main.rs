//! JSON REST skeleton (`Cell` for in-memory store).
use sova::prelude::*;
use sova::Cell;

#[derive(Clone)]
struct Db {
    items: Cell<Vec<String>>,
}

impl Default for Db {
    fn default() -> Self {
        Self {
            items: Cell::new(Vec::new()),
        }
    }
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
    Json(db.items.get())
}

async fn create(mut req: Request) -> Result<(u16, Json<serde_json::Value>)> {
    #[derive(serde::Deserialize)]
    struct Body {
        name: String,
    }
    let body: Body = req.json().await?;
    let db = req.state::<Db>();
    let mut n = 0;
    db.items.update(|items| {
        let mut next = items.clone();
        next.push(body.name);
        n = next.len();
        next
    });
    Ok((201, Json(serde_json::json!({ "ok": true, "n": n }))))
}

async fn show(req: Request) -> Option<Json<serde_json::Value>> {
    let id: usize = req.param_as("id").unwrap_or(0);
    let db = req.state::<Db>();
    db.items
        .get()
        .get(id)
        .cloned()
        .map(|name| Json(serde_json::json!({ "id": id, "name": name })))
}
