//! Generate **static** VitePress markdown from Sova Rust sources.
//!
//! Plugins catalog table + one page per plugin under `docs/plugins/<slug>.md`.
//! Optional hand-written usage: `docs/.vitepress/plugin-usage/<slug>.md` (appended as ## Usage).
//! Plugin SDK page for authors. Nav/sidebar: `.vitepress/plugins-nav.generated.ts`.
//!
//! ```bash
//! cargo run -p sova-docs-gen
//! cargo run -p sova-docs-gen -- --check
//! ```

use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Expr, ImplItem, Item, Lit, Meta, visit::Visit};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(about = "Generate VitePress markdown from Sova sources")]
struct Args {
    #[arg(long)]
    check: bool,
    #[arg(long)]
    root: Option<PathBuf>,
}

#[derive(Clone)]
struct Feature {
    name: String,
    deps: Vec<String>,
}

struct Writer {
    docs: PathBuf,
    check: bool,
    stale: bool,
}

impl Writer {
    fn emit(&mut self, rel: &str, content: &str) -> Result<(), String> {
        let abs = self.docs.join(rel);
        let next = if content.ends_with('\n') {
            content.to_string()
        } else {
            format!("{content}\n")
        };
        if self.check {
            match fs::read_to_string(&abs) {
                Ok(prev) if prev == next => Ok(()),
                _ => {
                    eprintln!("[check] stale: {rel}");
                    self.stale = true;
                    Ok(())
                }
            }
        } else {
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&abs, next).map_err(|e| e.to_string())
        }
    }

    fn patch_marker(&mut self, rel: &str, id: &str, body: &str) -> Result<(), String> {
        let abs = self.docs.join(rel);
        let Ok(md) = fs::read_to_string(&abs) else {
            eprintln!("missing page for marker {id}: {rel}");
            return Ok(());
        };
        let start = format!("<!-- generated:{id} -->");
        let end = format!("<!-- /generated:{id} -->");
        let Some(i0) = md.find(&start) else {
            eprintln!("missing marker {id} in {rel}");
            return Ok(());
        };
        let Some(i1_rel) = md[i0..].find(&end) else {
            eprintln!("missing end marker {id} in {rel}");
            return Ok(());
        };
        let i1 = i0 + i1_rel;
        let block = format!("{start}\n{}\n{end}", body.trim());
        let next = format!("{}{}{}", &md[..i0], block, &md[i1 + end.len()..]);
        if self.check {
            if next != md {
                eprintln!("[check] stale marker {id} in {rel}");
                self.stale = true;
            }
            Ok(())
        } else if next != md {
            fs::write(&abs, next).map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}

fn main() {
    let args = Args::parse();
    let root = args
        .root
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let root = root
        .canonicalize()
        .unwrap_or_else(|e| panic!("repo root: {e}"));
    let docs = root.join("docs");
    match run(&root, &docs, args.check) {
        Ok(false) => {}
        Ok(true) => {
            eprintln!("docs generate --check failed");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("sova-docs-gen: {e}");
            std::process::exit(1);
        }
    }
}

fn run(root: &Path, docs: &Path, check: bool) -> Result<bool, String> {
    let mut w = Writer {
        docs: docs.to_path_buf(),
        check,
        stale: false,
    };

    let features = parse_features_ordered(
        &fs::read_to_string(root.join("crates/sova/Cargo.toml")).map_err(|e| e.to_string())?,
    )?;
    let feature_docs = parse_feature_docs(&root.join("crates/sova/src/doc_features.rs"))?;
    let crate_to_features = feature_plugin_map(&features);
    let related = related_features_by_crate(&features);

    let plugins_dir = root.join("plugins");
    let mut plugin_slugs: Vec<String> = Vec::new();
    let mut plugin_index_rows: Vec<String> = Vec::new();
    let mut nav_entries: Vec<String> = Vec::new();
    nav_entries.push(r#"  { text: 'Catalog', link: '/plugins/' }"#.to_string());

    let mut entries: Vec<_> = fs::read_dir(&plugins_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_dir()).unwrap_or(false) && {
                let n = e.file_name().to_string_lossy().into_owned();
                n.starts_with("sova-") || n == "sovax"
            }
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for ent in entries {
        let crate_name = ent.file_name().to_string_lossy().into_owned();
        let crate_dir = ent.path();
        let slug = if crate_name == "sovax" {
            "cli".to_string()
        } else {
            crate_name.trim_start_matches("sova-").to_string()
        };
        let lib = crate_dir.join("src/lib.rs");
        let crate_docs = if lib.is_file() {
            extract_crate_docs(&fs::read_to_string(&lib).map_err(|e| e.to_string())?)?
        } else {
            String::new()
        };

        let mut meta_desc = String::new();
        let mut plugin_ids: Vec<String> = Vec::new();
        for path in rust_sources(&crate_dir) {
            let src = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let Ok(file) = syn::parse_file(&src) else {
                continue;
            };
            let mut v = PluginVisitor::default();
            v.visit_file(&file);
            if meta_desc.is_empty() {
                if let Some(d) = v.description {
                    meta_desc = d;
                }
            }
            if let Some(id) = v.plugin_id {
                if !plugin_ids.iter().any(|x| x == &id) {
                    plugin_ids.push(id);
                }
            }
        }
        // Prefer id matching the docs slug (e.g. `meta` over `sitemap` in sova-meta).
        let plugin_id = plugin_ids
            .iter()
            .find(|id| *id == &slug)
            .cloned()
            .or_else(|| plugin_ids.first().cloned())
            .unwrap_or_else(|| slug.clone());

        let cargo_toml = fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap_or_default();
        let pkg_desc = cargo_description(&cargo_toml).unwrap_or_default();
        let version = cargo_version(&cargo_toml).unwrap_or_else(|| "?".into());
        let facade_feats = {
            let mut set = BTreeMap::new();
            for f in crate_to_features.get(&crate_name).into_iter().flatten() {
                set.insert(f.clone(), ());
            }
            for f in related.get(&crate_name).into_iter().flatten() {
                set.insert(f.clone(), ());
            }
            set.into_keys().collect::<Vec<_>>()
        };
        let summary = if !meta_desc.is_empty() {
            meta_desc
        } else if !pkg_desc.is_empty() {
            pkg_desc
        } else {
            crate_docs.lines().next().unwrap_or("").to_string()
        };

        let feats_section = if facade_feats.is_empty() {
            String::new()
        } else {
            let mut rows = String::from("| Feature | What you get |\n|---------|-------------|\n");
            for f in &facade_feats {
                let desc = feature_docs.get(f).map(String::as_str).unwrap_or("—");
                rows.push_str(&format!("| `{f}` | {desc} |\n"));
            }
            format!(
                "\n```bash\ncargo add sova --features {}\n```\n\n{rows}",
                facade_feats.join(",")
            )
        };

        let mut page = format!(
            "---\ntitle: {slug}\neditLink: false\n---\n\n# `{slug}`\n\n\
**{summary}** · crate `{crate_name}` `{version}` · id `{plugin_id}`\n"
        );
        page.push_str(&feats_section);
        if !crate_docs.is_empty() {
            page.push('\n');
            page.push_str(&crate_docs);
            page.push('\n');
        }
        let usage_path = w.docs.join(".vitepress/plugin-usage").join(format!("{slug}.md"));
        if let Ok(usage) = fs::read_to_string(&usage_path) {
            let usage = usage.trim();
            if !usage.is_empty() {
                page.push_str("\n## Usage\n\n");
                page.push_str(usage);
                page.push('\n');
            }
        }
        w.emit(&format!("plugins/{slug}.md"), &page)?;

        let feats_cell = if facade_feats.is_empty() {
            "—".to_string()
        } else {
            facade_feats
                .iter()
                .map(|f| format!("`{f}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let summary_cell = summary.replace('|', "\\|");
        plugin_index_rows.push(format!(
            "| [`{slug}`](/plugins/{slug}) | `{version}` | {summary_cell} | {feats_cell} |"
        ));
        nav_entries.push(format!(
            "  {{ text: '{slug}', link: '/plugins/{slug}' }}"
        ));
        plugin_slugs.push(slug);
    }

    let plugins_table = format!(
        "| Plugin | Version | Summary | Features |\n|--------|---------|---------|----------|\n{}",
        plugin_index_rows.join("\n")
    );
    w.patch_marker("plugins/index.md", "plugins-table", &plugins_table)?;

    let nav_ts = format!(
        "// Generated by sova-docs-gen — do not edit.\n\
export const pluginsNav = [\n{}\n] as const\n\n\
export const pluginsSidebar = [\n\
  {{\n\
    text: 'Plugins',\n\
    collapsed: false,\n\
    items: [\n{}\n\
    ],\n\
  }},\n\
]\n",
        nav_entries.join(",\n"),
        nav_entries.join(",\n"),
    );
    w.emit(".vitepress/plugins-nav.generated.ts", &nav_ts)?;

    // Remove legacy JSON if present
    let legacy = w.docs.join(".vitepress/plugins-sidebar.generated.json");
    if legacy.is_file() && !check {
        let _ = fs::remove_file(&legacy);
    }

    if !check {
        prune_plugin_pages(&w.docs.join("plugins"), &plugin_slugs)?;
    }

    let plugin_sdk = extract_crate_docs(
        &fs::read_to_string(root.join("crates/sova-core/src/plugin.rs"))
            .map_err(|e| e.to_string())?,
    )?;
    w.emit(
        "api/plugin-sdk.md",
        &format!(
            "---\ntitle: Plugin SDK\neditLink: false\n---\n\n# Plugin SDK\n\n\
![Plugin SDK](/banners/plugin-sdk.svg)\n\n\
> Auto-generated from `crates/sova-core/src/plugin.rs`. For writing plugins — app usage is under [Plugins](/plugins/).\n\n\
{plugin_sdk}\n"
        ),
    )?;

    if check {
        if w.stale {
            Ok(true)
        } else {
            eprintln!("docs generate --check ok");
            Ok(false)
        }
    } else {
        eprintln!(
            "generated plugins catalog ({} pages), sidebar, plugin-sdk",
            plugin_slugs.len()
        );
        Ok(false)
    }
}

fn prune_plugin_pages(plugins_docs: &Path, keep: &[String]) -> Result<(), String> {
    let Ok(rd) = fs::read_dir(plugins_docs) else {
        return Ok(());
    };
    for ent in rd.filter_map(|e| e.ok()) {
        let path = ent.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == "index.md" || !name.ends_with(".md") {
            continue;
        }
        let slug = name.trim_end_matches(".md");
        if !keep.iter().any(|s| s == slug) {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            eprintln!("removed stale plugin page: {name}");
        }
    }
    Ok(())
}

fn parse_features_ordered(toml_text: &str) -> Result<Vec<Feature>, String> {
    let Some((_, rest)) = toml_text.split_once("[features]") else {
        return Ok(Vec::new());
    };
    let body = rest.split("\n[").next().unwrap_or(rest);
    let mut out = Vec::new();
    let mut lines = body.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, rhs0)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().to_string();
        let mut rhs = rhs0.trim().to_string();
        if rhs.starts_with('[') && !rhs.ends_with(']') {
            for more in lines.by_ref() {
                rhs.push(' ');
                rhs.push_str(more.trim());
                if more.contains(']') {
                    break;
                }
            }
        }
        out.push(Feature {
            name,
            deps: parse_deps_list(&rhs),
        });
    }
    Ok(out)
}

fn parse_deps_list(rhs: &str) -> Vec<String> {
    let inner = rhs
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn feature_plugin_map(features: &[Feature]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in features {
        for d in &f.deps {
            if let Some(crate_name) = d.strip_prefix("dep:") {
                if crate_name.starts_with("sova-") {
                    map.entry(crate_name.to_string())
                        .or_default()
                        .push(f.name.clone());
                }
            }
        }
    }
    map
}

fn related_features_by_crate(features: &[Feature]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in features {
        for d in &f.deps {
            let crate_name = if let Some(c) = d.strip_prefix("dep:") {
                c.to_string()
            } else if d.starts_with("sova-") {
                d.split('/').next().unwrap_or(d).to_string()
            } else {
                continue;
            };
            if !crate_name.starts_with("sova-") {
                continue;
            }
            let list = map.entry(crate_name).or_default();
            if !list.contains(&f.name) {
                list.push(f.name.clone());
            }
        }
    }
    map
}

fn parse_feature_docs(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("/// Feature `") else {
            continue;
        };
        let Some((name, desc)) = rest.split_once("`: ") else {
            continue;
        };
        map.insert(name.to_string(), desc.trim().to_string());
    }
    Ok(map)
}

fn cargo_description(toml_text: &str) -> Option<String> {
    cargo_package_str(toml_text, "description")
}

fn cargo_version(toml_text: &str) -> Option<String> {
    cargo_package_str(toml_text, "version")
}

fn cargo_package_str(toml_text: &str, key: &str) -> Option<String> {
    for line in toml_text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim().trim_start_matches('=').trim();
            if let Some(s) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
                return Some(s.to_string());
            }
        }
        if line.starts_with('[') && line != "[package]" {
            break;
        }
    }
    None
}

