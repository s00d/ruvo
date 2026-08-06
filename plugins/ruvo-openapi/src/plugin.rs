use crate::build::{build_document, BuildOptions};
use crate::doc::Doc;
use ruvo_core::extend::{RouteTable, RouteValue};
use ruvo_core::{App, Plugin, Request, Response};
use std::path::PathBuf;
use std::sync::Arc;

impl RouteValue for Doc {}

/// Attach [`Doc`] metadata to the last registered HTTP route.
pub trait OpenApiDocExt {
    fn doc(&mut self, doc: Doc) -> &mut Self;
}

impl OpenApiDocExt for App {
    fn doc(&mut self, doc: Doc) -> &mut Self {
        self.with(doc);
        self
    }
}

impl OpenApiDocExt for ruvo_core::Router {
    fn doc(&mut self, doc: Doc) -> &mut Self {
        self.with(doc);
        self
    }
}

/// Serves Scalar UI and OpenAPI JSON from the compiled [`RouteTable`].
pub struct OpenApi {
    title: String,
    version: String,
    servers: Vec<String>,
    mount: String,
    local_assets: Option<PathBuf>,
}

impl OpenApi {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            servers: Vec::new(),
            mount: "/docs".into(),
            local_assets: None,
        }
    }

    pub fn server(mut self, url: impl Into<String>) -> Self {
        self.servers.push(url.into());
        self
    }

    pub fn mount(mut self, path: impl Into<String>) -> Self {
        self.mount = path.into();
        self
    }

    /// Prefer a local `scalar.js` next to the HTML (offline). CDN used when unset.
    pub fn local_assets(mut self, dir: impl Into<PathBuf>) -> Self {
        self.local_assets = Some(dir.into());
        self
    }
}

#[derive(Clone)]
struct OpenApiState {
    title: String,
    version: String,
    servers: Vec<String>,
    mount: String,
    script_src: String,
}

impl Plugin for OpenApi {
    fn install(self, app: &mut App) {
        let mount = if self.mount.ends_with('/') && self.mount.len() > 1 {
            self.mount.trim_end_matches('/').to_string()
        } else {
            self.mount.clone()
        };
        let openapi_path = format!("{mount}/openapi.json");
        let script_src = match &self.local_assets {
            Some(dir) => {
                // Path relative to mount — apps can static-serve `dir` at `{mount}/`.
                let file = dir.join("scalar.js");
                format!("{mount}/{}", file.file_name().and_then(|s| s.to_str()).unwrap_or("scalar.js"))
            }
            None => "https://cdn.jsdelivr.net/npm/@scalar/api-reference".to_string(),
        };
        let state = Arc::new(OpenApiState {
            title: self.title,
            version: self.version,
            servers: self.servers,
            mount: mount.clone(),
            script_src,
        });

        let json_state = Arc::clone(&state);
        app.get(&openapi_path, move |req: Request| {
            let json_state = Arc::clone(&json_state);
            async move {
                let table = req.state::<RouteTable>();
                let doc = build_document(
                    table.as_ref(),
                    &BuildOptions {
                        title: &json_state.title,
                        version: &json_state.version,
                        servers: &json_state.servers,
                        docs_prefix: &json_state.mount,
                    },
                );
                Response::json(&doc)
            }
        })
        .doc(Doc::skip());

        let html_state = Arc::clone(&state);
        app.get(&mount, move |_req: Request| {
            let html_state = Arc::clone(&html_state);
            async move {
                let spec_url = format!("{}/openapi.json", html_state.mount);
                Response::html(scalar_html(&spec_url, &html_state.script_src))
            }
        })
        .doc(Doc::skip());

        let mount_check = mount.clone();
        app.register_check("openapi", move |state| {
            let mount = mount_check.clone();
            async move {
                let Some(table) = state.get::<RouteTable>() else {
                    return Ok(());
                };
                let missing = crate::undocumented_from_table(table.as_ref(), &mount);
                if !missing.is_empty() {
                    return Err(ruvo_core::Error::Internal(format!(
                        "undocumented routes: {}",
                        missing.join(", ")
                    )));
                }
                Ok(())
            }
        });
    }
}

fn scalar_html(spec_url: &str, script_src: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <title>API Reference</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
  <script id="api-reference" data-url="{spec_url}"></script>
  <script src="{script_src}"></script>
</body>
</html>"#
    )
}
