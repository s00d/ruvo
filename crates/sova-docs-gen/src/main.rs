//! Generate **static** VitePress markdown from Sova Rust sources.
//!
//! ## Plugins (`docs/plugins/<slug>.md`)
//! 1. Summary + crate / docs.rs / plugin id
//! 2. Install (`cargo add`) + features table
//! 3. Overview — `docs/.vitepress/plugin-guides/<slug>.md` if present, else crate `//!`
//! 4. Quick start — `docs/.vitepress/plugin-usage/<slug>.md`
//! 5. Examples + related plugins (hardcoded maps)
//!
//! ## Plugin SDK (`docs/api/plugin-sdk.md` + `docs/api/plugin-sdk/<slug>.md`)
//! Guides in `docs/.vitepress/plugin-sdk-guides/` + ordered list in `plugin_sdk_pages()`.
//! Also: catalog table, grouped sidebars.
//!
//! ```bash
//! cargo run -p sova-docs-gen
//! cargo run -p sova-docs-gen -- --check
//! ```

use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::{visit::Visit, Expr, ImplItem, Item, Lit, Meta};
use walkdir::WalkDir;

/// GitHub tree base for in-repo example paths.
const GH_TREE: &str = "https://github.com/s00d/sova/tree/master";

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

        let install_feat = preferred_install_feature(&slug, &facade_feats);
        let mut page = format!(
            "---\ntitle: {slug}\neditLink: false\n---\n\n# `{slug}`\n\n\
**{summary}**\n\n\
| | |\n|--|--|\n\
| Crate | [`{crate_name}`](https://docs.rs/{crate_name}/{version}) `{version}` |\n\
| Plugin id | `{plugin_id}` |\n\
| Category | {category} |\n",
            category = plugin_category(&slug),
        );

        if let Some(feat) = &install_feat {
            page.push_str(&format!(
                "\n## Install\n\n```bash\ncargo add sova --features {feat}\n```\n"
            ));
        } else if crate_name == "sovax" {
            page.push_str(
                "\n## Install\n\n```bash\ncargo install cargo-sovax\n# or: cargo run -p sovax -- <cmd>\n```\n",
            );
        }

        if !facade_feats.is_empty() {
            let mut rows = String::from(
                "\n## Features\n\n| Feature | What you get |\n|---------|-------------|\n",
            );
            for f in &facade_feats {
                let desc = feature_docs.get(f).map(String::as_str).unwrap_or("—");
                rows.push_str(&format!("| `{f}` | {desc} |\n"));
            }
            page.push_str(&rows);
        }

        let guide_path = w
            .docs
            .join(".vitepress/plugin-guides")
            .join(format!("{slug}.md"));
        let guide = fs::read_to_string(&guide_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let overview = guide.unwrap_or_else(|| crate_docs.clone());
        if !overview.is_empty() {
            page.push_str("\n## Overview\n\n");
            page.push_str(&linkify_example_paths(&overview));
            page.push('\n');
        }

        let usage_path = w
            .docs
            .join(".vitepress/plugin-usage")
            .join(format!("{slug}.md"));
        if let Ok(usage) = fs::read_to_string(&usage_path) {
            let usage = usage.trim();
            if !usage.is_empty() {
                page.push_str("\n## Quick start\n\n");
                page.push_str(&linkify_example_paths(usage));
                page.push('\n');
            }
        }

        if let Some(ex) = plugin_examples(&slug) {
            page.push_str("\n## Examples\n\n");
            for line in ex {
                page.push_str(&format!("- {}\n", example_md_link(line)));
            }
        }

        if let Some(rel) = plugin_related(&slug) {
            page.push_str("\n## Related\n\n");
            page.push_str(
                &rel.iter()
                    .map(|s| format!("[`{s}`](/plugins/{s})"))
                    .collect::<Vec<_>>()
                    .join(" · "),
            );
            page.push('\n');
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
        let cat = plugin_category(&slug);
        plugin_index_rows.push(format!(
            "| [`{slug}`](/plugins/{slug}) | {cat} | `{version}` | {summary_cell} | {feats_cell} |"
        ));
        plugin_slugs.push(slug);
    }

    let plugins_table = format!(
        "| Plugin | Category | Version | Summary | Features |\n|--------|----------|---------|---------|----------|\n{}",
        plugin_index_rows.join("\n")
    );
    w.patch_marker("plugins/index.md", "plugins-table", &plugins_table)?;

    let sidebar = build_grouped_sidebar(&plugin_slugs);
    let nav = build_grouped_nav(&plugin_slugs);
    let nav_ts = format!(
        "// Generated by sova-docs-gen — do not edit.\n\
export const pluginsNav = [\n  {{ text: 'Catalog', link: '/plugins/' }},\n\
{nav}\n\
] as const\n\n\
export const pluginsSidebar = [\n  {{ text: 'Catalog', link: '/plugins/' }},\n\
{sidebar}\n\
]\n"
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

    generate_plugin_sdk(root, &mut w)?;
    linkify_handwritten_docs(&mut w)?;

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

/// Ordered Plugin SDK pages. Body from `docs/.vitepress/plugin-sdk-guides/<slug>.md`.
fn plugin_sdk_pages() -> &'static [SdkPage] {
    &[
        SdkPage {
            slug: "overview",
            title: "Overview",
            summary: "Mental model, import surfaces, and install checklist for plugin authors.",
            group: "Start",
        },
        SdkPage {
            slug: "plugin-trait",
            title: "Plugin trait",
            summary: "id, meta, requires, install, closure plugins, and SDK versioning.",
            group: "Start",
        },
        SdkPage {
            slug: "middleware",
            title: "Middleware",
            summary: "named, with_leaked, with_state, MwEntry — when to use each.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "state",
            title: "State & dependencies",
            summary: "app.state, markers, Needs, hard requires vs soft has_plugin / try_state.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "config",
            title: "Config",
            summary: "Toml unset-fill, env precedence, parse_duration / parse_bytes, features.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "lifecycle",
            title: "Lifecycle & services",
            summary: "on_startup / on_shutdown, pool pattern, BackgroundService, CLI mode.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "checks-cli",
            title: "Checks & CLI",
            summary: "register_check vs register_audit, probes, register_cli commands.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "routes",
            title: "Routes & introspection",
            summary: "Plugin routes, path helpers, RouteValue / MetaMap, match captures.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "extractors",
            title: "Extractors & Problem+",
            summary: "Path/Json/State handlers, EventBus, API problem+json errors.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "events",
            title: "Events",
            summary: "EventBus listen/dispatch + catalog of first-party plugin events.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "html-hooks",
            title: "HTML & log hooks",
            summary: "HTML inject, logger_skip_path, add_log_event_hook for DevTools-style sinks.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "errors",
            title: "Errors",
            summary: "Startup Err vs panic, ErrorResponse, soft degradation.",
            group: "Cookbook",
        },
        SdkPage {
            slug: "recipes",
            title: "Recipes",
            summary: "Patterns copied from in-tree plugins (cookies→csrf, pools, tasks, store).",
            group: "Patterns",
        },
        SdkPage {
            slug: "extend-api",
            title: "extend API",
            summary: "Symbol table for sova_core::extend — what it is and who uses it.",
            group: "Reference",
        },
        SdkPage {
            slug: "testing",
            title: "Testing",
            summary: "In-process TestClient, ResponseAssert, TestApp/sqlite, cookies, auth hooks, real examples.",
            group: "Reference",
        },
    ]
}

struct SdkPage {
    slug: &'static str,
    title: &'static str,
    summary: &'static str,
    group: &'static str,
}

fn generate_plugin_sdk(root: &Path, w: &mut Writer) -> Result<(), String> {
    let pages = plugin_sdk_pages();
    let guides = w.docs.join(".vitepress/plugin-sdk-guides");
    let mut slugs: Vec<&str> = Vec::new();

    for page in pages {
        let guide_path = guides.join(format!("{}.md", page.slug));
        let body = fs::read_to_string(&guide_path)
            .map_err(|e| format!("missing plugin-sdk guide {}: {e}", guide_path.display()))?;
        let body = body.trim();
        if body.is_empty() {
            return Err(format!("empty plugin-sdk guide: {}", page.slug));
        }

        let mut md = format!(
            "---\ntitle: {title}\neditLink: false\n---\n\n# {title}\n\n\
{summary}\n\n\
> Author guide — edit `docs/.vitepress/plugin-sdk-guides/{slug}.md`, then `pnpm docs:generate`.\n\n\
{body}\n",
            title = page.title,
            summary = page.summary,
            slug = page.slug,
            body = body,
        );

        // Append rustdoc extract on the trait page.
        if page.slug == "plugin-trait" {
            let plugin_rs = fs::read_to_string(root.join("crates/sova-core/src/plugin.rs"))
                .map_err(|e| e.to_string())?;
            let rustdoc = extract_crate_docs(&plugin_rs)?;
            if !rustdoc.trim().is_empty() {
                md.push_str("\n## From `sova-core` rustdoc\n\n");
                md.push_str(rustdoc.trim());
                md.push('\n');
            }
        }

        w.emit(&format!("api/plugin-sdk/{}.md", page.slug), &md)?;
        slugs.push(page.slug);
    }

    // Index lives at /api/plugin-sdk (same URL as before the multi-page split).
    let mut toc = String::from("| Page | Summary |\n|------|---------|\n");
    for page in pages {
        toc.push_str(&format!(
            "| [`{}`](/api/plugin-sdk/{}) | {} |\n",
            page.title, page.slug, page.summary
        ));
    }
    let index = format!(
        "---\ntitle: Plugin SDK\neditLink: false\n---\n\n# Plugin SDK\n\n\
![Plugin SDK](/banners/plugin-sdk.svg)\n\n\
Write `sova-*` plugins against `sova_core::extend` and the [`Plugin`](/api/plugin-sdk/plugin-trait) trait.\n\
App users: use the [Plugins](/plugins/) catalog instead.\n\n\
How pages are built: guides in [`plugin-sdk-guides`](https://github.com/s00d/sova/tree/master/docs/.vitepress/plugin-sdk-guides) → `sova-docs-gen` → `docs/api/plugin-sdk*`.\n\n\
## Pages\n\n\
<!-- generated:plugin-sdk-toc -->\n\
{toc}\
<!-- /generated:plugin-sdk-toc -->\n\n\
## Quick links\n\n\
- Start: [Overview](/api/plugin-sdk/overview) · [Plugin trait](/api/plugin-sdk/plugin-trait)\n\
- Cookbook: [Middleware](/api/plugin-sdk/middleware) · [State](/api/plugin-sdk/state) · [Recipes](/api/plugin-sdk/recipes)\n\
- Reference: [extend API](/api/plugin-sdk/extend-api) · [Testing](/api/plugin-sdk/testing)\n\
- Scaffold: `cargo sovax generate plugin <name>`\n"
    );
    w.emit("api/plugin-sdk.md", &index)?;

    // Nav / sidebar TS
    let mut by_group: BTreeMap<&str, Vec<&SdkPage>> = BTreeMap::new();
    for page in pages {
        by_group.entry(page.group).or_default().push(page);
    }
    let group_order = ["Start", "Cookbook", "Patterns", "Reference"];
    let mut sidebar_blocks = Vec::new();
    let mut nav_items = Vec::new();
    for group in group_order {
        let Some(items) = by_group.get(group) else {
            continue;
        };
        let lines: Vec<String> = items
            .iter()
            .map(|p| {
                format!(
                    "    {{ text: '{}', link: '/api/plugin-sdk/{}' }}",
                    p.title, p.slug
                )
            })
            .collect();
        sidebar_blocks.push(format!(
            "  {{\n    text: '{group}',\n    collapsed: false,\n    items: [\n{}\n    ],\n  }}",
            lines.join(",\n")
        ));
        for p in items {
            nav_items.push(format!(
                "  {{ text: '{}', link: '/api/plugin-sdk/{}' }}",
                p.title, p.slug
            ));
        }
    }
    let nav_ts = format!(
        "// Generated by sova-docs-gen — do not edit.\n\
export const pluginSdkNav = [\n  {{ text: 'Index', link: '/api/plugin-sdk' }},\n\
{}\n\
] as const\n\n\
export const pluginSdkSidebar = [\n  {{ text: 'Plugin SDK', link: '/api/plugin-sdk' }},\n\
{}\n\
]\n",
        nav_items.join(",\n"),
        sidebar_blocks.join(",\n")
    );
    w.emit(".vitepress/plugin-sdk-nav.generated.ts", &nav_ts)?;

    if !w.check {
        prune_sdk_pages(&w.docs.join("api/plugin-sdk"), &slugs)?;
        // Drop the short-lived /plugin-sdk/ tree if present.
        let legacy = w.docs.join("plugin-sdk");
        if legacy.is_dir() {
            let _ = fs::remove_dir_all(&legacy);
        }
    }
    Ok(())
}

fn prune_sdk_pages(dir: &Path, keep: &[&str]) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    for ent in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let ent = ent.map_err(|e| e.to_string())?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if name == "index.md" {
            continue;
        }
        if let Some(stem) = name.strip_suffix(".md") {
            if !keep.iter().any(|s| *s == stem) {
                let _ = fs::remove_file(ent.path());
            }
        }
    }
    Ok(())
}

