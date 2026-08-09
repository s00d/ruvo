//! Inject DevTools markup before `</body>` (via `sova_core::html`).

use sova_core::html::{inject, HtmlAnchor, HtmlInject};

pub const DEVTOOLS_MARKER: &str = "<!-- sova_devtools -->";

/// Insert toolbar + assets. Returns `None` when already present / empty.
pub fn inject_body(html: &str, fragment: &str) -> Option<String> {
    inject(
        html,
        &HtmlInject::new(HtmlAnchor::BeforeCloseBody, fragment)
            .marker(DEVTOOLS_MARKER)
            .skip_if_contains(&["id=\"sova-devtools\""]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_before_body_close() {
        let html = "<html><body><p>hi</p></body></html>";
        let out = inject_body(html, "<div id=\"sova-devtools\"></div>").unwrap();
        assert!(out.contains(DEVTOOLS_MARKER));
        assert!(out.find("sova-devtools").unwrap() < out.find("</body>").unwrap());
    }

    #[test]
    fn idempotent() {
        let html = format!("<body>{DEVTOOLS_MARKER}</body>");
        assert!(inject_body(&html, "<div></div>").is_none());
    }
}
