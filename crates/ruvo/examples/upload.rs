//! Multipart file upload demo.
use ruvo::{App, Json, MultipartExt, Request, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.max_body_size(8 * 1024 * 1024);

    app.get("/", |_| async {
        ruvo::Response::html(include_str!("upload/views/index.html"))
    });
    app.post("/upload", upload);
    app.listen(3004).await
}

async fn upload(mut req: Request) -> Result<Json<serde_json::Value>> {
    let fields = req.multipart().await?;
    let files: Vec<_> = fields
        .iter()
        .filter(|f| f.filename.is_some())
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "filename": f.filename,
                "bytes": f.data.len(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "files": files, "fields": fields.len() })))
}