fn preferred_install_feature(slug: &str, feats: &[String]) -> Option<String> {
    let prefer: &[&str] = match slug {
        "http" => &["http-client"],
        "static" => &["static-files"],
        "sse" => &["sse-feed"],
        "quic" => &["quic-udp"],
        "ai" => &["ai-openai", "ai"],
        "graphql" => &["graphql", "graphql-server"],
        "grpc" => &["grpc"],
        "rabbit" => &["rabbit"],
        "auth" => &["auth"],
        "passport" => &["passport"],
        "db" => &["db"],
        "mail" => &["mail"],
        "session" => &["session"],
        "store" => &["store"],
        "storage" => &["storage"],
        "fs" => &["fs"],
        "tasks" => &["tasks"],
        "tasks-store" => &["tasks-store"],
        "notifications" => &["notifications"],
        "observability" => &["observability"],
        "i18n" => &["i18n"],
        "meta" => &["meta"],
        "vld" => &["vld"],
        "templates" => &["templates"],
        "idempotency" => &["idempotency"],
        "response-cache" => &["response-cache"],
        "rate-limit" => &["rate-limit"],
        _ => &[],
    };
    for p in prefer {
        if feats.iter().any(|f| f == p) {
            return Some((*p).to_string());
        }
    }
    feats.first().cloned()
}

