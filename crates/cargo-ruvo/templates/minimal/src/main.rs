use ruvo::prelude::*;

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.get("/", || async { "ok" });
    modules::register(&mut app);
    app.listen(3000).await
}
