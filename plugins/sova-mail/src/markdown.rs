//! Markdown → HTML for mail bodies (`pulldown-cmark`).

use pulldown_cmark::{html, Options, Parser};

pub fn to_html(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_heading_and_link() {
        let html = to_html("# Hi\n\n[Go](https://example.com)");
        assert!(html.contains("<h1>Hi</h1>"), "{html}");
        assert!(html.contains("href=\"https://example.com\""), "{html}");
    }
}
