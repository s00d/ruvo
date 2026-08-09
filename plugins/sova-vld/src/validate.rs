//! `Validate<T>` route attribute, erased hook, and `req.valid()`.

use crate::coerce::coerce_object;
use crate::ValidationError;
use sova_core::extend::{named, BoxFuture, MwEntry, RouteTable, RouteValue};
use sova_core::{App, Request, Response, Router};
use serde_json::{Map, Value};
use std::any::type_name;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::sync::Arc;
use vld::schema::VldParse;

/// Where to read input for [`Validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidateSource {
    Body,
    Query,
    Params,
    /// Merge: query, then body, then params (params win).
    All,
    /// Multipart form (feature `form`).
    Form,
}

/// Route attribute: declare the validated DTO for a route.
pub struct Validate<T> {
    pub source: ValidateSource,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Validate<T> {
    pub fn body() -> Self {
        Self {
            source: ValidateSource::Body,
            _ty: PhantomData,
        }
    }
    pub fn query() -> Self {
        Self {
            source: ValidateSource::Query,
            _ty: PhantomData,
        }
    }
    pub fn params() -> Self {
        Self {
            source: ValidateSource::Params,
            _ty: PhantomData,
        }
    }
    pub fn all() -> Self {
        Self {
            source: ValidateSource::All,
            _ty: PhantomData,
        }
    }
    pub fn form() -> Self {
        Self {
            source: ValidateSource::Form,
            _ty: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> RouteValue for Validate<T> {
    fn label(&self) -> Cow<'static, str> {
        let src = match self.source {
            ValidateSource::Body => "body",
            ValidateSource::Query => "query",
            ValidateSource::Params => "params",
            ValidateSource::All => "all",
            ValidateSource::Form => "form",
        };
        Cow::Owned(format!("Validate<{src}: {}>", type_name::<T>()))
    }
}

/// Parsed DTO stored on the request by [`ValidateHook`].
#[derive(Debug, Clone)]
pub struct Validated<T>(pub T);

#[allow(clippy::type_complexity, clippy::result_large_err)]
type HookFn = Arc<dyn Fn(Request) -> BoxFuture<Result<Request, Response>> + Send + Sync>;

/// Type-erased runner invoked by route middleware.
pub struct ValidateHook {
    run: HookFn,
}

impl RouteValue for ValidateHook {
    fn label(&self) -> Cow<'static, str> {
        Cow::Borrowed("ValidateHook")
    }
}

impl ValidateHook {
    pub fn wrap<F>(f: F) -> Self
    where
        F: Fn(Request) -> BoxFuture<Result<Request, Response>> + Send + Sync + 'static,
    {
        Self { run: Arc::new(f) }
    }

    pub fn body<T: VldParse + Send + Sync + 'static>() -> Self {
        Self::wrap(|mut req| {
            Box::pin(async move {
                let value = read_body_json(&mut req).await.map_err(|e| e.respond(&req))?;
                finish::<T>(req, value)
            })
        })
    }

    pub fn query<T: VldParse + Send + Sync + 'static>() -> Self {
        Self::wrap(|req| {
            Box::pin(async move {
                let value = read_query_value(&req);
                finish::<T>(req, value)
            })
        })
    }

    pub fn params<T: VldParse + Send + Sync + 'static>() -> Self {
        Self::wrap(|req| {
            Box::pin(async move {
                let value = read_params_value(&req);
                finish::<T>(req, value)
            })
        })
    }

    pub fn all<T: VldParse + Send + Sync + 'static>() -> Self {
        Self::wrap(|mut req| {
            Box::pin(async move {
                let query = read_query_value(&req);
                let body = match read_body_json(&mut req).await {
                    Ok(b) => b,
                    Err(_) if matches!(req.method, http::Method::GET | http::Method::HEAD) => {
                        Value::Object(Map::new())
                    }
                    Err(e) => return Err(e.respond(&req)),
                };
                let params = read_params_value(&req);
                finish::<T>(req, merge_objects(query, body, params))
            })
        })
    }

    pub fn form<T: VldParse + Send + Sync + 'static>() -> Self {
        #[cfg(feature = "form")]
        {
            Self::wrap(|mut req| {
                Box::pin(async move {
                    let value = read_form_value(&mut req)
                        .await
                        .map_err(|e| e.respond(&req))?;
                    finish::<T>(req, value)
                })
            })
        }
        #[cfg(not(feature = "form"))]
        {
            let _ = type_name::<T>();
            Self::wrap(|_req| {
                Box::pin(async move {
                    Err(Response::text(
                        "Validate::form requires sova_vld feature `form`",
                    )
                    .status(500))
                })
            })
        }
    }

    pub(crate) fn run(&self, req: Request) -> BoxFuture<Result<Request, Response>> {
        (self.run)(req)
    }
}

