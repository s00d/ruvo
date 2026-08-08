//! Trailing-slash 301 middleware.

use crate::canonical::apply_slash;
use crate::defaults::{MetaDefaults, TrailingSlash};
use sova_core::extend::named;
use sova_core::{App, IntoResponse, Next, Redirect, Request};

pub fn install_slash_middleware(app: &mut App) {
    app.use_middleware(named("meta-slash", |req: Request, next: Next| async move {
        let policy = req
            .try_state::<MetaDefaults>()
            .map(|d| d.trailing_slash)
            .unwrap_or(TrailingSlash::Keep);
        if policy == TrailingSlash::Keep || req.method != http::Method::GET {
            return next(req).await;
        }
        let normalized = apply_slash(&req.path, policy);
        if normalized != req.path {
            return Redirect::permanent(normalized).into_response();
        }
        next(req).await
    }));
}
