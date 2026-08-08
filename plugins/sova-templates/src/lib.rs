//! Template engines for Sova (MiniJinja).

use minijinja::{path_loader, AutoEscape, Environment, Value};
use minijinja_autoreload::AutoReloader;
use sova_core::{App, Error, Plugin, Request, Response, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type PerRequestProvider = Arc<dyn Fn(&Request) -> Value + Send + Sync>;

/// Shared ambient per-request helpers (plugins register into this at install).
#[derive(Default)]
pub struct TemplateHelpers {
    map: Mutex<HashMap<String, PerRequestProvider>>,
}

impl TemplateHelpers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F>(&self, name: impl Into<String>, provider: F)
    where
        F: Fn(&Request) -> Value + Send + Sync + 'static,
    {
        self.map
            .lock()
            .unwrap()
            .insert(name.into(), Arc::new(provider));
    }

    fn snapshot(&self) -> Vec<(String, PerRequestProvider)> {
        self.map
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), Arc::clone(v)))
            .collect()
    }
}

/// Get-or-create [`TemplateHelpers`] on the app and register a per-request provider.
pub fn register_per_request<F>(app: &mut App, name: impl Into<String>, provider: F)
where
    F: Fn(&Request) -> Value + Send + Sync + 'static,
{
    let helpers = ensure_helpers(app);
    helpers.register(name, provider);
}

fn ensure_helpers(app: &mut App) -> Arc<TemplateHelpers> {
    if let Some(h) = app.try_state::<TemplateHelpers>() {
        return h;
    }
    let helpers = TemplateHelpers::new();
    app.state(helpers);
    app.try_state::<TemplateHelpers>()
        .expect("TemplateHelpers just inserted")
}

/// Pluggable template engine.
///
/// New code should prefer [`Templates`] + [`RenderExt`], which adds:
/// - autoescape/XSS safety
/// - loader from a directory
/// - ambient (global + per-request) context providers
pub trait TemplateEngine: Send + Sync {
    fn render(&self, name: &str, ctx: Value) -> Result<String>;
}

/// Simple in-memory MiniJinja engine (manual `add_template`).
#[derive(Clone)]
pub struct MiniJinjaEngine {
    env: Environment<'static>,
}

impl MiniJinjaEngine {
    pub fn new() -> Self {
        let mut env = Environment::new();
        // Security default: always escape HTML-sensitive characters.
        env.set_auto_escape_callback(|_| AutoEscape::Html);
        if cfg!(debug_assertions) {
            env.set_debug(true);
        }
        Self { env }
    }

    pub fn add_template(&mut self, name: &str, source: &str) -> Result<()> {
        self.env
            .add_template_owned(name.to_string(), source.to_string())
            .map_err(|e| Error::Internal(format!("template: {e}")))?;
        Ok(())
    }

    pub fn render_html<S: Serialize>(&self, name: &str, ctx: S) -> Result<Response> {
        let value = Value::from_serialize(&ctx);
        let html = TemplateEngine::render(self, name, value)?;
        Ok(Response::html(html))
    }
}

impl Default for MiniJinjaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine for MiniJinjaEngine {
    fn render(&self, name: &str, ctx: Value) -> Result<String> {
        let tmpl = self
            .env
            .get_template(name)
            .map_err(|e| missing_template_err(name, &e))?;

        tmpl.render(ctx)
            .map_err(|e| render_failed_err(name, &e))
    }
}

fn format_render_err(name: &str, e: &minijinja::Error) -> String {
    let loc = match (e.name(), e.line()) {
        (Some(n), Some(line)) => format!(" (in {n}:{line})"),
        (_, Some(line)) => format!(" (line {line})"),
        (Some(n), None) => format!(" (in {n})"),
        (None, None) => String::new(),
    };
    if cfg!(debug_assertions) {
        // MiniJinja `#` debug rendering includes source snippet around the line.
        format!("render {name}{loc}:\n{e:#}")
    } else {
        format!("render {name} failed{loc}: {e}")
    }
}

fn missing_template_err(name: &str, e: &minijinja::Error) -> Error {
    let detail = format!("template {name}: {e}");
    if cfg!(debug_assertions) {
        Error::Internal(detail)
    } else {
        tracing::error!(template = %name, error = %e, "template not found");
        Error::Internal(format!("template {name} not found"))
    }
}

