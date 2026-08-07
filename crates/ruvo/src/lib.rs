//! Ruvo — Express-like HTTP framework for Rust.
//!
//! Thin facade over `ruvo-core` plus optional plugin crates.
//! Prefer [`prelude`] in application `main`; plugin authors use [`extend`].

mod app;
mod error;

pub use app::{App, BoundApp};
pub use error::{AppError, Result};
pub use ruvo_core::{
    logger, with_state, BackgroundService, ClientAddr, ConfigDoc, Error, Html, Http, IntoResponse,
    Json, NoContent, Next, OnUpgrade, Plugin, Redirect, Request, Response, Router, Server, Shutdown,
    TestClient, Text,
};
#[cfg(feature = "tls")]
pub use ruvo_core::Tls;

/// Everyday imports for application code.
pub mod prelude {
    pub use crate::{
        logger, App, Error, Html, IntoResponse, Json, Next, NoContent, Plugin, Redirect, Request,
        Response, Result, Router, Text,
    };
}

/// Extension API (handlers, bodies, [`Bind`](extend::Bind), …) — see `ruvo_core::extend`.
pub mod extend {
    pub use ruvo_core::extend::*;
}

#[cfg(feature = "env")]
pub use ruvo_env::{self, require as env_require, EnvError};

#[cfg(feature = "cors")]
pub use ruvo_cors::Cors;
#[cfg(feature = "shield")]
pub use ruvo_shield::Shield;

#[cfg(feature = "cookies")]
pub use ruvo_cookies::{CookieBuilder, CookieLayer, CookieLayerPresent, Cookies, ResponseCookieExt};

#[cfg(feature = "static-files")]
pub use ruvo_static::Static;

#[cfg(feature = "compress")]
pub use ruvo_compress::Compress;

#[cfg(feature = "rate-limit")]
pub use ruvo_rate_limit::RateLimit;

#[cfg(feature = "session")]
pub use ruvo_session::{memory_sessions, Session, SessionExt, SessionLayer};

/// Key-value store backends (`Memory`, `File`, `Postgres`, `Sqlite`).
#[cfg(feature = "store")]
pub mod store {
    pub use ruvo_store::{namespace, KvStore, MemoryStore as Memory, Namespace};

    #[cfg(feature = "store-file")]
    pub use ruvo_store_file::{Durability, FileStore as File};

    #[cfg(feature = "store-postgres")]
    pub use ruvo_store_postgres::PostgresStore as Postgres;

    #[cfg(feature = "store-sqlite")]
    pub use ruvo_store_sqlite::SqliteStore as Sqlite;

    #[cfg(feature = "store-crypto")]
    pub use ruvo_store::{encrypted, encrypted_ns, AppKey, Encrypted};
}

#[cfg(feature = "store")]
pub use store::{namespace, KvStore, Namespace};

/// Task queue backends (`Memory`, `File`, `Postgres`, `Sqlite`).
#[cfg(feature = "tasks-store")]
pub mod tasks {
    pub use ruvo_tasks_store::{
        EnqueueOpts, MemoryStore as Memory, Task, TaskError, TaskStatus, TaskStore,
    };

    #[cfg(feature = "tasks-file")]
    pub use ruvo_tasks_file::FileTaskStore as File;

    #[cfg(feature = "tasks-postgres")]
    pub use ruvo_tasks_postgres::PostgresTaskStore as Postgres;

    #[cfg(feature = "tasks-sqlite")]
    pub use ruvo_tasks_sqlite::SqliteTaskStore as Sqlite;

    #[cfg(feature = "tasks")]
    pub use ruvo_tasks::{bearer_guard, HttpTaskError, TaskBackend, Tasks};
}

#[cfg(feature = "tasks-store")]
pub use tasks::{EnqueueOpts, Task, TaskError, TaskStatus, TaskStore};

#[cfg(feature = "tasks")]
pub use tasks::{bearer_guard, HttpTaskError, TaskBackend, Tasks};

#[cfg(feature = "db")]
pub use ruvo_db::{
    test_db, transaction, ActiveModelTrait, Db, DbError, DbExt, DbHandle, EntityTrait, Set,
    TestDb,
};

#[cfg(feature = "udp")]
pub use ruvo_udp::UdpService;

#[cfg(feature = "quic-udp")]
pub use ruvo_quic::{Http3Service, QuicDatagramClient, QuicDatagramService};

#[cfg(feature = "sse-feed")]
pub use ruvo_sse::{sse_response, SseChannel, SseEvent};

#[cfg(feature = "templates")]
pub use ruvo_templates::{MiniJinjaEngine, MiniJinjaTemplates, RenderExt, Templates, TemplateEngine};

#[cfg(feature = "multipart")]
pub use ruvo_multipart::{MultipartExt, MultipartField};

#[cfg(feature = "cli")]
pub use ruvo_cli::{Parser, ServerArgs};

#[cfg(feature = "vld")]
pub use ruvo_vld::{
    ValidExt, Validate, ValidateHook, ValidateRouteExt, ValidateSource, Validated, ValidationError,
    ValidationExt, Vld,
};

#[cfg(feature = "vld")]
pub use vld;

#[cfg(feature = "openapi")]
pub use ruvo_openapi::{undocumented, Doc, OpenApi, OpenApiDocExt, OpenApiValidate};

#[cfg(feature = "i18n")]
pub use ruvo_i18n::{
    localize_path, localized_url, mount_localized, strip_locale_prefix, template_fn, I18n, I18nExt,
    I18nRouteExt, I18nScope, Locale, PrefixMode, ROOT_SCOPE,
};

#[cfg(feature = "ws")]
pub use ruvo_ws::{
    origin_allowed, upgrade_ws, Hub, Message, RoomHandle, Ws, WsRouteExt, WsSession,
};

#[cfg(feature = "vld-openapi")]
pub use ruvo_vld::{doc_schema, DocVldExt, VldDocSchema};

#[cfg(feature = "vld-flash-templates")]
pub use ruvo_vld::with_validation_flash;

#[cfg(feature = "http-client")]
pub use ruvo_http::{
    FakeTransport, Http as OutboundHttp, HttpBound, HttpClient, HttpError, HttpExt, NamedClient,
    OutRequest, OutResponse, PendingRequest, RequestId, StubBody,
};

#[cfg(feature = "meta")]
pub use ruvo_meta::{
    absolute_url, render_html, resolve_meta, strip_tracking, Article, BreadcrumbList, ChangeFreq,
    Entry, FAQPage, Meta, MetaDefaults, MetaExt, MetaOverlay, MetaPage, Organization, Product,
    ResolvedMeta, ToJsonLd, TrailingSlash, WebSite,
};

#[cfg(feature = "meta")]
pub mod schema {
    pub use ruvo_meta::schema::*;
}

#[cfg(feature = "meta-templates")]
pub use ruvo_meta::with_meta;

/// Install a default `tracing` subscriber (`RUST_LOG`, default `ruvo=info`).
///
/// Usually unnecessary: [`App::listen`] / [`BoundApp::serve`] call this via `try_init`.
/// Set `RUVO_LOG=off` to skip. With the `cli` feature, `ServerArgs::init_tracing` still
/// applies `--log-level`.
pub fn init_tracing() {
    ruvo_core::extend::ensure_tracing();
}
