//! GraphQL console — in-process schema execute.

use super::{json_response, ActionResponse};
use async_graphql::Request as GqlRequest;
use serde::Deserialize;
use serde_json::{json, Value};
use sova_core::{Request, Response};
use sova_graphql::{GraphqlJsonRequest, SchemaHandle};
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct GraphqlActionRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<Value>,
    #[serde(default)]
    pub operation_name: Option<String>,
}

pub async fn handle(req: &mut Request) -> Response {
    let started = Instant::now();
    let body: GraphqlActionRequest = match req.json().await {
        Ok(b) => b,
        Err(e) => {
            return json_response(ActionResponse::err(format!("invalid JSON: {e}"), 0.0), 400);
        }
    };

    if body.query.trim().is_empty() {
        return json_response(ActionResponse::err("query required", 0.0), 400);
    }

    let handle = match req.try_state::<SchemaHandle>() {
        Some(h) => h,
        None => {
            return json_response(
                ActionResponse::err("GraphQL server not installed", 0.0),
                503,
            );
        }
    };

    let gql_req: GqlRequest = GraphqlJsonRequest {
        query: body.query,
        operation_name: body.operation_name,
        variables: body.variables,
    }
    .into();

    let response = handle.execute(gql_req).await;
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    let value = serde_json::to_value(&response).unwrap_or(json!({ "error": "serialize failed" }));
    json_response(ActionResponse::ok(value, ms), 200)
}