fn render_failed_err(name: &str, e: &minijinja::Error) -> Error {
    let detail = format_render_err(name, e);
    if cfg!(debug_assertions) {
        Error::Internal(detail)
    } else {
        tracing::error!(
            template = %name,
            line = ?e.line(),
            error = %e,
            "template render failed"
        );
        Error::Internal("template render failed".into())
    }
}

/// Pre-evaluated ambient template context (globals ± per-request helpers).
///
/// Freeze once at compose time (e.g. `req.mail()`), then render later without a
/// [`Request`] — used by mail deferred `.view()` and other non-HTTP consumers.
#[derive(Clone, Default)]
pub struct FrozenAmbient {
    map: HashMap<String, Value>,
}

impl FrozenAmbient {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// MiniJinja template set stored in Sova app state.
#[derive(Clone)]
pub struct MiniJinjaTemplates {
    engine: MiniJinjaTemplatesEngine,
    globals: Arc<HashMap<String, Value>>,
    per_request: Arc<HashMap<String, PerRequestProvider>>,
    helpers: Arc<TemplateHelpers>,
}

#[derive(Clone)]
enum MiniJinjaTemplatesEngine {
    Static(Arc<Environment<'static>>),
    Reload(Arc<AutoReloader>),
}

impl MiniJinjaTemplates {
    /// Globals + per-request helpers evaluated against `req` (owned, no borrow).
    pub fn freeze_ambient(&self, req: &Request) -> FrozenAmbient {
        let mut map = self.merge_base();
        for (k, provider) in self.helpers.snapshot() {
            map.insert(k, provider(req));
        }
        for (k, provider) in self.per_request.iter() {
            map.insert(k.clone(), (provider)(req));
        }
        FrozenAmbient { map }
    }

    /// Globals only — for background jobs / mail compose without a [`Request`].
    pub fn freeze_globals(&self) -> FrozenAmbient {
        FrozenAmbient {
            map: self.merge_base(),
        }
    }

    /// Render `name` merging a previously frozen ambient map with local `ctx`.
    pub fn render_owned<T: Serialize>(
        &self,
        ambient: &FrozenAmbient,
        name: &str,
        ctx: T,
    ) -> Result<String> {
        self.render_merged(name, ambient.map.clone(), ctx)
    }

    /// Render a template to a string (same ambient merge as [`Self::render_html`]).
    ///
    /// Use for email bodies, fragments, or any non-HTTP consumer. Layouts work via
    /// Jinja `{% extends %}` in the template file.
    pub fn render_str<T: Serialize>(&self, req: &Request, name: &str, ctx: T) -> Result<String> {
        self.render_owned(&self.freeze_ambient(req), name, ctx)
    }

    /// Render with globals only (no per-request helpers) — for background jobs without a [`Request`].
    pub fn render_str_globals<T: Serialize>(&self, name: &str, ctx: T) -> Result<String> {
        self.render_owned(&self.freeze_globals(), name, ctx)
    }

    pub fn render_html<T: Serialize>(&self, req: &Request, name: &str, ctx: T) -> Result<Response> {
        Ok(Response::html(self.render_str(req, name, ctx)?))
    }

    fn merge_base(&self) -> HashMap<String, Value> {
        (*self.globals).clone()
    }

    fn render_merged<T: Serialize>(
        &self,
        name: &str,
        mut merged: HashMap<String, Value>,
        ctx: T,
    ) -> Result<String> {
        let json = serde_json::to_value(ctx).map_err(Error::from)?;
        let obj = json.as_object().ok_or_else(|| {
            Error::BadRequest("templates.render ctx must serialize to an object".into())
        })?;
        for (k, v) in obj {
            merged.insert(k.clone(), Value::from_serialize(v));
        }
        let ctx_val = Value::from(merged);

        match &self.engine {
            MiniJinjaTemplatesEngine::Static(env) => {
                let tmpl = env
                    .get_template(name)
                    .map_err(|e| missing_template_err(name, &e))?;
                tmpl.render(ctx_val)
                    .map_err(|e| render_failed_err(name, &e))
            }
            MiniJinjaTemplatesEngine::Reload(reloader) => {
                let env = reloader
                    .acquire_env()
                    .map_err(|e| Error::Internal(format!("autoreload env: {e}")))?;
                let tmpl = env
                    .get_template(name)
                    .map_err(|e| missing_template_err(name, &e))?;
                tmpl.render(ctx_val)
                    .map_err(|e| render_failed_err(name, &e))
            }
        }
    }

