use crate::error::{Error, IntoResponse, Result};
use crate::extract::FromRequest;
use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Type-erased async handler: `Request -> Response` (middleware / outer chain).
pub type Handler = Arc<dyn Fn(Request) -> BoxFuture<Response> + Send + Sync>;

/// Leaf handler that may return [`Error`] for `error_handler`.
pub type FallibleHandler =
    Arc<dyn Fn(Request) -> BoxFuture<Result<Response>> + Send + Sync>;

pub(crate) type ErrorHandlerFn =
    Arc<dyn Fn(Error) -> BoxFuture<Response> + Send + Sync>;

/// Convert async functions into a [`FallibleHandler`].
pub trait IntoHandler<T> {
    fn into_handler(self) -> FallibleHandler;
}

pub struct ResponseMarker;
pub struct ResultMarker;

impl<F, Fut, R> IntoHandler<(ResponseMarker,)> for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    fn into_handler(self) -> FallibleHandler {
        Arc::new(move |req| {
            let fut = self(req);
            Box::pin(async move { Ok(fut.await.into_response()) })
        })
    }
}

impl<F, Fut, R> IntoHandler<(ResultMarker,)> for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R>> + Send + 'static,
    R: IntoResponse,
{
    fn into_handler(self) -> FallibleHandler {
        Arc::new(move |req| {
            let fut = self(req);
            Box::pin(async move { Ok(fut.await?.into_response()) })
        })
    }
}

/// Marker for handler errors that map via [`IntoResponse`] (not `error_handler`).
///
/// Implement for plugin error newtypes (e.g. validation). Do **not** implement for [`Error`].
pub trait ErrorResponse: IntoResponse {}

/// `Result<T, E>` where `E: ErrorResponse`.
pub struct FallibleResponseMarker;

impl<F, Fut, R, E> IntoHandler<(FallibleResponseMarker, E)> for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = std::result::Result<R, E>> + Send + 'static,
    R: IntoResponse,
    E: ErrorResponse + 'static,
{
    fn into_handler(self) -> FallibleHandler {
        Arc::new(move |req| {
            let fut = self(req);
            Box::pin(async move {
                match fut.await {
                    Ok(r) => Ok(r.into_response()),
                    Err(e) => Ok(e.into_response()),
                }
            })
        })
    }
}

/// `Fn() -> Fut` handlers that ignore the request (e.g. `|| async { "ok" }`).
pub struct NoArgResponseMarker;

impl<F, Fut, R> IntoHandler<(NoArgResponseMarker,)> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    fn into_handler(self) -> FallibleHandler {
        Arc::new(move |_req| {
            let fut = self();
            Box::pin(async move { Ok(fut.await.into_response()) })
        })
    }
}

impl IntoHandler<()> for FallibleHandler {
    fn into_handler(self) -> FallibleHandler {
        self
    }
}

macro_rules! impl_extract_handlers {
    ($(($marker:ident, $($T:ident),+));+ $(;)?) => {
        $(
            pub struct $marker;
            impl_extract_handlers!(@one $marker, $($T),+);
        )+
    };
    (@one $marker:ident, $($T:ident),+) => {
        impl<FnH, Fut, R, $($T),+> IntoHandler<($marker, $($T),+)> for FnH
        where
            FnH: Fn($($T),+) -> Fut + Send + Sync + 'static,
            $($T: FromRequest + 'static,)+
            Fut: Future<Output = R> + Send + 'static,
            R: IntoResponse,
        {
            fn into_handler(self) -> FallibleHandler {
                let handler = Arc::new(self);
                Arc::new(move |mut req| {
                    let handler = Arc::clone(&handler);
                    Box::pin(async move {
                        $(
                            #[allow(non_snake_case)]
                            let $T = $T::from_request(&mut req).await?;
                        )+
                        Ok(handler($($T),+).await.into_response())
                    })
                })
            }
        }

        impl<FnH, Fut, R, $($T),+> IntoHandler<($marker, ResultMarker, $($T),+)> for FnH
        where
            FnH: Fn($($T),+) -> Fut + Send + Sync + 'static,
            $($T: FromRequest + 'static,)+
            Fut: Future<Output = Result<R>> + Send + 'static,
            R: IntoResponse,
        {
            fn into_handler(self) -> FallibleHandler {
                let handler = Arc::new(self);
                Arc::new(move |mut req| {
                    let handler = Arc::clone(&handler);
                    Box::pin(async move {
                        $(
                            #[allow(non_snake_case)]
                            let $T = $T::from_request(&mut req).await?;
                        )+
                        Ok(handler($($T),+).await?.into_response())
                    })
                })
            }
        }

        impl<FnH, Fut, R, ErrE, $($T),+> IntoHandler<($marker, FallibleResponseMarker, ErrE, $($T),+)>
            for FnH
        where
            FnH: Fn($($T),+) -> Fut + Send + Sync + 'static,
            $($T: FromRequest + Send + 'static,)+
            Fut: Future<Output = std::result::Result<R, ErrE>> + Send + 'static,
            R: IntoResponse,
            ErrE: ErrorResponse + Send + Sync + 'static,
        {
            fn into_handler(self) -> FallibleHandler {
                let handler = Arc::new(self);
                Arc::new(move |mut req| {
                    let handler = Arc::clone(&handler);
                    Box::pin(async move {
                        $(
                            #[allow(non_snake_case)]
                            let $T = $T::from_request(&mut req).await?;
                        )+
                        match handler($($T),+).await {
                            Ok(r) => Ok(r.into_response()),
                            Err(e) => Ok(e.into_response()),
                        }
                    })
                })
            }
        }
    };
}

impl_extract_handlers! {
    (Extract1, T1);
    (Extract2, T1, T2);
    (Extract3, T1, T2, T3);
    (Extract4, T1, T2, T3, T4);
    (Extract5, T1, T2, T3, T4, T5);
    (Extract6, T1, T2, T3, T4, T5, T6);
    (Extract7, T1, T2, T3, T4, T5, T6, T7);
    (Extract8, T1, T2, T3, T4, T5, T6, T7, T8);
}

/// Wrap a fallible leaf so middleware chains see a plain [`Handler`].
pub fn wrap_errors(handler: FallibleHandler, eh: Option<ErrorHandlerFn>) -> Handler {
    Arc::new(move |req| {
        let handler = Arc::clone(&handler);
        let eh = eh.clone();
        Box::pin(async move {
            match handler(req).await {
                Ok(res) => res,
                // Plugin already decided status/body — do not run error_handler.
                Err(Error::Response(res)) => *res,
                Err(err) => match &eh {
                    Some(hook) => hook(err).await,
                    None => err.into_response(),
                },
            }
        })
    })
}
