//! Security response headers (helmet-style subset). HSTS stays on [`sova_core::Tls`].

use sova_core::extend::{named, with_leaked};
use sova_core::{App, Plugin};

/// Apply common browser security headers on every response.
#[derive(Clone)]
pub struct Shield {
    frame: Option<&'static str>,
    content_type: Option<&'static str>,
    referrer: Option<&'static str>,
    coop: Option<&'static str>,
    corp: Option<&'static str>,
    dns_prefetch: Option<&'static str>,
    download_options: Option<&'static str>,
    permitted_cross_domain: Option<&'static str>,
    xss_protection: Option<&'static str>,
    csp: Option<&'static str>,
    csp_explicit: bool,
}

impl Shield {
    pub fn new() -> Self {
        Self {
            frame: Some("SAMEORIGIN"),
            content_type: Some("nosniff"),
            referrer: Some("no-referrer"),
            coop: Some("same-origin"),
            corp: Some("same-origin"),
            dns_prefetch: Some("off"),
            download_options: Some("noopen"),
            permitted_cross_domain: Some("none"),
            xss_protection: Some("0"),
            csp: None,
            csp_explicit: false,
        }
    }

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

    pub fn cross_origin_opener(mut self, value: &'static str) -> Self {
        self.coop = Some(value);
        self
    }

    pub fn cross_origin_opener_off(mut self) -> Self {
        self.coop = None;
        self
    }

    pub fn cross_origin_resource(mut self, value: &'static str) -> Self {
        self.corp = Some(value);
        self
    }

    pub fn cross_origin_resource_off(mut self) -> Self {
        self.corp = None;
        self
    }

    pub fn dns_prefetch(mut self, value: &'static str) -> Self {
        self.dns_prefetch = Some(value);
        self
    }

    pub fn dns_prefetch_off(mut self) -> Self {
        self.dns_prefetch = None;
        self
    }

    pub fn download_options(mut self, value: &'static str) -> Self {
        self.download_options = Some(value);
        self
    }

    pub fn download_options_off(mut self) -> Self {
        self.download_options = None;
        self
    }

    pub fn permitted_cross_domain(mut self, value: &'static str) -> Self {
        self.permitted_cross_domain = Some(value);
        self
    }

    pub fn permitted_cross_domain_off(mut self) -> Self {
        self.permitted_cross_domain = None;
        self
    }

    pub fn xss_protection(mut self, value: &'static str) -> Self {
        self.xss_protection = Some(value);
        self
    }

    pub fn xss_protection_off(mut self) -> Self {
        self.xss_protection = None;
        self
    }

    /// Raw `Content-Security-Policy` (no default — set explicitly).
    pub fn csp(mut self, policy: &'static str) -> Self {
        self.csp = Some(policy);
        self.csp_explicit = true;
        self
    }

    pub fn csp_off(mut self) -> Self {
        self.csp = None;
        self.csp_explicit = true;
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

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Shield")
            .description("Baseline security response headers (helmet-style)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Some(doc) = app.config_doc() {
            if let Some(section) = doc.section("shield") {
                if !self.csp_explicit {
                    if let Some(p) = section.get("csp").and_then(|v| v.as_str()) {
                        self.csp = Some(Box::leak(p.to_string().into_boxed_str()));
                    }
                }
                if let Some(v) = section.get("frame").and_then(|v| v.as_str()) {
                    self.frame = Some(Box::leak(v.to_string().into_boxed_str()));
                }
            }
        }
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
                if let Some(v) = shield.coop {
                    res = res.header("cross-origin-opener-policy", v);
                }
                if let Some(v) = shield.corp {
                    res = res.header("cross-origin-resource-policy", v);
                }
                if let Some(v) = shield.dns_prefetch {
                    res = res.header("x-dns-prefetch-control", v);
                }
                if let Some(v) = shield.download_options {
                    res = res.header("x-download-options", v);
                }
                if let Some(v) = shield.permitted_cross_domain {
                    res = res.header("x-permitted-cross-domain-policies", v);
                }
                if let Some(v) = shield.xss_protection {
                    res = res.header("x-xss-protection", v);
                }
                if let Some(v) = shield.csp {
                    res = res.header("content-security-policy", v);
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
    use sova_core::{Request, Response};

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
        assert_eq!(
            res.headers()
                .get("referrer-policy")
                .and_then(|v| v.to_str().ok()),
            Some("no-referrer")
        );
        assert_eq!(
            res.headers()
                .get("cross-origin-opener-policy")
                .and_then(|v| v.to_str().ok()),
            Some("same-origin")
        );
        assert_eq!(
            res.headers()
                .get("x-xss-protection")
                .and_then(|v| v.to_str().ok()),
            Some("0")
        );
        assert!(res.headers().get("content-security-policy").is_none());
    }
}
