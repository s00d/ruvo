//! GraphQL outbound client demo (fake in default; set GRAPHQL_URL for live).

use sova::{FakeGraphql, GraphQl, GraphQlExt, App, Json, Request, Result};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    if let Ok(url) = std::env::var("GRAPHQL_URL") {
        app.install(GraphQl::client(url));
    } else {
        let fake = FakeGraphql::new().stub(
            "hello",
            json!({ "hello": "from sova-graphql (fake)" }),
        );
        app.install(GraphQl::fake(fake));
    }

    app.get("/api/hello", |req: Request| async move {
        let data = req
            .graphql()
            .query("query { hello }")
            .data()
            .await?;
        Ok::<_, sova::Error>(Json(data))
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    eprintln!("api_graphql listening on http://127.0.0.1:{port}");
    app.listen(port).await
}