fn plugin_category(slug: &str) -> &'static str {
    match slug {
        "shield" | "cors" | "csrf" | "compress" | "cookies" | "rate-limit" | "idempotency"
        | "response-cache" | "static" | "env" | "acme" => "HTTP",
        "auth" | "passport" | "session" | "vld" => "Auth",
        "db" | "redis" | "store" | "storage" | "fs" | "tasks" | "tasks-store" => "Data",
        "templates" | "mail" | "i18n" | "meta" | "openapi" => "Content",
        "ws" | "sse" | "udp" | "quic" | "notifications" => "Realtime",
        "observability" | "activity" | "devtools" => "Ops",
        "http" | "ai" | "graphql" | "grpc" | "rabbit" => "Integrations",
        "cli" => "Tooling",
        _ => "Other",
    }
}

fn plugin_category_order() -> &'static [(&'static str, &'static str)] {
    &[
        ("HTTP", "HTTP & middleware"),
        ("Auth", "Auth & validation"),
        ("Data", "Data & jobs"),
        ("Content", "Content & mail"),
        ("Realtime", "Realtime"),
        ("Ops", "Observability"),
        ("Integrations", "Integrations"),
        ("Tooling", "Tooling"),
        ("Other", "Other"),
    ]
}

fn build_grouped_sidebar(slugs: &[String]) -> String {
    build_grouped_blocks(slugs, 2, true)
}

