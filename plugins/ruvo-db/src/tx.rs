use crate::handle::{DbHandle, DbPool};
use crate::DbError;
use ruvo_core::extend::named;
use ruvo_core::{Next, Request, Response};
use sea_orm::TransactionTrait;
use std::sync::Arc;

/// Open a request-scoped transaction; commit on 2xx, otherwise rollback.
pub fn transaction() -> impl ruvo_core::extend::IntoMwEntry {
    named("db_tx", |mut req: Request, next: Next| async move {
        let Some(DbHandle::Conn(conn)) = req.get::<DbHandle>().cloned() else {
            return Response::text("database connection missing for transaction").status(500);
        };
        let tx = match conn.begin().await {
            Ok(tx) => tx,
            Err(err) => return DbError(err).into_response_via_error(),
        };
        let arc = Arc::new(tx);
        req.set(DbHandle::Tx(Arc::clone(&arc)));
        let res = next(req).await;
        match Arc::try_unwrap(arc) {
            Ok(tx) => {
                if res.status_code().is_success() {
                    if let Err(err) = tx.commit().await {
                        tracing::error!(error = %err, "db commit failed");
                        return Response::text("Internal Server Error").status(500);
                    }
                } else if let Err(err) = tx.rollback().await {
                    tracing::error!(error = %err, "db rollback failed");
                }
            }
            Err(_) => tracing::error!("db transaction still held after request"),
        }
        res
    })
}

trait IntoResponseViaError {
    fn into_response_via_error(self) -> Response;
}

impl IntoResponseViaError for DbError {
    fn into_response_via_error(self) -> Response {
        ruvo_core::IntoResponse::into_response(self)
    }
}

/// Helper used by plugin install to inject Conn handle.
pub(crate) fn inject_conn(pool: DbPool) -> impl ruvo_core::extend::IntoMwEntry {
    named(
        "db",
        ruvo_core::with_state(pool, |pool, mut req, next| async move {
            match pool.get().await {
                Ok(conn) => {
                    req.set(DbHandle::Conn(conn));
                    next(req).await
                }
                Err(err) => ruvo_core::IntoResponse::into_response(err),
            }
        }),
    )
}
