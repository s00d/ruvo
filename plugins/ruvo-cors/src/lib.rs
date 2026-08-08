//! CORS plugin for Ruvo (Express [`cors`](https://expressjs.com/en/resources/middleware/cors.html)-style).

use http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE, VARY,
};
use http::Method;
use ruvo_core::extend::{named, with_leaked};
use ruvo_core::{App, Plugin, Request, Response};

#[derive(Clone, Debug)]
enum OriginMode {
    /// Literal `Access-Control-Allow-Origin` value (`*` or a single origin).
    Exact(String),
    /// Mirror request `Origin` when it is in the list.
    List(Vec<String>),
}

/// CORS plugin.
#[derive(Clone)]
pub struct Cors {
    origin: OriginMode,
    origin_explicit: bool,
    methods: String,
    /// Empty string → reflect `Access-Control-Request-Headers` on preflight.
    headers: String,
    exposed: Option<String>,
    credentials: bool,
    credentials_explicit: bool,
    max_age: Option<u64>,
}

impl Cors {
    pub fn new() -> Self {
        Self {
            origin: OriginMode::Exact("*".into()),
            origin_explicit: false,
            methods: "GET, POST, PUT, PATCH, DELETE, OPTIONS".into(),
            // CSRF headers for cookie-session SPAs (Laravel / axios XSRF).
            headers: "Content-Type, Authorization, X-CSRF-Token, X-XSRF-TOKEN".into(),
            exposed: None,
            credentials: false,
            credentials_explicit: false,
            max_age: Some(86400),
        }
    }

    /// Single allowed origin, or `"*"` (Express `origin` string).
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = OriginMode::Exact(origin.into());
        self.origin_explicit = true;
        self
    }

    /// Allow any of these origins (mirror matching request `Origin`).
    pub fn origins(mut self, origins: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.origin = OriginMode::List(origins.into_iter().map(Into::into).collect());
        self.origin_explicit = true;
        self
    }

    pub fn methods(mut self, methods: impl Into<String>) -> Self {
        self.methods = methods.into();
        self
    }

    /// Allowed request headers. Empty → reflect `Access-Control-Request-Headers`.
    pub fn headers(mut self, headers: impl Into<String>) -> Self {
        self.headers = headers.into();
        self
    }

    /// `Access-Control-Expose-Headers`.
    pub fn exposed(mut self, headers: impl Into<String>) -> Self {
        self.exposed = Some(headers.into());
        self
    }

    /// `Access-Control-Allow-Credentials`.
    ///
    /// When combined with [`Self::origin`]`("*")`, browsers reject `*` + credentials.
    /// This plugin mirrors the request `Origin` instead of emitting `*`.
    pub fn credentials(mut self, on: bool) -> Self {
        self.credentials = on;
        self.credentials_explicit = true;
        self
    }

    /// `Access-Control-Max-Age` for preflight (seconds). `None` omits the header.
    pub fn max_age(mut self, secs: impl Into<Option<u64>>) -> Self {
        self.max_age = secs.into();
        self
    }

    fn apply_config(&mut self, app: &App) {
        let Some(doc) = app.config_doc() else {
            return;
        };
        let Some(section) = doc.section("cors") else {
            return;
        };
        if !self.origin_explicit {
            if let Some(toml::Value::Array(arr)) = section.get("origins") {
                let list: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                if !list.is_empty() {
                    self.origin = OriginMode::List(list);
                }
            } else if let Some(o) = section.get("origin").and_then(|v| v.as_str()) {
                self.origin = OriginMode::Exact(o.to_string());
            }
        }
        if !self.credentials_explicit {
            if let Some(v) = section.get("credentials").and_then(|v| v.as_bool()) {
                self.credentials = v;
            }
        }
        if let Some(v) = section.get("max_age").and_then(|v| v.as_integer()) {
            self.max_age = Some(v as u64);
        }
    }
}

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Cors {
    fn id(&self) -> &'static str {
        "cors"
    }

    fn meta(&self) -> ruvo_core::PluginMeta {
        ruvo_core::PluginMeta::new("CORS")
            .description("Cross-Origin Resource Sharing headers")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        self.apply_config(app);
        app.use_middleware(named(
            "cors",
            with_leaked(self, |cors, req, next| async move {
                if req.method == Method::OPTIONS {
                    return preflight(cors, &req);
                }
                let origin = req.header("origin").map(str::to_owned);
                let mut res = next(req).await;
                apply_cors(cors, origin.as_deref(), None, &mut res, false);
                res
            }),
        ));
    }
}

fn preflight(cors: &Cors, req: &Request) -> Response {
    let mut res = Response::empty().status(204);
    let origin = req.header("origin");
    let acrh = req.header("access-control-request-headers");
    apply_cors(cors, origin, acrh, &mut res, true);
    res
}

fn apply_cors(
    cors: &Cors,
    req_origin: Option<&str>,
    acrh: Option<&str>,
    res: &mut Response,
    preflight: bool,
) {
    let Some(allow_origin) = resolve_origin(&cors.origin, req_origin, cors.credentials) else {
        return;
    };

    let mirrored_star = cors.credentials
        && matches!(&cors.origin, OriginMode::Exact(o) if o == "*")
        && allow_origin != "*";
    if !matches!(&cors.origin, OriginMode::Exact(o) if o == "*") || mirrored_star {
        append_vary(res, "Origin");
    }

    set_header(res, ACCESS_CONTROL_ALLOW_ORIGIN, &allow_origin);

    if cors.credentials {
        set_header(res, ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
    }

    if let Some(ref exposed) = cors.exposed {
        if !exposed.is_empty() {
            set_header(res, ACCESS_CONTROL_EXPOSE_HEADERS, exposed);
        }
    }

    if preflight {
        set_header(res, ACCESS_CONTROL_ALLOW_METHODS, &cors.methods);
        let allow_headers = if cors.headers.is_empty() {
            acrh.unwrap_or("").to_string()
        } else {
            cors.headers.clone()
        };
        if !allow_headers.is_empty() {
            set_header(res, ACCESS_CONTROL_ALLOW_HEADERS, &allow_headers);
        }
        if let Some(age) = cors.max_age {
            set_header(res, ACCESS_CONTROL_MAX_AGE, &age.to_string());
        }
    }
}

fn resolve_origin(mode: &OriginMode, req_origin: Option<&str>, credentials: bool) -> Option<String> {
    match mode {
        OriginMode::Exact(o) if o == "*" => {
            if credentials {
                // Browsers reject `*` + credentials — mirror request Origin instead.
                req_origin.map(str::to_string)
            } else {
                Some("*".into())
            }
        }
        OriginMode::Exact(o) => Some(o.clone()),
        OriginMode::List(list) => {
            let origin = req_origin?;
            if list.iter().any(|o| o == origin) {
                Some(origin.to_string())
            } else {
                None
            }
        }
    }
}

fn set_header(res: &mut Response, name: http::HeaderName, value: &str) {
    if let Ok(v) = value.parse() {
        res.headers_mut().insert(name, v);
    }
}

fn append_vary(res: &mut Response, token: &str) {
    let existing = res
        .headers()
        .get(VARY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if existing
        .split(',')
        .any(|t| t.trim().eq_ignore_ascii_case(token))
    {
        return;
    }
    let value = if existing.is_empty() {
        token.to_string()
    } else {
        format!("{existing}, {token}")
    };
    if let Ok(v) = value.parse() {
        res.headers_mut().insert(VARY, v);
    }
}