fn build_grouped_nav(slugs: &[String]) -> String {
    // Top-nav: nested category menus (no `collapsed` — that's sidebar-only).
    build_grouped_blocks(slugs, 2, false)
}

fn build_grouped_blocks(slugs: &[String], indent: usize, collapsed: bool) -> String {
    let pad = " ".repeat(indent);
    let mut by_cat: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for s in slugs {
        by_cat
            .entry(plugin_category(s))
            .or_default()
            .push(s.as_str());
    }
    let mut blocks = Vec::new();
    for (cat, title) in plugin_category_order() {
        let Some(items) = by_cat.get(cat) else {
            continue;
        };
        if items.is_empty() {
            continue;
        }
        let mut sorted = items.clone();
        sorted.sort();
        let lines: Vec<String> = sorted
            .iter()
            .map(|s| format!("{pad}  {{ text: '{s}', link: '/plugins/{s}' }}"))
            .collect();
        let head = if collapsed {
            format!("{pad}{{\n{pad}  text: '{title}',\n{pad}  collapsed: false,\n{pad}  items: [")
        } else {
            format!("{pad}{{\n{pad}  text: '{title}',\n{pad}  items: [")
        };
        blocks.push(format!("{head}\n{}\n{pad}  ],\n{pad}}}", lines.join(",\n")));
    }
    blocks.join(",\n")
}

