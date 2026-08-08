//! Shared HTML/JSON bodies for the framework comparison stand.
//! Every implementation must return these exact bytes.

pub const HOME: &str = include_str!("../../../fixtures/home.html");
pub const ABOUT: &str = include_str!("../../../fixtures/about.html");
pub const BLOG: &str = include_str!("../../../fixtures/blog.html");
pub const POST_HELLO: &str = include_str!("../../../fixtures/post_hello.html");
pub const CONTACT: &str = include_str!("../../../fixtures/contact.html");
pub const HEALTH_JSON: &str = include_str!("../../../fixtures/health.json");

pub const CONTENT_TYPE_HTML: &str = "text/html; charset=utf-8";
pub const CONTENT_TYPE_JSON: &str = "application/json";

/// Routes exercised by verify + load.
pub const PATHS: &[&str] = &["/", "/about", "/blog", "/blog/hello", "/contact", "/api/health"];

pub fn body_for(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/" => Some((CONTENT_TYPE_HTML, HOME)),
        "/about" => Some((CONTENT_TYPE_HTML, ABOUT)),
        "/blog" => Some((CONTENT_TYPE_HTML, BLOG)),
        "/blog/hello" => Some((CONTENT_TYPE_HTML, POST_HELLO)),
        "/contact" => Some((CONTENT_TYPE_HTML, CONTACT)),
        "/api/health" => Some((CONTENT_TYPE_JSON, HEALTH_JSON)),
        _ => None,
    }
}
