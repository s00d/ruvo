use crate::app::{AppInner, ShutdownHook, StartupHook};
use crate::error::Result;
use crate::service::BoxedService;
use tokio::net::TcpListener;

use super::ExternalShutdown;

mod accept;
mod convert;
mod serve;
#[cfg(feature = "tls")]
mod tls;

#[cfg(not(feature = "tls"))]
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_tcp(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: TcpListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    accept::run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        accept::AcceptKind::Tcp(listener),
    )
    .await
}

#[cfg(feature = "tls")]
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_tcp(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: TcpListener,
    external_shutdown: Option<ExternalShutdown>,
    tls: Option<crate::tls::TlsRuntime>,
) -> Result<()> {
    accept::run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        accept::AcceptKind::Tcp(listener),
        tls,
    )
    .await
}

#[cfg(all(unix, not(feature = "tls")))]
pub(super) async fn run_unix(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: tokio::net::UnixListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    accept::run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        accept::AcceptKind::Unix(listener),
    )
    .await
}

#[cfg(all(unix, feature = "tls"))]
pub(super) async fn run_unix(
    inner: AppInner,
    startups: Vec<StartupHook>,
    shutdowns: Vec<ShutdownHook>,
    services: Vec<BoxedService>,
    start_services: bool,
    listener: tokio::net::UnixListener,
    external_shutdown: Option<ExternalShutdown>,
) -> Result<()> {
    accept::run_accept_loop(
        inner,
        startups,
        shutdowns,
        services,
        start_services,
        external_shutdown,
        accept::AcceptKind::Unix(listener),
        None,
    )
    .await
}
