//! Blog-style meta: Article JSON-LD, sitemap, robots.
//!
//! ```ignore
//! cargo run -p ruvo --example meta_blog --features meta
//! ```

use ruvo::{render_html, schema, App, Meta, MetaExt, Request, Response, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    app.install(
        Meta::new()
            .site_name("Ruvo Blog")
            .title_template("{} — Ruvo Blog")
            .public_url("http://127.0.0.1:3000")
            .twitter_site("@ruvo")
            .provider("/blog/:slug", |_ctx| async move {
                Ok(vec![
                    ruvo::Entry::new("/blog/hello"),
                    ruvo::Entry::new("/blog/meta"),
                ])
            }),
    );

    app.get("/", |mut req: Request| async move {
        req.meta()
            .title("Home")
            .description("A tiny Ruvo blog with document meta.");
        Response::html(format!(
            "<!doctype html><html><head>{}</head><body><h1>Ruvo Blog</h1>\
             <p><a href=\"/blog/hello\">hello</a></p></body></html>",
            render_html(&req.resolved_meta())
        ))
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
                author: Some("Ruvo".into()),
                ..Default::default()
            })
            .jsonld_schema(&schema::BreadcrumbList::from_pairs(&[
                ("Home", "/"),
                (title.as_str(), path.as_str()),
            ]));
        Response::html(format!(
            "<!doctype html><html><head>{}</head><body><h1>{}</h1>\
             <p><a href=\"/sitemap.xml\">sitemap</a> · <a href=\"/robots.txt\">robots</a></p>\
             </body></html>",
            render_html(&req.resolved_meta()),
            title
        ))
    });

    app.get("/about", |_req: Request| async {
        Response::html("<h1>About</h1>")
    })
    .with(
        Meta::page()
            .title("About")
            .description("About this blog"),
    );

    println!("meta blog on http://127.0.0.1:3000 — try /blog/hello, /sitemap.xml, /robots.txt");
    app.listen(3000).await
}