fn example_md_link(path: &str) -> String {
    format!("[`{path}`]({GH_TREE}/{path})")
}

fn is_repo_example_path(s: &str) -> bool {
    let s = s.trim_end_matches('/');
    if s != "examples" && !s.starts_with("examples/") {
        return false;
    }
    !s.is_empty()
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.'))
}

/// Turn `` `examples/...` `` into GitHub markdown links (idempotent for already-linked paths).
fn linkify_example_paths(md: &str) -> String {
    let mut out = String::with_capacity(md.len() + 64);
    let mut rest = md;
    while let Some(start) = rest.find('`') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        if !out.ends_with('[') && is_repo_example_path(inner) {
            out.push_str(&example_md_link(inner));
        } else {
            out.push('`');
            out.push_str(inner);
            out.push('`');
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn linkify_handwritten_docs(w: &mut Writer) -> Result<(), String> {
    const RELS: &[&str] = &[
        "examples.md",
        "guide/getting-started.md",
        "guide/concepts.md",
        "guide/configuration.md",
        "guide/devtools.md",
        "plugins/index.md",
    ];
    for rel in RELS {
        let path = w.docs.join(rel);
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("{rel}: {e}"))?;
        let linked = linkify_example_paths(&raw);
        if linked != raw {
            w.emit(rel, &linked)?;
        }
    }

    // Keep VitePress author sources linkified too (GitHub browse).
    for sub in [".vitepress/plugin-usage", ".vitepress/plugin-guides"] {
        let dir = w.docs.join(sub);
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&dir).map_err(|e| format!("{sub}: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let name = entry.file_name();
            let rel = format!("{sub}/{}", name.to_string_lossy());
            let raw = fs::read_to_string(&path).map_err(|e| format!("{rel}: {e}"))?;
            let linked = linkify_example_paths(&raw);
            if linked != raw {
                w.emit(&rel, &linked)?;
            }
        }
    }
    Ok(())
}

fn plugin_examples(slug: &str) -> Option<&'static [&'static str]> {
    Some(match slug {
        "ai" => &["examples/api/api_ai"],
        "graphql" => &[
            "examples/api/api_graphql",
            "examples/api/api_graphql_server",
        ],
        "grpc" => &["examples/api/api_grpc"],
        "rabbit" => &["examples/api/api_rabbit"],
        "auth" => &[
            "examples/cabinet",
            "examples/web/hackernews",
            "examples/api/api_auth",
            "examples/basic/auth",
        ],
        "passport" => &["examples/api/api_jwt", "examples/api/api_oauth"],
        "db" => &["examples/api/crud", "examples/cabinet"],
        "tasks" => &["examples/misc/tasks"],
        "redis" => &["examples/misc/redis"],
        "storage" => &["examples/misc/storage", "examples/web/upload"],
        "fs" => &["examples/misc/fs_demo"],
        "store" => &["examples/misc/redb"],
        "sse" => &["examples/realtime/sse", "examples/realtime/sse_feed"],
        "ws" => &["examples/realtime/ws_chat"],
        "quic" => &["examples/net/quic_udp_echo"],
        "udp" => &["examples/net/udp_echo"],
        "acme" => &["examples/net/acme_hello"],
        "i18n" => &["examples/web/i18n", "examples/web/templates_i18n"],
        "meta" => &["examples/web/meta_blog"],
        "static" => &["examples/web/static_files"],
        "templates" => &["examples/web/templates"],
        "vld" => &["examples/api/api_validated"],
        "openapi" => &["examples/api/api_preset"],
        "cli" => &["examples/basic/cli"],
        "http" => &["examples/cabinet"],
        "mail" => &["examples/cabinet"],
        "session" => &["examples/cabinet", "examples/web/hackernews"],
        "notifications" => &["examples/cabinet"],
        "observability" => &["examples/misc/bench_loaded"],
        _ => return None,
    })
}

