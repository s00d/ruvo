//! Local FS via `Fs` + `req.fs()`.
//!
//! ```bash
//! cargo run -p fs_demo
//! # curl 'http://127.0.0.1:3011/list?path=notes'
//! # curl -X POST 'http://127.0.0.1:3011/write?path=notes/hi.txt' -d 'hello'
//! ```

use serde_json::json;
use sova::prelude::*;
use sova::{Fs, FsExt};

#[tokio::main]
async fn main() -> Result<()> {
    let root = std::env::var("SOVA_FS_ROOT").unwrap_or_else(|_| "./data".into());
    let mut app = App::new();
    app.install(Fs::new(root));

    app.get("/", || async {
        Json(json!({
            "ok": true,
            "hint": "GET /list?path= · GET /walk?path= · GET /read?path= · POST /write?path= (body) · DELETE /rm?path="
        }))
    });

    app.get("/list", |req: Request| async move {
        let path = req.query("path").unwrap_or("");
        let entries = req.fs().read_dir(path).await?;
        let rows: Vec<_> = entries
            .into_iter()
            .map(|e| {
                json!({
                    "path": e.path,
                    "name": e.name,
                    "is_dir": e.is_dir,
                    "len": e.len,
                })
            })
            .collect();
        Ok::<_, Error>(Json(json!({ "path": path, "entries": rows })))
    });

    app.get("/walk", |req: Request| async move {
        let path = req.query("path").unwrap_or("");
        let entries = req.fs().walk(path).await?;
        let rows: Vec<_> = entries
            .into_iter()
            .map(|e| json!({ "path": e.path, "is_dir": e.is_dir, "len": e.len }))
            .collect();
        Ok::<_, Error>(Json(json!({ "path": path, "entries": rows })))
    });

    app.get("/read", |req: Request| async move {
        let path = req
            .query("path")
            .ok_or_else(|| Error::BadRequest("query path required".into()))?;
        let data = req.fs().read_to_string(path).await?;
        Ok::<_, Error>(Json(json!({ "path": path, "data": data })))
    });

    app.post("/write", |mut req: Request| async move {
        let path = req
            .query("path")
            .ok_or_else(|| Error::BadRequest("query path required".into()))?
            .to_string();
        let body = req.body().await?;
        req.fs().write(&path, &body).await?;
        Ok::<_, Error>(Json(
            json!({ "ok": true, "path": path, "bytes": body.len() }),
        ))
    });

    app.delete("/rm", |req: Request| async move {
        let path = req
            .query("path")
            .ok_or_else(|| Error::BadRequest("query path required".into()))?;
        let meta = req.fs().metadata(path).await?;
        if meta.is_dir {
            req.fs().remove_dir(path).await?;
        } else {
            req.fs().remove_file(path).await?;
        }
        Ok::<_, Error>(Json(json!({ "ok": true, "path": path })))
    });

    app.listen(3011).await
}
