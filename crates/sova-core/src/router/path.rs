pub fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return "/".into();
    }
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

pub(super) fn normalize_prefix(prefix: &str) -> String {
    if prefix.is_empty() || prefix == "/" {
        return String::new();
    }
    let mut p = normalize_path(prefix);
    while p.ends_with('/') && p.len() > 1 {
        p.pop();
    }
    p
}

pub fn join_paths(prefix: &str, path: &str) -> String {
    let path = if path.is_empty() { "/" } else { path };
    if prefix.is_empty() {
        return normalize_path(path);
    }
    if path == "/" {
        return prefix.to_string();
    }
    let path = path.strip_prefix('/').unwrap_or(path);
    format!("{prefix}/{path}")
}

/// Convert Express `:id` to OpenAPI `{id}`.
///
/// Returns `None` when the path contains a wildcard (`*…`) — those are not
/// expressible as OpenAPI path items.
pub fn to_brace_path(path: &str) -> Option<String> {
    let path = normalize_path(path);
    if path == "/" {
        return Some("/".into());
    }
    let mut out = String::new();
    for seg in path.trim_matches('/').split('/') {
        if seg.starts_with('*') {
            return None;
        }
        out.push('/');
        out.push_str(&colon_params_to_braces(seg));
    }
    Some(out)
}

/// Convert Express `:id` / `*path` to matchit `{id}` / `{*path}`.
///
/// Also rewrites inline params (`/sitemap-:n.xml` → `/sitemap-{n}.xml`).
pub(crate) fn to_matchit_path(path: &str) -> String {
    let path = normalize_path(path);
    if path == "/" {
        return "/".into();
    }
    path.trim_matches('/')
        .split('/')
        .map(|seg| {
            if let Some(name) = seg.strip_prefix('*') {
                if name.is_empty() {
                    "{*path}".to_string()
                } else {
                    format!("{{*{name}}}")
                }
            } else {
                colon_params_to_braces(seg)
            }
        })
        .fold(String::new(), |mut acc, seg| {
            acc.push('/');
            acc.push_str(&seg);
            acc
        })
}

/// `:id` → `{id}`; `sitemap-:n.xml` → `sitemap-{n}.xml`.
fn colon_params_to_braces(seg: &str) -> String {
    let bytes = seg.as_bytes();
    let mut out = String::with_capacity(seg.len() + 2);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            if end > start {
                out.push('{');
                out.push_str(&seg[start..end]);
                out.push('}');
                i = end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_path_params() {
        assert_eq!(to_brace_path("/users/:id").as_deref(), Some("/users/{id}"));
        assert_eq!(to_brace_path("/").as_deref(), Some("/"));
        assert_eq!(
            to_brace_path("/sitemap-:n.xml").as_deref(),
            Some("/sitemap-{n}.xml")
        );
    }

    #[test]
    fn matchit_inline_param() {
        assert_eq!(to_matchit_path("/sitemap-:n.xml"), "/sitemap-{n}.xml");
        assert_eq!(to_matchit_path("/users/:id"), "/users/{id}");
    }

    #[test]
    fn brace_path_rejects_wildcard() {
        assert!(to_brace_path("/files/*path").is_none());
    }
}
