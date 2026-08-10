//! Translation store: load, flatten, merge root+scope, pre-serialize, etag.

use bytes::Bytes;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

pub const ROOT_SCOPE: &str = "root";

/// Locale metadata exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct Locale {
    pub code: String,
    pub iso: String,
    pub name: String,
    pub dir: String,
    /// When `false`, omit from hreflang / sitemap alternates.
    pub seo: bool,
    /// Explicit `og:locale` (e.g. `en_US`). When absent, derived from `iso`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub og: Option<String>,
}

impl Locale {
    pub fn new(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            iso: code.clone(),
            name: code.clone(),
            dir: "ltr".into(),
            seo: true,
            og: None,
            code,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_iso(mut self, iso: impl Into<String>) -> Self {
        self.iso = iso.into();
        self
    }

    pub fn with_dir(mut self, dir: impl Into<String>) -> Self {
        self.dir = dir.into();
        self
    }

    pub fn with_seo(mut self, seo: bool) -> Self {
        self.seo = seo;
        self
    }

    pub fn with_og(mut self, og: impl Into<String>) -> Self {
        self.og = Some(og.into());
        self
    }

    /// `og:locale` value: explicit `og` or `iso` with `-` → `_`.
    pub fn og_locale(&self) -> String {
        self.og
            .clone()
            .unwrap_or_else(|| self.iso.replace('-', "_"))
    }
}

/// One locale+scope snapshot.
#[derive(Debug, Clone)]
pub struct Scope {
    pub flat: HashMap<Box<str>, Box<str>>,
    pub payload: Bytes,
    pub etag: Box<str>,
    /// Nested JSON after merge (for prefix/keys queries).
    pub tree: Value,
}

/// Immutable translation catalog.
#[derive(Debug, Clone)]
pub struct Store {
    pub scopes: HashMap<(Box<str>, Box<str>), Arc<Scope>>,
    pub locales: Vec<Locale>,
    /// Hash of all payloads — version token for `?v=`.
    pub version: Box<str>,
}

impl Store {
    pub fn get(&self, locale: &str, scope: &str) -> Option<Arc<Scope>> {
        self.scopes
            .get(&(locale.into(), scope.into()))
            .map(Arc::clone)
    }

    pub fn lookup_flat(&self, locale: &str, scope: &str, key: &str) -> Option<String> {
        self.get(locale, scope)
            .and_then(|s| s.flat.get(key).map(|v| v.to_string()))
    }
}

/// Load `locales/{lang}.json` and `locales/pages/{page}/{lang}.json`.
pub fn load_store(dir: &Path, locales: &[Locale]) -> sova_core::Result<Store> {
    let mut roots: HashMap<String, Value> = HashMap::new();
    let mut pages: HashMap<(String, String), Value> = HashMap::new();

    for loc in locales {
        let root_path = dir.join(format!("{}.json", loc.code));
        let root = read_json_object(&root_path)?;
        roots.insert(loc.code.clone(), root);

        let pages_dir = dir.join("pages");
        if pages_dir.is_dir() {
            for entry in std::fs::read_dir(&pages_dir).map_err(sova_core::Error::from)? {
                let entry = entry.map_err(sova_core::Error::from)?;
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let page = entry.file_name().to_string_lossy().into_owned();
                let page_file = entry.path().join(format!("{}.json", loc.code));
                if page_file.is_file() {
                    let v = read_json_object(&page_file)?;
                    pages.insert((loc.code.clone(), page), v);
                }
            }
        }
    }

    let mut scopes = HashMap::new();
    let mut version_hasher = std::collections::hash_map::DefaultHasher::new();

    for loc in locales {
        let root_tree = roots
            .get(&loc.code)
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        let root_scope = build_scope(&root_tree)?;
        root_scope.payload.hash(&mut version_hasher);
        scopes.insert(
            (loc.code.clone().into_boxed_str(), ROOT_SCOPE.into()),
            Arc::new(root_scope),
        );

        for ((lang, page), page_tree) in &pages {
            if lang != &loc.code {
                continue;
            }
            let merged = deep_merge(&root_tree, page_tree);
            let scope = build_scope(&merged)?;
            scope.payload.hash(&mut version_hasher);
            scopes.insert(
                (lang.clone().into_boxed_str(), page.clone().into_boxed_str()),
                Arc::new(scope),
            );
        }
    }

    let version = format!("{:016x}", version_hasher.finish()).into_boxed_str();
    Ok(Store {
        scopes,
        locales: locales.to_vec(),
        version,
    })
}

fn read_json_object(path: &Path) -> sova_core::Result<Value> {
    if !path.is_file() {
        return Ok(Value::Object(Map::new()));
    }
    let raw = std::fs::read_to_string(path).map_err(sova_core::Error::from)?;
    let v: Value = serde_json::from_str(&raw)
        .map_err(|e| sova_core::Error::Internal(format!("i18n {}: {e}", path.display())))?;
    if !v.is_object() {
        return Err(sova_core::Error::Internal(format!(
            "i18n {}: root must be a JSON object",
            path.display()
        )));
    }
    Ok(v)
}

fn build_scope(tree: &Value) -> sova_core::Result<Scope> {
    let mut flat = HashMap::new();
    flatten(tree, "", &mut flat);
    let payload = Bytes::from(
        serde_json::to_vec(tree)
            .map_err(|e| sova_core::Error::Internal(format!("i18n serialize: {e}")))?,
    );
    let etag = etag_for(&payload);
    Ok(Scope {
        flat,
        payload,
        etag,
        tree: tree.clone(),
    })
}

fn etag_for(payload: &Bytes) -> Box<str> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    payload.hash(&mut h);
    format!("\"{:016x}\"", h.finish()).into_boxed_str()
}

