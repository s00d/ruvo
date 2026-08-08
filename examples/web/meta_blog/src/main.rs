//! Blog-style meta: Article JSON-LD, sitemap, robots (head inject).
//!
//! ```ignore
//! cargo run -p meta_blog
//! ```

use sova::{schema, App, Html, Meta, MetaExt, Request, Result, Robots, Sitemap};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.install(
        Meta::new()
            .site_name("Sova Blog")
            .title_template("{} — Sova Blog")
            .public_url("http://127.0.0.1:3000")
            .twitter_site("@sova"),
    );
    app.install(
        Sitemap::new().provider("/blog/:slug", |_ctx| async move {
            Ok(vec![
                sova::Entry::new("/blog/hello"),
                sova::Entry::new("/blog/meta"),
            ])
        }),
    );
    app.install(Robots::new());

    app.get("/", |mut req: Request| async move {
        req.meta()
            .title("Home")
            .description("A tiny Sova blog with document meta.");
        Html(
            "<h1>Sova Blog</h1><p><a href=\"/blog/hello\">hello</a></p>".to_string(),
        )
    })
    .with(Meta::page().title("Home").description("Blog home"));

    app.get("/blog/:slug", |mut req: Request| async move {
        let slug = req.params.get("slug").cloned().unwrap_or_default();
        let title = format!("Post {slug}");
        let path = format!("/blog/{slug}");
        req.meta()
            .title(title.clone())
            .description(format!("Article about {slug}"))
            .jsonld_schema(&schema::Article {
                headline: title.clone(),
                author: Some("Sova".into()),
                ..Default::default()
            })
            .jsonld_schema(&schema::BreadcrumbList::from_pairs(&[
                ("Home", "/"),
                (title.as_str(), path.as_str()),
            ]));
        Html(format!(
            "<h1>{title}</h1>\
             <p><a href=\"/sitemap.xml\">sitemap</a> · <a href=\"/robots.txt\">robots</a></p>"
        ))
    });

    app.get("/about", || async { Html("<h1>About</h1>".to_string()) })
        .with(
            Meta::page()
                .title("About")
                .description("About this blog"),
        );

    println!("meta blog on http://127.0.0.1:3000 — try /blog/hello, /sitemap.xml, /robots.txt");
    app.listen(3000).await
}
