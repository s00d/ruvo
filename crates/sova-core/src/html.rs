//! HTML string helpers for plugins that mutate `text/html` responses.
//!
//! # Layering
//!
//! 1. **String ops** — [`inject`], [`inject_before`], [`replace_once`],
//!    [`replace_between`] (markers / anchors, no HTTP).
//! 2. **Response** — [`crate::Response::map_buffered_html`] skips streams / non-HTML.
//! 3. **Middleware** — [`crate::middleware::before`] / [`after`] / [`around`] /
//!    [`map_html`] for request/response hooks around the whole chain.
//!
//! Prefer markers (`<!-- plugin -->`) so stacked injects stay idempotent.
//! Template-slot style edits: put `<!--slot:name-->…<!--/slot:name-->` in the
//! document and use [`replace_between`].

/// Where to place a fragment inside an HTML document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlAnchor {
    /// Before `</head>` (creates `<head>` after `<html>` if missing).
    BeforeCloseHead,
    /// Right after `<body…>`.
    AfterOpenBody,
    /// Before `</body>` (appends if missing).
    BeforeCloseBody,
}

/// Options for [`inject`].
#[derive(Debug, Clone)]
pub struct HtmlInject<'a> {
    pub fragment: &'a str,
    pub anchor: HtmlAnchor,
    /// If this substring is already present, return `None` (idempotent).
    pub marker: Option<&'a str>,
    /// Extra skip needles (e.g. `id="sova-devtools"`).
    pub skip_if_contains: &'a [&'a str],
}

impl<'a> HtmlInject<'a> {
    pub fn new(anchor: HtmlAnchor, fragment: &'a str) -> Self {
        Self {
            fragment,
            anchor,
            marker: None,
            skip_if_contains: &[],
        }
    }

    pub fn marker(mut self, marker: &'a str) -> Self {
        self.marker = Some(marker);
        self
    }

    pub fn skip_if_contains(mut self, needles: &'a [&'a str]) -> Self {
        self.skip_if_contains = needles;
        self
    }
}

/// Case-insensitive substring search; returns byte index in `hay` (ASCII tags).
pub fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

/// Insert `fragment` immediately before the first case-insensitive `needle`.
pub fn inject_before(html: &str, needle: &str, fragment: &str) -> Option<String> {
    if fragment.is_empty() {
        return None;
    }
    let idx = find_ci(html, needle)?;
    let mut out = String::with_capacity(html.len() + fragment.len());
    out.push_str(&html[..idx]);
    out.push_str(fragment);
    out.push_str(&html[idx..]);
    Some(out)
}

/// Insert `fragment` immediately after the first case-insensitive open tag `needle`
/// (e.g. `"<body"` → after the closing `>` of that tag).
pub fn inject_after_open_tag(html: &str, open_tag: &str, fragment: &str) -> Option<String> {
    if fragment.is_empty() {
        return None;
    }
    let start = find_ci(html, open_tag)?;
    let gt = html[start..].find('>')? + start;
    let idx = gt + 1;
    let mut out = String::with_capacity(html.len() + fragment.len());
    out.push_str(&html[..idx]);
    out.push_str(fragment);
    out.push_str(&html[idx..]);
    Some(out)
}

/// Replace the first occurrence of `from` with `to` (case-sensitive).
pub fn replace_once(html: &str, from: &str, to: &str) -> Option<String> {
    let idx = html.find(from)?;
    let mut out = String::with_capacity(html.len() - from.len() + to.len());
    out.push_str(&html[..idx]);
    out.push_str(to);
    out.push_str(&html[idx + from.len()..]);
    Some(out)
}

/// Replace content between `start_marker` and `end_marker` (exclusive markers kept).
pub fn replace_between(
    html: &str,
    start_marker: &str,
    end_marker: &str,
    replacement: &str,
) -> Option<String> {
    let start = html.find(start_marker)? + start_marker.len();
    let end_rel = html[start..].find(end_marker)?;
    let end = start + end_rel;
    let mut out = String::with_capacity(html.len() - (end - start) + replacement.len());
    out.push_str(&html[..start]);
    out.push_str(replacement);
    out.push_str(&html[end..]);
    Some(out)
}

/// Apply [`HtmlInject`]. Returns `None` when skipped or unchanged.
pub fn inject(html: &str, opts: &HtmlInject<'_>) -> Option<String> {
    let frag = opts.fragment;
    if frag.trim().is_empty() {
        return None;
    }
    if let Some(m) = opts.marker {
        if html.contains(m) {
            return None;
        }
    }
    for n in opts.skip_if_contains {
        if html.contains(n) {
            return None;
        }
    }

    let mut block = String::new();
    if let Some(m) = opts.marker {
        block.push_str(m);
        block.push('\n');
    }
    block.push_str(frag);

    match opts.anchor {
        HtmlAnchor::BeforeCloseHead => inject_before_close_head(html, &block),
        HtmlAnchor::AfterOpenBody => inject_after_open_tag(html, "<body", &block)
            .or_else(|| Some(format!("{html}\n{block}\n"))),
        HtmlAnchor::BeforeCloseBody => {
            inject_before(html, "</body>", &block).or_else(|| Some(format!("{html}\n{block}\n")))
        }
    }
}

fn inject_before_close_head(html: &str, block: &str) -> Option<String> {
    if let Some(out) = inject_before(html, "</head>", block) {
        return Some(out);
    }
    // After <html…>
    if let Some(start) = find_ci(html, "<html") {
        if let Some(gt) = html[start..].find('>') {
            let idx = start + gt + 1;
            let mut out = String::with_capacity(html.len() + block.len() + 16);
            out.push_str(&html[..idx]);
            out.push_str("\n<head>\n");
            out.push_str(block);
            out.push_str("</head>\n");
            out.push_str(&html[idx..]);
            return Some(out);
        }
    }
    // Bare fragment
    Some(format!(
        "<!doctype html>\n<html>\n<head>\n{block}</head>\n<body>\n{html}\n</body>\n</html>\n"
    ))
}

/// Convenience: head inject with marker.
pub fn inject_head(html: &str, fragment: &str, marker: &str) -> Option<String> {
    inject(
        html,
        &HtmlInject::new(HtmlAnchor::BeforeCloseHead, fragment).marker(marker),
    )
}

/// Convenience: before `</body>` with marker.
pub fn inject_body_end(html: &str, fragment: &str, marker: &str) -> Option<String> {
    inject(
        html,
        &HtmlInject::new(HtmlAnchor::BeforeCloseBody, fragment).marker(marker),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_and_body() {
        let html = "<html><head></head><body><p>x</p></body></html>";
        let h = inject_head(html, "<title>T</title>", "<!-- m -->").unwrap();
        assert!(h.contains("<title>T</title>"));
        assert!(h.contains("<!-- m -->"));
        let b = inject_body_end(&h, "<div id=\"x\"></div>", "<!-- b -->").unwrap();
        assert!(b.find("id=\"x\"").unwrap() < b.find("</body>").unwrap());
    }

    #[test]
    fn replace_between_works() {
        let html = "a<!--s-->OLD<!--e-->z";
        let out = replace_between(html, "<!--s-->", "<!--e-->", "NEW").unwrap();
        assert_eq!(out, "a<!--s-->NEW<!--e-->z");
    }

    #[test]
    fn idempotent_marker() {
        let html = "<html><head><!-- m --></head><body></body></html>";
        assert!(inject_head(html, "<title>T</title>", "<!-- m -->").is_none());
    }
}
