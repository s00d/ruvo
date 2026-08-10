//! Optional schema mount (feature `server`).

use crate::context::GraphqlContext;
use crate::subscription::{install_subscription_mount, ws_runner, WsRunner};
use async_graphql::{ObjectType, Request, Response, Schema, SubscriptionType};
use serde::Deserialize;
use serde_json::{json, Value};
use sova_core::extend::BoxFuture;
use sova_core::{App, DevToolsConfigRegistry, Request as SovaRequest, Response as SovaResponse};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Deserialize)]
pub struct GraphqlJsonRequest {
    pub query: String,
    #[serde(default)]
    pub operation_name: Option<String>,
    #[serde(default)]
    pub variables: Option<Value>,
}

impl From<GraphqlJsonRequest> for Request {
    fn from(value: GraphqlJsonRequest) -> Self {
        let mut req = Request::new(value.query);
        if let Some(name) = value.operation_name {
            req = req.operation_name(name);
        }
        if let Some(vars) = value.variables {
            req = req.variables(async_graphql::Variables::from_json(vars));
        }
        req
    }
}

type ExecFn = Arc<dyn Fn(Request) -> BoxFuture<Response> + Send + Sync>;

#[derive(Clone)]
pub struct SchemaHandle {
    exec: ExecFn,
    sdl: Arc<String>,
    ws: Option<WsRunner>,
}

impl SchemaHandle {
    pub fn from_schema<Q, M, S>(schema: Schema<Q, M, S>) -> Self
    where
        Q: ObjectType + 'static,
        M: ObjectType + 'static,
        S: SubscriptionType + 'static,
    {
        let sdl = schema.sdl();
        let schema = Arc::new(schema);
        let ws = Some(ws_runner(schema.as_ref().clone()));
        Self {
            sdl: Arc::new(sdl),
            exec: Arc::new({
                let schema = Arc::clone(&schema);
                move |req| {
                    let schema = Arc::clone(&schema);
                    Box::pin(async move { schema.execute(req).await })
                }
            }),
            ws,
        }
    }

    pub async fn execute(&self, request: impl Into<Request>) -> Response {
        (self.exec)(request.into()).await
    }

    pub async fn execute_http(&self, http: &SovaRequest, body: GraphqlJsonRequest) -> Response {
        let op_name = body
            .operation_name
            .clone()
            .unwrap_or_else(|| "(anonymous)".into());
        let kind = operation_kind(&body.query);
        let auth = http.header("authorization").is_some();
        let started = Instant::now();
        let mut gql_req: Request = body.into();
        gql_req = gql_req.data(GraphqlContext::from_request(http));
        let response = self.execute(gql_req).await;
        let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
        tracing::debug!(
            target: "sova.graphql",
            operation = %op_name,
            kind = kind,
            duration_ms = duration_ms,
            errors = response.errors.len(),
            auth = auth,
        );
        response
    }

    pub fn sdl(&self) -> &str {
        &self.sdl
    }

    pub(crate) fn ws_runner(&self) -> Option<WsRunner> {
        self.ws.clone()
    }
}

pub async fn execute_request<Q, M, S>(
    schema: &Schema<Q, M, S>,
    request: impl Into<Request>,
) -> Response
where
    Q: ObjectType + 'static,
    M: ObjectType + 'static,
    S: SubscriptionType + 'static,
{
    schema.execute(request).await
}

/// Server mount options (path, GraphiQL, subscriptions, …).
#[derive(Clone, Debug)]
pub(crate) struct ServerMountConfig {
    pub path: String,
    pub graphiql: bool,
    pub graphiql_path: String,
    pub subscriptions_path: Option<String>,
    pub allow_get_queries: bool,
    pub sdl_path: Option<String>,
}

