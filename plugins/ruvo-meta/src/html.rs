//! Render HTML `<head>` fragment from [`ResolvedMeta`].

use crate::resolve::ResolvedMeta;
use serde_json::Value;

pub fn render_html(meta: &ResolvedMeta) -> String {
    let mut out = String::new();
    if let Some(ref t) = meta.title {
        out.push_str(&format!("<title>{}</title>\n", escape(t)));
    }
    if let Some(ref d) = meta.description {
        out.push_str(&format!(
            "<meta name=\"description\" content=\"{}\">\n",
            escape(d)
        ));
    }
    let robots = if meta.noindex {
        "noindex, nofollow"
    } else {
        "index, follow"
    };
    out.push_str(&format!(
        "<meta name=\"robots\" content=\"{robots}\">\n"
    ));
    if let Some(ref c) = meta.canonical {
        out.push_str(&format!(
            "<link rel=\"canonical\" href=\"{}\">\n",
            escape(c)
        ));
    }
    for (lang, href) in &meta.hreflang {
        out.push_str(&format!(
            "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">\n",
            escape(lang),
            escape(href)
        ));
    }

    // Open Graph
    if let Some(ref t) = meta.title {
        out.push_str(&format!(
            "<meta property=\"og:title\" content=\"{}\">\n",
            escape(t)
        ));
    }
    if let Some(ref d) = meta.description {
        out.push_str(&format!(
            "<meta property=\"og:description\" content=\"{}\">\n",
            escape(d)
        ));
    }
    out.push_str(&format!(
        "<meta property=\"og:type\" content=\"{}\">\n",
        escape(&meta.og_type)
    ));
    if let Some(ref c) = meta.canonical {
        out.push_str(&format!(
            "<meta property=\"og:url\" content=\"{}\">\n",
            escape(c)
        ));
    }
    if let Some(ref img) = meta.image {
        out.push_str(&format!(
            "<meta property=\"og:image\" content=\"{}\">\n",
            escape(img)
        ));
    }
    if let Some(ref sn) = meta.site_name {
        out.push_str(&format!(
            "<meta property=\"og:site_name\" content=\"{}\">\n",
            escape(sn)
        ));
    }
    if let Some(ref loc) = meta.og_locale {
        out.push_str(&format!(
            "<meta property=\"og:locale\" content=\"{}\">\n",
            escape(loc)
        ));
    }
    for alt in &meta.og_locale_alternate {
        out.push_str(&format!(
            "<meta property=\"og:locale:alternate\" content=\"{}\">\n",
            escape(alt)
        ));
    }

    // Twitter
    out.push_str("<meta name=\"twitter:card\" content=\"summary_large_image\">\n");
    if let Some(ref site) = meta.twitter_site {
        out.push_str(&format!(
            "<meta name=\"twitter:site\" content=\"{}\">\n",
            escape(site)
        ));
    }
    if let Some(ref t) = meta.title {
        out.push_str(&format!(
            "<meta name=\"twitter:title\" content=\"{}\">\n",
            escape(t)
        ));
    }
    if let Some(ref d) = meta.description {
        out.push_str(&format!(
            "<meta name=\"twitter:description\" content=\"{}\">\n",
            escape(d)
        ));
    }
    if let Some(ref img) = meta.image {
        out.push_str(&format!(
            "<meta name=\"twitter:image\" content=\"{}\">\n",
            escape(img)
        ));
    }

    for block in &meta.jsonld {
        let with_ctx = ensure_context(block.clone());
        if let Ok(s) = serde_json::to_string(&with_ctx) {
            out.push_str("<script type=\"application/ld+json\">");
            out.push_str(&s);
            out.push_str("</script>\n");
        }
    }

    out
}

fn ensure_context(mut v: Value) -> Value {
    if let Value::Object(ref mut map) = v {
        map.entry("@context".to_string())
            .or_insert_with(|| Value::String("https://schema.org".into()));
    }
    v
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
