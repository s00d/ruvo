//! MiniJinjaEngine add_template / render_html.

use sova_templates::MiniJinjaEngine;
use serde::Serialize;

#[derive(Serialize)]
struct Ctx {
    name: String,
}

#[test]
fn render_html_ok() {
    let mut eng = MiniJinjaEngine::new();
    eng.add_template("hi", "Hello {{ name }}!")
        .expect("add_template");
    let res = eng
        .render_html(
            "hi",
            Ctx {
                name: "ada".into(),
            },
        )
        .expect("render");
    assert_eq!(res.body_bytes(), Some(b"Hello ada!".as_slice()));
}

#[test]
fn missing_template_is_err() {
    let eng = MiniJinjaEngine::new();
    let err = match eng.render_html("missing", Ctx { name: "x".into() }) {
        Err(e) => e,
        Ok(_) => panic!("expected missing template error"),
    };
    assert!(err.to_string().contains("template"), "{err}");
}
