//! Optional schema mount (feature `server`).

use async_graphql::{ObjectType, Request, Response, Schema, SubscriptionType};
use serde::Deserialize;
use serde_json::Value;
use sova_core::extend::BoxFuture;
use sova_core::{App, Request as SovaRequest, Response as SovaResponse};
use std::sync::Arc;

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
            if let Ok(v) = async_graphql::Variables::from_json(vars) {
                req = req.variables(v);
            }
        }
        req
    }
}

type ExecFn = Arc<dyn Fn(Request) -> BoxFuture<Response> + Send + Sync>;

#[derive(Clone)]
pub struct SchemaHandle {
    exec: ExecFn,
    sdl: Arc<String>,
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
        Self {
            sdl: Arc::new(sdl),
            exec: Arc::new(move |req| {
                let schema = Arc::clone(&schema);
                Box::pin(async move { schema.execute(req).await })
            }),
        }
    }

    pub async fn execute(&self, request: impl Into<Request>) -> Response {
        (self.exec)(request.into()).await
    }

    pub fn sdl(&self) -> &str {
        &self.sdl
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

pub(crate) fn install_mount(app: &mut App, handle: SchemaHandle, path: &str, graphiql: bool) {
    let path = normalize_path(path);
    app.state(handle.clone());

    let h = handle.clone();
    app.post(&path, move |mut req: SovaRequest| {
        let h = h.clone();
        async move {
            let body: GraphqlJsonRequest = match req.json().await {
                Ok(b) => b,
                Err(e) => {
                    return SovaResponse::json(&serde_json::json!({
                        "errors": [{ "message": format!("invalid GraphQL JSON body: {e}") }]
                    }))
                    .status(400);
                }
            };
            let res = h.execute(body).await;
            SovaResponse::json(&res)
        }
    });

    if graphiql {
        let endpoint = path.clone();
        app.get(&path, move |_req: SovaRequest| {
            let endpoint = endpoint.clone();
            async move {
                let html = async_graphql::http::graphiql_source(&endpoint, None);
                SovaResponse::html(html)
            }
        });
    }
}

fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return "/graphql".into();
    }
    if path.starts_with('/') {
        path.trim_end_matches('/').to_string()
    } else {
        format!("/{}", path.trim_end_matches('/'))
    }
}
