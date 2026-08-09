//! Inject SEO head fragment into HTML responses (via `sova_core::html`).

use sova_core::html::{inject, HtmlAnchor, HtmlInject};

/// Comment marker — skip if already present (idempotent).
pub const META_MARKER: &str = "<!-- sova_meta -->";

/// Insert `fragment` into `body`. Returns `None` when no change is needed.
pub fn inject_head(body: &str, fragment: &str) -> Option<String> {
    if fragment.trim().is_empty() {
        return None;
    }

    let has_title = contains_tag(body, "title");
    let frag = if has_title {
        strip_title_tags(fragment)
    } else {
        fragment.to_string()
    };
    if frag.trim().is_empty() {
        return None;
    }

    inject(
        body,
        &HtmlInject::new(HtmlAnchor::BeforeCloseHead, &frag)
            .marker(META_MARKER)
            .skip_if_contains(&["data-sova_meta"]),
    )
}

fn contains_tag(hay: &str, tag: &str) -> bool {
    let lower = hay.to_ascii_lowercase();
    lower.contains(&format!("<{tag}")) || lower.contains(&format!("<{tag} "))
}

fn strip_title_tags(fragment: &str) -> String {
    let mut out = String::new();
    let lower = fragment.to_ascii_lowercase();
    let mut rest = fragment;
    let mut rest_l = lower.as_str();
    while let Some(start) = rest_l.find("<title") {
        out.push_str(&rest[..start]);
        let after_open = &rest_l[start..];
        if let Some(end_rel) = after_open.find("</title>") {
            let mut skip = start + end_rel + "</title>".len();
            if rest[skip..].starts_with('\n') {
                skip += 1;
            }
            rest = &rest[skip..];
            rest_l = &rest_l[skip..];
        } else {
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_bare_body() {
        let out = inject_head("<h1>Hi</h1>", "<title>T</title>\n").unwrap();
        assert!(out.contains("<title>T</title>"));
        assert!(out.contains("<h1>Hi</h1>"));
        assert!(out.contains(META_MARKER));
    }

    #[test]
    fn inserts_before_close_head() {
        let body = "<html><head><meta charset=\"utf-8\"></head><body>x</body></html>";
        let out = inject_head(body, "<title>T</title>\n").unwrap();
        assert!(out.contains("<title>T</title>"));
        let pos_title = out.find("<title>T</title>").unwrap();
        let pos_close = out.to_ascii_lowercase().find("</head>").unwrap();
        assert!(pos_title < pos_close);
    }

    #[test]
    fn skips_duplicate_title() {
        let body = "<html><head><title>Mine</title></head><body></body></html>";
        let out = inject_head(
            body,
            "<title>Other</title>\n<meta name=\"description\" content=\"d\">\n",
        )
        .unwrap();
        assert!(out.contains("<title>Mine</title>"));
        assert!(!out.contains("<title>Other</title>"));
        assert!(out.contains("description"));
    }

    #[test]
    fn idempotent_marker() {
        let body = format!("<html><head>{META_MARKER}</head><body></body></html>");
        assert!(inject_head(&body, "<title>T</title>").is_none());
    }
}