#[allow(clippy::result_large_err)]
fn finish<T: VldParse + Send + Sync + 'static>(
    mut req: Request,
    value: Value,
) -> Result<Request, Response> {
    match T::vld_parse_value(&value) {
        Ok(v) => {
            req.set(Validated(v));
            Ok(req)
        }
        Err(e) => {
            let err = ValidationError::from(e);
            #[cfg(feature = "i18n")]
            let err = crate::i18n_msg::localize(err, &req);
            Err(err.respond(&req))
        }
    }
}

fn merge_objects(query: Value, body: Value, params: Value) -> Value {
    let mut map = Map::new();
    if let Value::Object(q) = query {
        map.extend(q);
    }
    if let Value::Object(b) = body {
        map.extend(b);
    }
    if let Value::Object(p) = params {
        map.extend(p);
    }
    Value::Object(map)
}

pub(crate) async fn read_body_json(req: &mut Request) -> Result<Value, ValidationError> {
    let bytes = req.body().await?;
    if bytes.is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_slice(&bytes).map_err(|e| {
        ValidationError(vld::error::VldError::single(
            vld::error::IssueCode::ParseError,
            format!("Invalid JSON: {e}"),
        ))
    })
}

pub(crate) fn read_params_value(req: &Request) -> Value {
    let mut map = Map::new();
    for (k, v) in &req.params {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(map)
}

pub(crate) fn read_query_value(req: &Request) -> Value {
    let raw = req.raw_query();
    if !raw.is_empty() {
        // serde_qs cannot deserialize into `Value` at the top level; use Map.
        if let Ok(map) = serde_qs::from_str::<Map<String, Value>>(raw) {
            return Value::Object(map);
        }
    }
    let mut map = Map::new();
    for (k, v) in &req.query {
        map.insert(k.clone(), Value::String(v.clone()));
    }
    Value::Object(map)
}

#[cfg(feature = "form")]
pub(crate) async fn read_form_value(req: &mut Request) -> Result<Value, ValidationError> {
    let data = req.input().await?;
    let mut map = Map::new();
    for (name, values) in data.text_map() {
        match values.as_slice() {
            [] => {}
            [one] => {
                map.insert(name.clone(), Value::String(one.clone()));
            }
            many => {
                map.insert(
                    name.clone(),
                    Value::Array(many.iter().cloned().map(Value::String).collect()),
                );
            }
        }
    }
    for (name, uploads) in data.file_map() {
        if let Some(f) = uploads.first() {
            map.insert(
                name.clone(),
                serde_json::json!({
                    "filename": f.filename,
                    "content_type": f.content_type,
                    "size": f.data.len(),
                }),
            );
        }
    }
    Ok(Value::Object(map))
}

#[cfg_attr(not(feature = "openapi"), allow(dead_code))]
pub(crate) fn coerce_with_schema(value: &mut Value, schema: &Value) {
    if let Value::Object(map) = value {
        coerce_object(map, schema);
    }
}

/// Access DTO stored by [`ValidateHook`].
pub trait ValidExt {
    fn valid<T: Send + Sync + 'static>(&self) -> &T;
    fn take_valid<T: Send + Sync + 'static>(&mut self) -> Option<T>;
}

impl ValidExt for Request {
    fn valid<T: Send + Sync + 'static>(&self) -> &T {
        self.get::<Validated<T>>()
            .map(|v| &v.0)
            .unwrap_or_else(|| {
                panic!(
                    "validated `{}` missing — use `.validate_body::<T>()` (etc.) on the route",
                    type_name::<T>()
                )
            })
    }

    fn take_valid<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.take::<Validated<T>>().map(|v| v.0)
    }
}

