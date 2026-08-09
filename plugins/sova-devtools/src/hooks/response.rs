use crate::collector::{DevToolsBag, RateLimitSnap};
use sova_core::Response;

pub fn collect_response_meta(bag: &DevToolsBag, res: &Response) {
    let h = res.headers();
    let encoding = h
        .get(http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    bag.set_encoding(encoding);

    let limit = h
        .get("ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    let remaining = h
        .get("ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    let reset = h
        .get("ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    if limit.is_some() || remaining.is_some() || reset.is_some() {
        bag.set_rate_limit(Some(RateLimitSnap {
            limit,
            remaining,
            reset,
        }));
    }
}
