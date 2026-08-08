//! `req.http()` — client bound to inbound deadline and trace headers.

use crate::client::{propagation_headers, HttpClient, NamedClient, PendingRequest};
use ruvo_core::RequestId;
use http::Method;
use ruvo_core::Request;

/// Extension: outbound client with request context.
pub trait HttpExt {
    fn http(&self) -> HttpBound<'_>;
}

impl HttpExt for Request {
    fn http(&self) -> HttpBound<'_> {
        HttpBound { req: self }
    }
}

/// Bound view: deadline + propagation applied on each call.
pub struct HttpBound<'a> {
    req: &'a Request,
}

impl HttpBound<'_> {
    fn client(&self) -> std::sync::Arc<HttpClient> {
        self.req.state::<HttpClient>()
    }

    fn apply(&self, mut pending: PendingRequest) -> PendingRequest {
        // Ensure id exists for propagation (middleware may have set it).
        let _ = self.req.get::<RequestId>();
        for (k, v) in propagation_headers(self.req).iter() {
            pending.headers.insert(k.clone(), v.clone());
        }
        pending.with_budget(self.req.deadline_remaining())
    }

    pub fn get(&self, url: impl Into<String>) -> PendingRequest {
        self.apply(self.client().get(url))
    }

    pub fn post(&self, url: impl Into<String>) -> PendingRequest {
        self.apply(self.client().post(url))
    }

    pub fn put(&self, url: impl Into<String>) -> PendingRequest {
        self.apply(self.client().put(url))
    }

    pub fn patch(&self, url: impl Into<String>) -> PendingRequest {
        self.apply(self.client().patch(url))
    }

    pub fn delete(&self, url: impl Into<String>) -> PendingRequest {
        self.apply(self.client().delete(url))
    }

    pub fn request(&self, method: Method, url: impl Into<String>) -> PendingRequest {
        self.apply(self.client().request(method, url))
    }

    pub fn named(&self, name: &str) -> NamedBound<'_> {
        NamedBound {
            req: self.req,
            named: self.client().named(name),
        }
    }
}

pub struct NamedBound<'a> {
    req: &'a Request,
    named: NamedClient,
}

impl NamedBound<'_> {
    fn apply(&self, mut pending: PendingRequest) -> PendingRequest {
        for (k, v) in propagation_headers(self.req).iter() {
            pending.headers.insert(k.clone(), v.clone());
        }
        pending.with_budget(self.req.deadline_remaining())
    }

    pub fn get(&self, path: impl Into<String>) -> PendingRequest {
        self.apply(self.named.get(path))
    }

    pub fn post(&self, path: impl Into<String>) -> PendingRequest {
        self.apply(self.named.post(path))
    }

    pub fn put(&self, path: impl Into<String>) -> PendingRequest {
        self.apply(self.named.put(path))
    }

    pub fn patch(&self, path: impl Into<String>) -> PendingRequest {
        self.apply(self.named.patch(path))
    }

    pub fn delete(&self, path: impl Into<String>) -> PendingRequest {
        self.apply(self.named.delete(path))
    }
}