fn vld_mw() -> MwEntry {
    named("vld", |req: Request, next: sova_core::Next| async move {
        if let Some(hook) = req.route_meta::<ValidateHook>() {
            match hook.run(req).await {
                Ok(req) => next(req).await,
                Err(res) => res,
            }
        } else {
            next(req).await
        }
    })
}

#[cfg(feature = "openapi")]
pub trait ValidateSchema: crate::VldDocSchema {}
#[cfg(feature = "openapi")]
impl<T: crate::VldDocSchema> ValidateSchema for T {}

#[cfg(not(feature = "openapi"))]
pub trait ValidateSchema {}
#[cfg(not(feature = "openapi"))]
impl<T> ValidateSchema for T {}

/// Sugar: attach [`Validate`] + [`ValidateHook`] (+ OpenAPI schema when enabled).
pub trait ValidateRouteExt {
    fn validate_body<T>(&mut self) -> &mut Self
    where
        T: VldParse + Send + Sync + 'static + ValidateSchema;
    fn validate_query<T>(&mut self) -> &mut Self
    where
        T: VldParse + Send + Sync + 'static + ValidateSchema;
    fn validate_params<T>(&mut self) -> &mut Self
    where
        T: VldParse + Send + Sync + 'static + ValidateSchema;
    fn validate_all<T>(&mut self) -> &mut Self
    where
        T: VldParse + Send + Sync + 'static + ValidateSchema;
    fn validate_form<T>(&mut self) -> &mut Self
    where
        T: VldParse + Send + Sync + 'static + ValidateSchema;
}

fn attach_common<T>(router: &mut Router, validate: Validate<T>, hook: ValidateHook)
where
    T: Send + Sync + 'static,
{
    router.with(validate);
    router.with(hook);
    router.route_middleware(vld_mw());
}

#[cfg(feature = "openapi")]
fn attach_openapi(router: &mut Router, source: ValidateSource, schema: Value) {
    use sova_openapi::OpenApiValidate;
    router.with_update(|o: &mut OpenApiValidate| match source {
        ValidateSource::Body | ValidateSource::Form => o.body = Some(schema),
        ValidateSource::Query => o.query = Some(schema),
        ValidateSource::Params => o.params = Some(schema),
        ValidateSource::All => {
            o.body = Some(schema.clone());
            o.query = Some(schema.clone());
            o.params = Some(schema);
        }
    });
}

trait AsRouterMut {
    fn as_router_mut(&mut self) -> &mut Router;
}
impl AsRouterMut for Router {
    fn as_router_mut(&mut self) -> &mut Router {
        self
    }
}
impl AsRouterMut for App {
    fn as_router_mut(&mut self) -> &mut Router {
        &mut *self
    }
}