fn rust_sources(crate_dir: &Path) -> Vec<PathBuf> {
    let src = crate_dir.join("src");
    if !src.is_dir() {
        return Vec::new();
    }
    let mut paths: Vec<PathBuf> = WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        .map(|e| e.into_path())
        .collect();
    paths.sort();
    paths
}

fn extract_crate_docs(src: &str) -> Result<String, String> {
    let file = syn::parse_file(src).map_err(|e| format!("syn: {e}"))?;
    let mut lines = Vec::new();
    for attr in &file.attrs {
        if let Meta::NameValue(nv) = &attr.meta {
            if nv.path.is_ident("doc") {
                if let Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    lines.push(s.value());
                }
            }
        }
    }
    Ok(sanitize_markdown_for_vitepress(lines.join("\n").trim()))
}

fn sanitize_markdown_for_vitepress(md: &str) -> String {
    sanitize_code_fences(&sanitize_rustdoc_links(md))
}

fn sanitize_code_fences(md: &str) -> String {
    md.lines()
        .map(|line| {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("```") {
                let lang = rest.trim();
                if lang == "ignore"
                    || lang == "rust,ignore"
                    || lang == "ignore,rust"
                    || lang.starts_with("rust,ignore")
                    || lang == "no_run"
                    || lang == "rust,no_run"
                    || lang == "should_panic"
                    || lang == "rust,should_panic"
                    || lang == "compile_fail"
                    || lang == "rust,compile_fail"
                {
                    return "```rust".to_string();
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_rustdoc_links(md: &str) -> String {
    let mut result = String::with_capacity(md.len());
    let mut rest = md;
    while !rest.is_empty() {
        let Some(bracket) = rest.find('[') else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..bracket]);
        let at = &rest[bracket..];
        if let Some((consumed, repl)) = try_rustdoc_link(at) {
            result.push_str(&repl);
            rest = &at[consumed..];
        } else {
            result.push('[');
            rest = &at[1..];
        }
    }
    result
}

fn try_rustdoc_link(s: &str) -> Option<(usize, String)> {
    let rest = s.strip_prefix('[')?;
    let (label, after_label) = if let Some(r) = rest.strip_prefix('`') {
        let mid = r.find('`')?;
        let name = &r[..mid];
        let after = &r[mid + 1..];
        if !after.starts_with("](") {
            return None;
        }
        (name, &after[2..])
    } else {
        let mid = rest.find(']')?;
        let name = &rest[..mid];
        let after = &rest[mid + 1..];
        if !after.starts_with('(') {
            return None;
        }
        (name, &after[1..])
    };
    let end_paren = after_label.find(')')?;
    let target = &after_label[..end_paren];
    if !target.contains("::") {
        return None;
    }
    let match_len = s.len() - after_label[end_paren + 1..].len();
    Some((match_len, format!("`{label}`")))
}

#[derive(Default)]
struct PluginVisitor {
    description: Option<String>,
    plugin_id: Option<String>,
}

impl<'ast> Visit<'ast> for PluginVisitor {
    fn visit_item(&mut self, item: &'ast Item) {
        if let Item::Impl(im) = item {
            let is_plugin = im
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last().map(|s| s.ident == "Plugin"))
                .unwrap_or(false);
            if is_plugin {
                for memb in &im.items {
                    if let ImplItem::Fn(f) = memb {
                        if f.sig.ident == "id" {
                            if let Some(s) = first_str_return(&f.block.stmts) {
                                self.plugin_id = Some(s);
                            }
                        }
                        if f.sig.ident == "meta" {
                            find_description_call(&f.block, &mut self.description);
                        }
                    }
                }
            }
        }
        syn::visit::visit_item(self, item);
    }
}

fn first_str_return(stmts: &[syn::Stmt]) -> Option<String> {
    for st in stmts {
        match st {
            syn::Stmt::Expr(
                Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s),
                    ..
                }),
                _,
            ) => return Some(s.value()),
            syn::Stmt::Expr(Expr::Block(b), _) => {
                if let Some(s) = first_str_return(&b.block.stmts) {
                    return Some(s);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_description_call(block: &syn::Block, out: &mut Option<String>) {
    struct V<'a>(&'a mut Option<String>);
    impl<'ast> Visit<'ast> for V<'_> {
        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            if node.method == "description" {
                if let Some(Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s),
                    ..
                })) = node.args.first()
                {
                    if self.0.is_none() {
                        *self.0 = Some(s.value());
                    }
                }
            }
            syn::visit::visit_expr_method_call(self, node);
        }
    }
    V(out).visit_block(block);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_intra_doc() {
        let s = "UDP as [`BackgroundService`](sova_core::BackgroundService).";
        assert_eq!(sanitize_rustdoc_links(s), "UDP as `BackgroundService`.");
    }

    #[test]
    fn maps_rustdoc_fences_to_rust() {
        let s = "```ignore\nfn x() {}\n```\n```rust,ignore\nfn y() {}\n```";
        let out = sanitize_code_fences(s);
        assert!(out.contains("```rust\nfn x()"));
        assert!(out.contains("```rust\nfn y()"));
        assert!(!out.contains("ignore"));
    }
}
