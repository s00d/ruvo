use crate::doc::Doc;
use crate::validate_meta::OpenApiValidate;
use http::Method;
use ruvo_core::extend::{to_brace_path, RouteEntry, RouteTable};
use serde_json::{json, Map, Value};

/// Fixed validation error body schema (auto-422).
pub fn validation_error_schema() -> Value {
    json!({
        "type": "object",
        "required": ["error", "issues"],
        "properties": {
            "error": { "type": "string", "const": "validation_failed" },
            "issues": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["path", "code", "message"],
                    "properties": {
                        "path": { "type": "string" },
                        "code": { "type": "string" },
                        "message": { "type": "string" }
                    }
                }
            }
        }
    })
}

pub struct BuildOptions<'a> {
    pub title: &'a str,
    pub version: &'a str,
    pub servers: &'a [String],
    /// Path prefix for docs routes to exclude (e.g. `/docs`).
    pub docs_prefix: &'a str,
}

/// Build an OpenAPI 3.1 document from a [`RouteTable`].
pub fn build_document(table: &RouteTable, opts: &BuildOptions<'_>) -> Value {
    let mut paths = Map::new();

    for entry in &table.0 {
        let RouteEntry::Http {
            method,
            path,
            meta,
        } = entry
        else {
            continue;
        };

        if path == opts.docs_prefix
            || path.starts_with(&format!("{}/", opts.docs_prefix.trim_end_matches('/')))
        {
            continue;
        }

        let doc = meta.get::<Doc>();
        if doc.as_ref().is_some_and(|d| d.is_skip()) {
            continue;
        }

        let Some(brace) = to_brace_path(path) else {
            continue;
        };

        let oav = meta.get::<OpenApiValidate>();
        let method_key = method_name(method);
        let operation = operation_object(path, doc.as_deref(), oav.as_deref());

        paths
            .entry(brace)
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .unwrap()
            .insert(method_key.into(), operation);
    }

    let mut root = json!({
        "openapi": "3.1.0",
        "info": {
            "title": opts.title,
            "version": opts.version,
        },
        "paths": paths,
    });

    if !opts.servers.is_empty() {
        let servers: Vec<Value> = opts
            .servers
            .iter()
            .map(|url| json!({ "url": url }))
            .collect();
        root.as_object_mut()
            .unwrap()
            .insert("servers".into(), Value::Array(servers));
    }

    root
}

fn method_name(method: &Method) -> &'static str {
    match *method {
        Method::GET => "get",
        Method::POST => "post",
        Method::PUT => "put",
        Method::PATCH => "patch",
        Method::DELETE => "delete",
        Method::HEAD => "head",
        Method::OPTIONS => "options",
        _ => "get",
    }
}

fn operation_object(
    express_path: &str,
    doc: Option<&Doc>,
    oav: Option<&OpenApiValidate>,
) -> Value {
    let mut op = Map::new();
    let mut parameters = Vec::new();

    let params_schema = oav
        .and_then(|o| o.params.as_ref())
        .or_else(|| doc.and_then(|d| d.params_schema.as_ref()));
    let query_schema = oav
        .and_then(|o| o.query.as_ref())
        .or_else(|| doc.and_then(|d| d.query_schema.as_ref()));
    let body_schema = oav
        .and_then(|o| o.body.as_ref())
        .or_else(|| doc.and_then(|d| d.body_schema.as_ref()));

    for name in path_param_names(express_path) {
        let schema = params_schema
            .and_then(|s| property_schema(s, &name))
            .unwrap_or_else(|| json!({ "type": "string" }));
        parameters.push(json!({
            "name": name,
            "in": "path",
            "required": true,
            "schema": schema,
        }));
    }

    if let Some(query) = query_schema {
        if let Some(props) = query.get("properties").and_then(|p| p.as_object()) {
            let required = query
                .get("required")
                .and_then(|r| r.as_array())
                .cloned()
                .unwrap_or_default();
            for (name, schema) in props {
                let req = required.iter().any(|v| v.as_str() == Some(name.as_str()));
                parameters.push(json!({
                    "name": name,
                    "in": "query",
                    "required": req,
                    "schema": schema,
                }));
            }
        } else {
            parameters.push(json!({
                "name": "query",
                "in": "query",
                "schema": query,
            }));
        }
    }

    if let Some(body) = body_schema {
        op.insert(
            "requestBody".into(),
            json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": body
                    }
                }
            }),
        );
    }

    if !parameters.is_empty() {
        op.insert("parameters".into(), Value::Array(parameters));
    }

    let mut responses = Map::new();
    if let Some(doc) = doc {
        for (code, schema) in &doc.responses {
            responses.insert(
                code.to_string(),
                json!({
                    "description": status_description(*code),
                    "content": {
                        "application/json": {
                            "schema": schema
                        }
                    }
                }),
            );
        }
    }
    if body_schema.is_some() || query_schema.is_some() {
        responses.entry("422".to_string()).or_insert_with(|| {
            json!({
                "description": "Validation failed",
                "content": {
                    "application/json": {
                        "schema": validation_error_schema()
                    }
                }
            })
        });
    }
    if responses.is_empty() {
        responses.insert("200".into(), json!({ "description": "OK" }));
    }
    op.insert("responses".into(), Value::Object(responses));

    Value::Object(op)
}

fn path_param_names(path: &str) -> Vec<String> {
    path.trim_matches('/')
        .split('/')
        .filter_map(|seg| seg.strip_prefix(':').map(str::to_string))
        .collect()
}

fn property_schema(params_schema: &Value, name: &str) -> Option<Value> {
    params_schema
        .get("properties")
        .and_then(|p| p.get(name))
        .cloned()
}

fn status_description(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        _ => "Response",
    }
}