    /// Schedule a filesystem reload on the next render (no-op for static engine).
    ///
    /// Useful in tests; in production the FS watcher usually triggers this.
    pub fn request_reload(&self) {
        if let MiniJinjaTemplatesEngine::Reload(reloader) = &self.engine {
            reloader.notifier().request_reload();
        }
    }
}

/// Plugin entrypoint for template sets.
pub struct Templates;

impl Templates {
    /// Configure MiniJinja templates from a directory.
    ///
    /// Template names are **relative to this directory**, including extension,
    /// e.g. `home.html`, `blog/post.html`.
    pub fn minijinja(dir: impl AsRef<Path>) -> MiniJinjaTemplatesBuilder {
        MiniJinjaTemplatesBuilder::new(dir.as_ref().to_path_buf())
    }
}

/// Builder that installs MiniJinja templates into app state.
pub struct MiniJinjaTemplatesBuilder {
    dir: PathBuf,
    autoreload: bool,
    autoreload_explicit: bool,
    globals: HashMap<String, Value>,
    per_request: HashMap<String, PerRequestProvider>,
}

impl MiniJinjaTemplatesBuilder {
    fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            autoreload: cfg!(debug_assertions),
            autoreload_explicit: false,
            globals: HashMap::new(),
            per_request: HashMap::new(),
        }
    }

    /// Add a variable available in all templates and requests.
    pub fn global<V: Serialize + Send + Sync + 'static>(
        mut self,
        name: impl Into<String>,
        value: V,
    ) -> Self {
        self.globals
            .insert(name.into(), Value::from_serialize(&value));
        self
    }

    /// Provide a per-request variable (called on every render).
    ///
    /// Typical usage: provide `t` translation function for current request.
    pub fn per_request<F>(mut self, name: impl Into<String>, provider: F) -> Self
    where
        F: Fn(&Request) -> Value + Send + Sync + 'static,
    {
        self.per_request.insert(name.into(), Arc::new(provider));
        self
    }

    /// Enable/disable filesystem autoreload (dev convenience).
    pub fn autoreload(mut self, enabled: bool) -> Self {
        self.autoreload = enabled;
        self.autoreload_explicit = true;
        self
    }
}

impl Plugin for MiniJinjaTemplatesBuilder {
    fn id(&self) -> &'static str {
        "templates"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Templates")
            .description("MiniJinja HTML templates with optional autoreload")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut sova_core::App) {
        if !self.autoreload_explicit {
            if let Some(doc) = app.config_doc() {
                if let Some(section) = doc.section("templates") {
                    if let Some(v) = section.get("autoreload").and_then(|v| v.as_bool()) {
                        self.autoreload = v;
                    }
                }
            }
        }
        let helpers = ensure_helpers(app);
        let globals = Arc::new(self.globals);
        let per_request = Arc::new(self.per_request);
        let dir = self.dir;
        let check_dir = dir.clone();
        let engine = if self.autoreload {
            let dir_str = dir
                .to_str()
                .unwrap_or_else(|| panic!("template dir path must be valid UTF-8: {dir:?}"))
                .to_string();
            let dir_load = dir.clone();
            let reloader = AutoReloader::new(move |notifier| {
                notifier.watch_path(dir_str.as_str(), true);
                let mut env = Environment::new();
                env.set_auto_escape_callback(|_| AutoEscape::Html);
                env.set_debug(cfg!(debug_assertions));
                env.set_loader(path_loader(&dir_load));
                Ok(env)
            });
            MiniJinjaTemplatesEngine::Reload(Arc::new(reloader))
        } else {
            let mut env = Environment::new();
            env.set_auto_escape_callback(|_| AutoEscape::Html);
            env.set_debug(cfg!(debug_assertions));
            env.set_loader(path_loader(&dir));
            MiniJinjaTemplatesEngine::Static(Arc::new(env))
        };

        app.state(MiniJinjaTemplates {
            engine,
            globals,
            per_request,
            helpers,
        });

        app.register_audit("templates", move |_state| {
            let dir = check_dir.clone();
            async move { compile_templates_in_dir(&dir) }
        });
    }
}

