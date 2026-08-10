//! HTTP/1 connection upgrade (WebSocket handshake, …).

use crate::response::Response;
use hyper::upgrade::{OnUpgrade as HyperOnUpgrade, Upgraded};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Cap for concurrent upgraded connections (`App::max_upgraded_connections`).
#[derive(Clone)]
pub(crate) struct UpgradeBudget(pub Arc<Semaphore>);

/// Pending upgrade extracted from the Hyper request (before `into_body`).
pub(crate) struct PendingUpgrade {
    pub(crate) on_upgrade: HyperOnUpgrade,
    pub(crate) budget: UpgradeBudget,
}

/// Holds one slot in `max_upgraded_connections` until dropped.
#[allow(dead_code)]
pub struct UpgradePermit(OwnedSemaphorePermit);

/// Handle to finish an HTTP upgrade (e.g. WebSocket).
///
/// Prefer ordinary routes + [`crate::Request::on_upgrade`] over [`crate::Router::raw`].
pub struct OnUpgrade {
    inner: HyperOnUpgrade,
    permit: OwnedSemaphorePermit,
}

impl OnUpgrade {
    /// Complete the protocol upgrade.
    ///
    /// Keep the returned [`UpgradePermit`] (or the whole tuple) alive for the
    /// lifetime of the upgraded connection so the budget slot stays reserved.
    pub async fn upgrade(self) -> Result<(Upgraded, UpgradePermit), hyper::Error> {
        let io = self.inner.await?;
        Ok((io, UpgradePermit(self.permit)))
    }
}

/// Try to take the upgrade; on budget exhaustion returns **503** + `Retry-After`.
pub(crate) fn take_upgrade(pending: PendingUpgrade) -> Result<OnUpgrade, Box<Response>> {
    let permit = match pending.budget.0.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return Err(Box::new(
                Response::text("Service Unavailable")
                    .status(503)
                    .header("retry-after", "5"),
            ));
        }
    };
    Ok(OnUpgrade {
        inner: pending.on_upgrade,
        permit,
    })
}
