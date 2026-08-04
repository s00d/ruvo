//! Process-local background services (UDP listeners, task workers, …).

use crate::handler::BoxFuture;
use crate::state::StateMap;
use std::sync::Arc;
use tokio::sync::watch;

/// Unified shutdown signal for [`BackgroundService`]s.
///
/// This is a thin wrapper around an internal `tokio::sync::watch` receiver,
/// but the tokio type does not leak into the plugin contract.
#[derive(Clone, Debug)]
pub struct Shutdown {
    inner: watch::Receiver<bool>,
}

impl Shutdown {
    pub(crate) fn new(inner: watch::Receiver<bool>) -> Self {
        Self { inner }
    }

    /// Whether shutdown was already triggered.
    pub fn is_triggered(&self) -> bool {
        *self.inner.borrow()
    }

    /// Wait until shutdown is triggered.
    pub async fn recv(&mut self) {
        loop {
            if *self.inner.borrow() {
                return;
            }
            if self.inner.changed().await.is_err() {
                return;
            }
        }
    }
}

/// Test-only helper for triggering [`Shutdown`] signals without exposing tokio types.
#[cfg(any(test, feature = "testing"))]
#[derive(Clone, Debug)]
pub struct ShutdownSender(watch::Sender<bool>);

#[cfg(any(test, feature = "testing"))]
#[cfg_attr(docsrs, doc(cfg(feature = "testing")))]
#[allow(dead_code)]
#[must_use]
pub fn shutdown_channel() -> (ShutdownSender, Shutdown) {
    let (tx, rx) = watch::channel(false);
    (ShutdownSender(tx), Shutdown::new(rx))
}

#[cfg(any(test, feature = "testing"))]
impl ShutdownSender {
    pub fn send(&self, value: bool) -> bool {
        self.0.send(value).is_ok()
    }
}

/// Long-running work started after `on_startup`, stopped after connection drain.
///
/// Services are **process-local** — they are not shared across processes.
/// Prefer one service per concern (UDP socket, queue worker, …).
pub trait BackgroundService: Send {
    fn name(&self) -> &str;

    /// Run until `shutdown` becomes `true` (or the future completes).
    fn run(
        self: Box<Self>,
        state: Arc<StateMap>,
        shutdown: Shutdown,
    ) -> BoxFuture<()>;
}

/// Type-erased service stored on [`crate::App`].
pub(crate) type BoxedService = Box<dyn BackgroundService>;

/// Wait until shutdown is triggered.
pub async fn wait_shutdown(mut shutdown: Shutdown) {
    shutdown.recv().await
}
