//! Security response headers (frame, nosniff, referrer). HSTS stays on [`ruvo_core::Tls`].

use ruvo_core::extend::{named, with_leaked};
use ruvo_core::{App, Plugin};

/// Apply common browser security headers on every response.
#[derive(Clone)]
pub struct Shield {
    frame: Option<&'static str>,
    content_type: Option<&'static str>,
    referrer: Option<&'static str>,
}

impl Shield {
    pub fn new() -> Self {
        Self {
            frame: Some("SAMEORIGIN"),
            content_type: Some("nosniff"),
            referrer: Some("no-referrer-when-downgrade"),
        }
    }

    /// `X-Frame-Options`. Pass `None` / `false` via [`Self::frame_off`] to disable.
    pub fn frame(mut self, value: &'static str) -> Self {
        self.frame = Some(value);
        self
    }

    pub fn frame_off(mut self) -> Self {
        self.frame = None;
        self
    }

    pub fn content_type(mut self, value: &'static str) -> Self {
        self.content_type = Some(value);
        self
    }

    pub fn content_type_off(mut self) -> Self {
        self.content_type = None;
        self
    }

    pub fn referrer(mut self, value: &'static str) -> Self {
        self.referrer = Some(value);
        self
    }

    pub fn referrer_off(mut self) -> Self {
        self.referrer = None;
        self
    }
}

impl Default for Shield {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Shield {
    fn id(&self) -> &'static str {
        "shield"
    }

    fn install(self, app: &mut App) {
        app.use_middleware(named(
            "shield",
            with_leaked(self, |shield, req, next| async move {
                let mut res = next(req).await;
                if let Some(v) = shield.frame {
                    res = res.header("x-frame-options", v);
                }
                if let Some(v) = shield.content_type {
                    res = res.header("x-content-type-options", v);
                }
                if let Some(v) = shield.referrer {
                    res = res.header("referrer-policy", v);
                }
                res
            }),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use ruvo_core::{Request, Response};

    #[tokio::test]
    async fn default_headers() {
        let mut app = App::new();
        app.install(Shield::default());
        app.get("/", |_r: Request| async { Response::text("ok") });
        let res = app.handle_request(Method::GET, "/", "").await;
        assert_eq!(
            res.headers()
                .get("x-frame-options")
                .and_then(|v| v.to_str().ok()),
            Some("SAMEORIGIN")
        );
        assert_eq!(
            res.headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff")
        );
        assert!(res.headers().get("referrer-policy").is_some());
    }
}
