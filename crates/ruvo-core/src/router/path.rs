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
        if let Some(name) = seg.strip_prefix(':') {
            out.push('{');
            out.push_str(name);
            out.push('}');
        } else {
            out.push_str(seg);
        }
    }
    Some(out)
}

/// Convert Express `:id` / `*path` to matchit `{id}` / `{*path}`.
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
            } else if let Some(name) = seg.strip_prefix(':') {
                format!("{{{name}}}")
            } else {
                seg.to_string()
            }
        })
        .fold(String::new(), |mut acc, seg| {
            acc.push('/');
            acc.push_str(&seg);
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_path_params() {
        assert_eq!(to_brace_path("/users/:id").as_deref(), Some("/users/{id}"));
        assert_eq!(to_brace_path("/").as_deref(), Some("/"));
    }

    #[test]
    fn brace_path_rejects_wildcard() {
        assert!(to_brace_path("/files/*path").is_none());
    }
}