fn plugin_related(slug: &str) -> Option<&'static [&'static str]> {
    Some(match slug {
        "auth" => &["passport", "session", "db", "mail", "activity"],
        "passport" => &["auth", "session", "db"],
        "session" => &["cookies", "csrf", "store", "redis", "auth"],
        "csrf" => &["session", "cookies"],
        "cookies" => &["session", "csrf"],
        "mail" => &["auth", "notifications", "templates"],
        "notifications" => &["db", "ws", "mail", "auth"],
        "db" => &["auth", "tasks", "store", "notifications"],
        "redis" => &["store", "session", "tasks-store"],
        "store" => &[
            "session",
            "redis",
            "rate-limit",
            "csrf",
            "idempotency",
            "response-cache",
        ],
        "tasks" => &["tasks-store", "db", "redis"],
        "tasks-store" => &["tasks", "redis", "db"],
        "storage" => &["static", "fs"],
        "fs" => &["storage", "static"],
        "static" => &["storage", "templates", "fs"],
        "templates" => &["mail", "meta", "i18n"],
        "meta" => &["templates", "i18n", "openapi"],
        "i18n" => &["templates", "vld", "meta"],
        "vld" => &["openapi", "i18n", "auth"],
        "openapi" => &["vld", "meta"],
        "ws" => &["sse", "notifications"],
        "sse" => &["ws"],
        "udp" => &["quic"],
        "quic" => &["udp", "acme"],
        "acme" => &["quic"],
        "http" => &["ai", "graphql", "grpc"],
        "ai" => &["http", "sse"],
        "graphql" => &["http", "grpc"],
        "grpc" => &["http", "graphql"],
        "rabbit" => &["redis", "tasks"],
        "observability" => &["activity"],
        "activity" => &["auth", "observability"],
        "shield" => &["cors", "csrf"],
        "cors" => &["shield"],
        "compress" => &["static"],
        "rate-limit" => &["store", "redis"],
        "idempotency" => &["store"],
        "response-cache" => &["store", "idempotency"],
        "env" => &["cli"],
        "cli" => &["env", "db", "tasks"],
        _ => return None,
    })
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
                    lit: Lit::Str(s), ..
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
                    lit: Lit::Str(s), ..
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
                    lit: Lit::Str(s), ..
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