pub(crate) fn install_server(app: &mut App, handle: SchemaHandle, cfg: ServerMountConfig) {
    register_devtools_mount(app, &cfg);
    let path = normalize_path(&cfg.path);
    app.state(handle.clone());

    let h = handle.clone();
    let allow_get = cfg.allow_get_queries;
    app.post(&path, {
        move |mut req: SovaRequest| {
            let h = h.clone();
            async move { execute_post(&h, &mut req).await }
        }
    });

    if allow_get {
        let h = handle.clone();
        app.get(&path, move |req: SovaRequest| {
            let h = h.clone();
            async move { execute_get(&h, &req).await }
        });
    }

    if cfg.graphiql {
        let graphiql_path = normalize_path(&cfg.graphiql_path);
        let endpoint = path.clone();
        let subscription = cfg.subscriptions_path.as_deref().map(normalize_path);
        app.get(&graphiql_path, move |_req: SovaRequest| {
            let endpoint = endpoint.clone();
            let subscription = subscription.clone();
            async move {
                let html = async_graphql::http::graphiql_source(&endpoint, subscription.as_deref());
                SovaResponse::html(html)
            }
        });
    }

    if let Some(sdl_path) = cfg.sdl_path {
        let sdl_path = normalize_path(&sdl_path);
        let h = handle.clone();
        app.get(&sdl_path, move |_req: SovaRequest| {
            let h = h.clone();
            async move {
                SovaResponse::text(h.sdl()).header("content-type", "text/plain; charset=utf-8")
            }
        });
    }

    if let (Some(sub_path), Some(ws)) = (cfg.subscriptions_path, handle.ws_runner()) {
        install_subscription_mount(app, &sub_path, ws);
    }
}

async fn execute_post(h: &SchemaHandle, req: &mut SovaRequest) -> SovaResponse {
    let body: GraphqlJsonRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return SovaResponse::json(&serde_json::json!({
                "errors": [{ "message": format!("invalid GraphQL JSON body: {e}") }]
            }))
            .status(400);
        }
    };
    SovaResponse::json(&h.execute_http(req, body).await)
}

async fn execute_get(h: &SchemaHandle, req: &SovaRequest) -> SovaResponse {
    let Some(query) = req.query("query") else {
        return SovaResponse::json(&serde_json::json!({
            "errors": [{ "message": "missing query parameter" }]
        }))
        .status(400);
    };
    let body = GraphqlJsonRequest {
        query: query.to_string(),
        operation_name: req.query("operationName").map(str::to_string),
        variables: req
            .query("variables")
            .and_then(|v| serde_json::from_str(v).ok()),
    };
    SovaResponse::json(&h.execute_http(req, body).await)
}

pub(crate) fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return "/graphql".into();
    }
    if path.starts_with('/') {
        path.trim_end_matches('/').to_string()
    } else {
        format!("/{}", path.trim_end_matches('/'))
    }
}

pub(crate) fn default_graphiql_path(api_path: &str) -> String {
    let api = normalize_path(api_path);
    if api == "/graphql" {
        "/graphiql".into()
    } else {
        format!("{api}/graphiql")
    }
}

pub(crate) fn default_subscriptions_path(api_path: &str) -> String {
    let api = normalize_path(api_path);
    format!("{api}/ws")
}

fn operation_kind(query: &str) -> &'static str {
    let head = query.split('{').next().unwrap_or(query).trim();
    if head.starts_with("mutation") {
        "mutation"
    } else if head.starts_with("subscription") {
        "subscription"
    } else {
        "query"
    }
}

fn register_devtools_mount(app: &mut App, cfg: &ServerMountConfig) {
    if app.try_state::<DevToolsConfigRegistry>().is_none() {
        app.state(DevToolsConfigRegistry::default());
    }
    let reg = app
        .try_state::<DevToolsConfigRegistry>()
        .expect("DevToolsConfigRegistry");
    reg.set(
        "graphql",
        json!({
            "api": normalize_path(&cfg.path),
            "graphiql": cfg.graphiql.then(|| normalize_path(&cfg.graphiql_path)),
            "subscriptions": cfg.subscriptions_path.as_ref().map(|p| normalize_path(p)),
            "sdl": cfg.sdl_path.as_ref().map(|p| normalize_path(p)),
            "allow_get_queries": cfg.allow_get_queries,
        }),
    );
}
