//! Form / multipart upload demo (`Request::input`, [`Upload`]).
use sova::{App, Json, Request, Result};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.max_body_size(8 * 1024 * 1024);

    app.get("/", |_| async {
        sova::Response::html(include_str!("views/index.html"))
    });
    app.post("/upload", upload);
    app.get("/sample", |_| async {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        sova::Response::download(path).await
    });
    app.listen(3004).await
}

async fn upload(mut req: Request) -> Result<Json<serde_json::Value>> {
    let data = req.input().await?;
    let files: Vec<_> = data
        .file_map()
        .values()
        .flatten()
        .map(|f| {
            serde_json::json!({
                "field": f.field,
                "filename": f.filename,
                "bytes": f.data.len(),
            })
        })
        .collect();
    let fields: serde_json::Map<String, serde_json::Value> = data
        .text_map()
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::Value::Array(v.iter().cloned().map(Into::into).collect()),
            )
        })
        .collect();
    Ok(Json(serde_json::json!({
        "files": files,
        "fields": fields,
    })))
}