fn compile_templates_in_dir(dir: &Path) -> Result<()> {
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.set_loader(path_loader(dir));
    for entry in walkdir_templates(dir)? {
        let rel = entry
            .strip_prefix(dir)
            .unwrap_or(entry.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        env.get_template(&rel)
            .map_err(|e| Error::Internal(format!("template `{rel}`: {e}")))?;
    }
    Ok(())
}

fn walkdir_templates(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        let rd = std::fs::read_dir(dir).map_err(|e| Error::Internal(format!("templates: {e}")))?;
        for ent in rd {
            let ent = ent.map_err(|e| Error::Internal(format!("templates: {e}")))?;
            let path = ent.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "html" | "j2" | "jinja" | "jinja2" | "txt"))
            {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(dir, &mut out)?;
    Ok(out)
}

/// Extension that renders a template into a HTML response or string.
pub trait RenderExt {
    fn render<T: Serialize>(&self, name: &str, ctx: T) -> Result<Response>;

    /// Same merge as [`Self::render`], but returns the HTML/string body (e.g. for email).
    fn render_str<T: Serialize>(&self, name: &str, ctx: T) -> Result<String>;
}

impl RenderExt for Request {
    fn render<T: Serialize>(&self, name: &str, ctx: T) -> Result<Response> {
        let templates = self.state::<MiniJinjaTemplates>();
        templates.render_html(self, name, ctx)
    }

    fn render_str<T: Serialize>(&self, name: &str, ctx: T) -> Result<String> {
        let templates = self.state::<MiniJinjaTemplates>();
        templates.render_str(self, name, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sova_core::{App, Request};
    use tempfile::tempdir;

    fn install_app(dir: &std::path::Path, builder: MiniJinjaTemplatesBuilder) -> App {
        let mut app = App::new();
        app.install(builder);
        let _ = dir;
        app
    }

    #[tokio::test]
    async fn xss_is_escaped_by_default() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("home.html"), r#"<h1>{{ title }}</h1>"#).unwrap();

        let mut app = install_app(dir.path(), Templates::minijinja(dir.path()).autoreload(false));
        app.get("/", |req: Request| async move {
            req.render(
                "home.html",
                serde_json::json!({ "title": "<script>alert(1)</script>" }),
            )
        });

        let res = app.handle_request(http::Method::GET, "/", "").await;
        let body = std::str::from_utf8(res.body_bytes().unwrap()).unwrap();
        assert!(body.contains("&lt;script&gt;alert(1)"), "{body}");
        assert!(
            body.contains("&lt;&#x2f;script&gt;") || body.contains("&lt;/script&gt;"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn ambient_t_is_called_and_sees_per_request_state() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("home.html"), r#"<p>{{ t("k") }}</p>"#).unwrap();

        let mut app = install_app(
            dir.path(),
            Templates::minijinja(dir.path())
                .autoreload(false)
                .per_request("t", |req: &Request| {
                    let prefix = req.header("x-prefix").unwrap_or("no-prefix").to_string();
                    Value::from_function(move |key: String| format!("{prefix}:{key}"))
                }),
        );

        app.get("/", |req: Request| async move {
            req.render("home.html", serde_json::json!({}))
        });

        let res = app.handle_request(http::Method::GET, "/", "").await;
        let body = std::str::from_utf8(res.body_bytes().unwrap()).unwrap();
        assert!(body.contains("no-prefix:k"), "{body}");
    }

    #[tokio::test]
    async fn context_merge_priority_local_over_per_request_over_global() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("home.html"), r#"<p>{{ label }}</p>"#).unwrap();

        let mut app = install_app(
            dir.path(),
            Templates::minijinja(dir.path())
                .autoreload(false)
                .global("label", "global")
                .per_request("label", |_| Value::from("per-request")),
        );

        app.get("/g", |req: Request| async move {
            // No local override → per-request wins over global.
            req.render("home.html", serde_json::json!({}))
        });
        app.get("/l", |req: Request| async move {
            req.render("home.html", serde_json::json!({ "label": "local" }))
        });

        let g = app.handle_request(http::Method::GET, "/g", "").await;
        let body = std::str::from_utf8(g.body_bytes().unwrap()).unwrap();
        assert!(body.contains("per-request"), "{body}");

        let l = app.handle_request(http::Method::GET, "/l", "").await;
        let body = std::str::from_utf8(l.body_bytes().unwrap()).unwrap();
        assert!(body.contains("local"), "{body}");
    }

    #[tokio::test]
    async fn nested_template_path_loads() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("blog")).unwrap();
        std::fs::write(dir.path().join("blog/post.html"), r#"<h1>{{ title }}</h1>"#).unwrap();

        let mut app = install_app(dir.path(), Templates::minijinja(dir.path()).autoreload(false));
        app.get("/", |req: Request| async move {
            req.render("blog/post.html", serde_json::json!({ "title": "Post" }))
        });

        let res = app.handle_request(http::Method::GET, "/", "").await;
        let body = std::str::from_utf8(res.body_bytes().unwrap()).unwrap();
        assert!(body.contains("<h1>Post</h1>"), "{body}");
    }

    #[tokio::test]
    async fn missing_template_names_template_in_error() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("home.html"), "ok").unwrap();

        let mut app = install_app(dir.path(), Templates::minijinja(dir.path()).autoreload(false));
        app.get("/", |req: Request| async move {
            match req.render("nope.html", serde_json::json!({})) {
                Ok(r) => r,
                Err(e) => e.into_response(),
            }
        });

        let res = app.handle_request(http::Method::GET, "/", "").await;
        assert_eq!(res.status_code().as_u16(), 500);
        let body = std::str::from_utf8(res.body_bytes().unwrap()).unwrap();
        assert!(body.contains("nope.html"), "{body}");
    }

    #[tokio::test]
    async fn render_error_includes_line_number() {
        let dir = tempdir().unwrap();
        // Line 1 ok, line 2 undefined filter → error at line 2.
        std::fs::write(
            dir.path().join("bad.html"),
            "ok\n{{ title | definitely_not_a_filter }}\n",
        )
        .unwrap();

        let mut app = install_app(dir.path(), Templates::minijinja(dir.path()).autoreload(false));
        app.get("/", |req: Request| async move {
            match req.render("bad.html", serde_json::json!({ "title": "x" })) {
                Ok(r) => r,
                Err(e) => e.into_response(),
            }
        });

        let res = app.handle_request(http::Method::GET, "/", "").await;
        assert_eq!(res.status_code().as_u16(), 500);
        let body = std::str::from_utf8(res.body_bytes().unwrap()).unwrap();
        assert!(body.contains("bad.html"), "{body}");
        // MiniJinja attaches line; our formatter adds `:2` or `(line 2)`.
        assert!(
            body.contains(":2") || body.contains("line 2") || body.contains("line: 2"),
            "expected line number in error, got: {body}"
        );
    }

    #[tokio::test]
    async fn autoreload_picks_up_file_change() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("home.html");
        std::fs::write(&path, r#"<p>v1</p>"#).unwrap();

        let mut app = install_app(dir.path(), Templates::minijinja(dir.path()).autoreload(true));
        app.get("/", |req: Request| async move {
            req.render("home.html", serde_json::json!({}))
        });
        app.get("/reload", |req: Request| async move {
            req.state::<MiniJinjaTemplates>().request_reload();
            Response::text("ok")
        });

        let v1 = app.handle_request(http::Method::GET, "/", "").await;
        let body = std::str::from_utf8(v1.body_bytes().unwrap()).unwrap();
        assert!(body.contains("v1"), "{body}");

        std::fs::write(&path, r#"<p>v2</p>"#).unwrap();
        let _ = app.handle_request(http::Method::GET, "/reload", "").await;

        let v2 = app.handle_request(http::Method::GET, "/", "").await;
        let body = std::str::from_utf8(v2.body_bytes().unwrap()).unwrap();
        assert!(body.contains("v2"), "{body}");
    }
}