macro_rules! impl_ext {
    ($target:ty) => {
        impl ValidateRouteExt for $target {
            fn validate_body<T>(&mut self) -> &mut Self
            where
                T: VldParse + Send + Sync + 'static + ValidateSchema,
            {
                let router = self.as_router_mut();
                attach_common(router, Validate::<T>::body(), ValidateHook::body::<T>());
                #[cfg(feature = "openapi")]
                attach_openapi(router, ValidateSource::Body, T::json_schema());
                self
            }
            fn validate_query<T>(&mut self) -> &mut Self
            where
                T: VldParse + Send + Sync + 'static + ValidateSchema,
            {
                let router = self.as_router_mut();
                #[cfg(feature = "openapi")]
                {
                    let schema = T::json_schema();
                    let schema2 = schema.clone();
                    let hook = ValidateHook::wrap(move |req| {
                        let schema2 = schema2.clone();
                        Box::pin(async move {
                            let mut value = read_query_value(&req);
                            coerce_with_schema(&mut value, &schema2);
                            finish::<T>(req, value)
                        })
                    });
                    attach_common(router, Validate::<T>::query(), hook);
                    attach_openapi(router, ValidateSource::Query, schema);
                }
                #[cfg(not(feature = "openapi"))]
                {
                    attach_common(router, Validate::<T>::query(), ValidateHook::query::<T>());
                }
                self
            }
            fn validate_params<T>(&mut self) -> &mut Self
            where
                T: VldParse + Send + Sync + 'static + ValidateSchema,
            {
                let router = self.as_router_mut();
                #[cfg(feature = "openapi")]
                {
                    let schema = T::json_schema();
                    let schema2 = schema.clone();
                    let hook = ValidateHook::wrap(move |req| {
                        let schema2 = schema2.clone();
                        Box::pin(async move {
                            let mut value = read_params_value(&req);
                            coerce_with_schema(&mut value, &schema2);
                            finish::<T>(req, value)
                        })
                    });
                    attach_common(router, Validate::<T>::params(), hook);
                    attach_openapi(router, ValidateSource::Params, schema);
                }
                #[cfg(not(feature = "openapi"))]
                {
                    attach_common(router, Validate::<T>::params(), ValidateHook::params::<T>());
                }
                self
            }
            fn validate_all<T>(&mut self) -> &mut Self
            where
                T: VldParse + Send + Sync + 'static + ValidateSchema,
            {
                let router = self.as_router_mut();
                #[cfg(feature = "openapi")]
                {
                    let schema = T::json_schema();
                    let schema2 = schema.clone();
                    let hook = ValidateHook::wrap(move |mut req| {
                        let schema2 = schema2.clone();
                        Box::pin(async move {
                            let mut query = read_query_value(&req);
                            coerce_with_schema(&mut query, &schema2);
                            let body = match read_body_json(&mut req).await {
                                Ok(b) => b,
                                Err(_)
                                    if matches!(
                                        req.method,
                                        http::Method::GET | http::Method::HEAD
                                    ) =>
                                {
                                    Value::Object(Map::new())
                                }
                                Err(e) => return Err(e.respond(&req)),
                            };
                            let mut params = read_params_value(&req);
                            coerce_with_schema(&mut params, &schema2);
                            finish::<T>(req, merge_objects(query, body, params))
                        })
                    });
                    attach_common(router, Validate::<T>::all(), hook);
                    attach_openapi(router, ValidateSource::All, schema);
                }
                #[cfg(not(feature = "openapi"))]
                {
                    attach_common(router, Validate::<T>::all(), ValidateHook::all::<T>());
                }
                self
            }
            fn validate_form<T>(&mut self) -> &mut Self
            where
                T: VldParse + Send + Sync + 'static + ValidateSchema,
            {
                let router = self.as_router_mut();
                attach_common(router, Validate::<T>::form(), ValidateHook::form::<T>());
                #[cfg(feature = "openapi")]
                attach_openapi(router, ValidateSource::Form, T::json_schema());
                self
            }
        }
    };
}

impl_ext!(Router);
impl_ext!(App);

/// POST/PUT/PATCH routes that lack a [`ValidateHook`].
pub fn missing_validate_routes(table: &RouteTable) -> Vec<String> {
    let mut missing = Vec::new();
    for entry in &table.0 {
        let sova_core::extend::RouteEntry::Http {
            method,
            path,
            meta,
        } = entry
        else {
            continue;
        };
        if !matches!(
            *method,
            http::Method::POST | http::Method::PUT | http::Method::PATCH
        ) {
            continue;
        }
        if meta.get::<ValidateHook>().is_some() {
            continue;
        }
        missing.push(format!("{method} {path}"));
    }
    missing
}

/// Plugin: coverage check for POST/PUT/PATCH without [`ValidateHook`].
pub struct Vld;

impl sova_core::Plugin for Vld {
    fn id(&self) -> &'static str {
        "vld"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Validation")
            .description("Request validation hooks and coverage check")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.register_audit("vld", |state| async move {
            let Some(table) = state.get::<RouteTable>() else {
                return Ok(());
            };
            let missing = missing_validate_routes(&table);
            if missing.is_empty() {
                Ok(())
            } else {
                Err(sova_core::Error::Internal(format!(
                    "POST/PUT/PATCH without Validate: {}",
                    missing.join(", ")
                )))
            }
        });
    }
}