/// Deep-merge objects; `overlay` wins on conflicts. Non-objects: overlay replaces.
pub fn deep_merge(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(a), Value::Object(b)) => {
            let mut out = a.clone();
            for (k, v) in b {
                match out.get(k) {
                    Some(existing) => {
                        out.insert(k.clone(), deep_merge(existing, v));
                    }
                    None => {
                        out.insert(k.clone(), v.clone());
                    }
                }
            }
            Value::Object(out)
        }
        (_, over) => over.clone(),
    }
}

/// Flatten nested JSON strings into dotted keys.
pub fn flatten(value: &Value, prefix: &str, out: &mut HashMap<Box<str>, Box<str>>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                match v {
                    Value::String(s) => {
                        out.insert(key.into_boxed_str(), s.clone().into_boxed_str());
                    }
                    Value::Object(_) => flatten(v, &key, out),
                    other => {
                        out.insert(key.into_boxed_str(), other.to_string().into_boxed_str());
                    }
                }
            }
        }
        Value::String(s) if !prefix.is_empty() => {
            out.insert(prefix.into(), s.clone().into_boxed_str());
        }
        _ => {}
    }
}

/// JS `getByPath`: exact top-level key first (`hasOwnProperty`), else dotted walk.
pub fn get_by_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if let Some(obj) = value.as_object() {
        if let Some(v) = obj.get(path) {
            return Some(v);
        }
    }
    let mut cur = value;
    for part in path.split('.') {
        cur = cur.as_object()?.get(part)?;
    }
    Some(cur)
}

/// Select a subtree or key set as a JSON object for API responses.
pub fn select_tree(tree: &Value, prefix: Option<&str>, keys: Option<&[&str]>) -> Value {
    if let Some(keys) = keys {
        let mut out = Map::new();
        for key in keys {
            if let Some(v) = get_by_path(tree, key) {
                insert_path(&mut out, key, v.clone());
            }
        }
        return Value::Object(out);
    }
    if let Some(prefix) = prefix {
        return match get_by_path(tree, prefix) {
            Some(v) => v.clone(),
            None => Value::Object(Map::new()),
        };
    }
    tree.clone()
}

fn insert_path(out: &mut Map<String, Value>, path: &str, value: Value) {
    if !path.contains('.') {
        out.insert(path.to_string(), value);
        return;
    }
    let mut parts = path.split('.').peekable();
    let first = parts.next().unwrap();
    let entry = out
        .entry(first.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let mut cur = entry;
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            if let Value::Object(map) = cur {
                map.insert(part.to_string(), value);
            }
            return;
        }
        if !cur.is_object() {
            *cur = Value::Object(Map::new());
        }
        let map = cur.as_object_mut().unwrap();
        cur = map
            .entry(part.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (rel, body) in files {
            let path = dir.path().join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, body).unwrap();
        }
        dir
    }

    #[test]
    fn flatten_nested_and_dotted_top_level() {
        let v: Value = serde_json::json!({
            "nav": { "about": "About" },
            "a.b": "literal"
        });
        let mut flat = HashMap::new();
        flatten(&v, "", &mut flat);
        assert_eq!(flat.get("nav.about").map(|s| s.as_ref()), Some("About"));
        assert_eq!(flat.get("a.b").map(|s| s.as_ref()), Some("literal"));
    }

    #[test]
    fn get_by_path_prefers_exact_key() {
        let v: Value = serde_json::json!({
            "a.b": "exact",
            "a": { "b": "nested" }
        });
        assert_eq!(
            get_by_path(&v, "a.b").and_then(|x| x.as_str()),
            Some("exact")
        );
    }

    #[test]
    fn page_merges_root_and_overrides() {
        let locales = vec![Locale::new("en")];
        let dir = write_temp(&[
            ("en.json", r#"{"shared":"root","nav":{"home":"Home"}}"#),
            ("pages/blog/en.json", r#"{"shared":"blog","title":"Post"}"#),
        ]);
        let store = load_store(dir.path(), &locales).unwrap();

        let root = store.get("en", "root").unwrap();
        assert_eq!(root.flat.get("shared").map(|s| s.as_ref()), Some("root"));
        assert!(!root.flat.contains_key("title"));

        let blog = store.get("en", "blog").unwrap();
        assert_eq!(blog.flat.get("shared").map(|s| s.as_ref()), Some("blog"));
        assert_eq!(blog.flat.get("nav.home").map(|s| s.as_ref()), Some("Home"));
        assert_eq!(blog.flat.get("title").map(|s| s.as_ref()), Some("Post"));
    }

    #[test]
    fn etag_stable_and_changes() {
        let locales = vec![Locale::new("en")];
        let dir = write_temp(&[("en.json", r#"{"a":"1"}"#)]);
        let s1 = load_store(dir.path(), &locales).unwrap();
        let s2 = load_store(dir.path(), &locales).unwrap();
        assert_eq!(
            s1.get("en", "root").unwrap().etag,
            s2.get("en", "root").unwrap().etag
        );

        std::fs::write(dir.path().join("en.json"), r#"{"a":"2"}"#).unwrap();
        let s3 = load_store(dir.path(), &locales).unwrap();
        assert_ne!(
            s1.get("en", "root").unwrap().etag,
            s3.get("en", "root").unwrap().etag
        );
    }
}
