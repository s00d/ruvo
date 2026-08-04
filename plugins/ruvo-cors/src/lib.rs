//! CORS plugin for Ruvo.

use http::Method;
use ruvo_core::extend::{named, with_leaked};
use ruvo_core::{App, Plugin, Response};

/// Simple CORS plugin.
#[derive(Clone)]
pub struct Cors {
    origin: String,
    methods: String,
    headers: String,
}

impl Cors {
    pub fn new() -> Self {
        Self {
            origin: "*".into(),
            methods: "GET, POST, PUT, PATCH, DELETE, OPTIONS".into(),
            headers: "Content-Type, Authorization".into(),
        }
    }

    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = origin.into();
        self
    }

    pub fn methods(mut self, methods: impl Into<String>) -> Self {
        self.methods = methods.into();
        self
    }

    pub fn headers(mut self, headers: impl Into<String>) -> Self {
        self.headers = headers.into();
        self
    }
}

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Cors {
    fn install(self, app: &mut App) {
        app.use_middleware(named(
            "cors",
            with_leaked(self, |cors, req, next| async move {
                if req.method == Method::OPTIONS {
                    return Response::empty()
                        .status(204)
                        .header("access-control-allow-origin", &cors.origin)
                        .header("access-control-allow-methods", &cors.methods)
                        .header("access-control-allow-headers", &cors.headers);
                }
                let mut res = next(req).await;
                res = res.header("access-control-allow-origin", &cors.origin);
                res
            }),
        ));
    }
}
