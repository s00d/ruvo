//! Inject SEO head fragment into HTML responses.

/// Comment marker — skip if already present (idempotent).
pub const META_MARKER: &str = "<!-- sova_meta -->";

/// Insert `fragment` into `body`. Returns `None` when no change is needed.
pub fn inject_head(body: &str, fragment: &str) -> Option<String> {
    if fragment.trim().is_empty() {
        return None;
    }
    if body.contains(META_MARKER) || body.contains("data-sova_meta") {
        return None;
    }

    let has_title = contains_tag(body, "title");
    let mut frag = String::new();
    frag.push_str(META_MARKER);
    frag.push('\n');
    if has_title {
        frag.push_str(&strip_title_tags(fragment));
    } else {
        frag.push_str(fragment);
    }
    if frag.trim() == META_MARKER {
        return None;
    }

    if let Some(idx) = find_ci(body, "</head>") {
        let mut out = String::with_capacity(body.len() + frag.len());
        out.push_str(&body[..idx]);
        out.push_str(&frag);
        out.push_str(&body[idx..]);
        return Some(out);
    }

    if let Some(after) = after_html_open(body) {
        let mut out = String::with_capacity(body.len() + frag.len() + 16);
        out.push_str(&body[..after]);
        out.push_str("<head>\n");
        out.push_str(&frag);
        out.push_str("</head>\n");
        out.push_str(&body[after..]);
        return Some(out);
    }

    // Bare fragment / no document shell.
    Some(format!(
        "<!doctype html>\n<html>\n<head>\n{frag}</head>\n<body>\n{body}\n</body>\n</html>\n"
    ))
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
            let skip = start + end_rel + "</title>".len();
            // also skip trailing newline if present
            let mut skip = skip;
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

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

fn after_html_open(body: &str) -> Option<usize> {
    let lower = body.to_ascii_lowercase();
    let start = lower.find("<html")?;
    let after_lt = &body[start..];
    let gt = after_lt.find('>')?;
    Some(start + gt + 1)
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
        let out = inject_head(body, "<title>Other</title>\n<meta name=\"description\" content=\"d\">\n")
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
