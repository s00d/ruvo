//! Sova — Express-like HTTP framework for Rust.
//!
//! Thin facade over `sova-core` plus optional plugin crates.
//! Prefer [`prelude`] in application `main`; plugin authors use [`extend`].

mod app;
mod doc_features;
mod error;
#[cfg(any(feature = "web", feature = "api"))]
mod preset;

pub use app::{App, BoundApp};
pub use error::{AppError, Result};
#[cfg(feature = "web")]
pub use preset::WebApp;
#[cfg(feature = "api")]
pub use preset::ApiApp;
pub use sova_core::{
    ensure_request_id, logger, logger_skip_path, logger_skip_paths, request_id, with_state,
    BackgroundService, Cell, CheckKind, CheckResult, ClientAddr, ConfigDoc, Error, FormData, Html,
    Http, IntoResponse, Json, LogConfig, LogRotate, MatchedRoute, MatchedRouteCapture, NoContent,
    Next, OnUpgrade, Plugin, PluginMeta, PluginSdkVersion, RateLimitIdentity, Redirect, Request,
    RequestId, Response, Router, Server, Shutdown, Slot, Text, Upload, UploadRules,
    PLUGIN_SDK_VERSION, referer_or,
};
#[cfg(feature = "testing")]
pub use sova_core::{ResponseAssert, TestClient};

#[cfg(feature = "tls")]
pub use sova_core::Tls;

/// Everyday imports for application code.
///
/// Status codes: `Response::json(&x).status(201)` or `(201, Json(x))`.
/// Forms/files: `Request::input` / `form` (feature `multipart` for multipart bodies).
pub mod prelude {
    pub use crate::{
        logger, request_id, App, Error, Html, IntoResponse, Json, Next, NoContent, Plugin, Redirect,
        Request, RequestId, Response, Result, Router, Text,
    };
    #[cfg(feature = "web")]
    pub use crate::WebApp;
    #[cfg(feature = "api")]
    pub use crate::ApiApp;
}

/// Extension API (handlers, bodies, [`Bind`](extend::Bind), …) — see `sova_core::extend`.
pub mod extend {
    pub use sova_core::extend::*;
}

#[cfg(feature = "devtools")]
pub use sova_devtools::{DevTools, DevToolsBag, DevToolsHub};

#[cfg(feature = "env")]
pub use sova_env::{self, require as env_require, EnvError};

#[cfg(feature = "cors")]
pub use sova_cors::Cors;
#[cfg(feature = "csrf")]
pub use sova_csrf::{Csrf, CsrfExt, CsrfToken};
#[cfg(feature = "shield")]
pub use sova_shield::Shield;

#[cfg(feature = "cookies")]
pub use sova_cookies::{CookieBuilder, CookieLayer, CookieLayerPresent, Cookies, ResponseCookieExt};

#[cfg(feature = "static-files")]
pub use sova_static::Static;

#[cfg(feature = "compress")]
pub use sova_compress::Compress;

#[cfg(feature = "rate-limit")]
pub use sova_rate_limit::{RateLimit, RateLimitKey};

#[cfg(feature = "session")]
pub use sova_session::{
    memory_sessions, KvSessionStore, SameSite, Session, SessionExt, SessionLayer,
    SessionStore, SessionStoreHandle, FLASH_ERRORS, FLASH_OLD, FLASH_STATUS, SESSION_USER_KEY,
};

#[cfg(feature = "session-sql")]
pub use sova_session::SqlSessionStore;

#[cfg(feature = "session-redis")]
pub use sova_session::RedisSessionStore;

#[cfg(feature = "store")]
mod shared_store;

/// Key-value store backends (`Memory`, `File`, `Sql`, `Redis`).
#[cfg(feature = "store")]
pub mod store {
    pub use sova_store::{namespace, AppStore, Cache, CacheError, KvStore, MemoryStore as Memory, Namespace};
    pub use crate::shared_store::SharedStore;

    #[cfg(feature = "store-file")]
    pub use sova_store::{Durability, FileStore as File};

    #[cfg(feature = "store-sql")]
    pub use sova_store::SqlStore as Sql;

    #[cfg(feature = "store-redis")]
    pub use sova_store::RedisStore as Redis;

    #[cfg(feature = "store-crypto")]
    pub use sova_store::{encrypted, encrypted_ns, AppKey, Encrypted};
}

#[cfg(feature = "store")]
pub use store::{namespace, AppStore, Cache, CacheError, KvStore, Namespace, SharedStore};

/// Task queue backends (`Memory`, `File`, `Sql`, `Redis`) + `Job` / `Dispatch` / scheduler.
#[cfg(feature = "tasks-store")]
pub mod tasks {
    pub use sova_tasks_store::{
        EnqueueOpts, MemoryStore as Memory, Task, TaskError, TaskStatus, TaskStore,
    };

    #[cfg(feature = "tasks-file")]
    pub use sova_tasks_store::FileTaskStore as File;

    #[cfg(feature = "tasks-sql")]
    pub use sova_tasks_store::SqlTaskStore as Sql;

    #[cfg(feature = "tasks-redis")]
    pub use sova_tasks_store::RedisTaskStore as Redis;

    #[cfg(feature = "tasks")]
    pub use sova_tasks::{
        ask, bearer_guard, confirm, enter_cli, error as console_error, info, is_interactive, line,
        priority, table, warn as console_warn, ConsoleGuard, Dispatch, HttpTaskError, Job, JobInfo,
        Schedule, TaskBackend, TaskRegistry, Tasks,
    };
}

#[cfg(feature = "tasks-store")]
pub use tasks::{EnqueueOpts, Task, TaskError, TaskStatus, TaskStore};

#[cfg(feature = "tasks")]
pub use tasks::{
    ask, bearer_guard, confirm, console_error, console_warn, enter_cli, info, is_interactive, line,
    priority, table, ConsoleGuard, Dispatch, HttpTaskError, Job, JobInfo, Schedule, TaskBackend,
    TaskRegistry, Tasks,
};

#[cfg(feature = "db")]
pub use sova_db::{
    parse_migrate_args, test_db, transaction, ActiveModelTrait, ColumnTrait, Db, DbError, DbExt,
    DbHandle, DbPool, EntityTrait, MigrateCmd, MigrationTrait, MigratorTrait, Page, PageExt,
    PageParams, PaginateExt, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TestDb,
};

#[cfg(feature = "redis")]
pub use sova_redis::{
    Redis, RedisError, RedisExt, RedisMessage, RedisPool, RedisSubscriber,
};

#[cfg(feature = "observability")]
pub use sova_observability::Observability;

#[cfg(feature = "observability-elasticsearch")]
pub use sova_observability::ElasticsearchLog;

#[cfg(feature = "udp")]
pub use sova_udp::UdpService;

#[cfg(feature = "quic-udp")]
pub use sova_quic::{Http3Service, QuicDatagramClient, QuicDatagramService};

#[cfg(feature = "sse-feed")]
pub use sova_sse::{sse_response, SseChannel, SseEvent};

#[cfg(feature = "templates")]
pub use sova_templates::{
    register_per_request, FrozenAmbient, MiniJinjaEngine, MiniJinjaTemplates, RenderExt,
    TemplateEngine,
    TemplateHelpers, Templates,
};



#[cfg(feature = "cli")]
pub use sovax::{Parser, ServerArgs};

#[cfg(feature = "vld")]
pub use sova_vld::{
    ValidExt, Validate, ValidateHook, ValidateRouteExt, ValidateSource, Validated, ValidationError,
    ValidationExt, Vld,
};

#[cfg(feature = "vld")]
pub use vld;

#[cfg(feature = "openapi")]
pub use sova_openapi::{undocumented, Doc, OpenApi, OpenApiDocExt, OpenApiValidate};

#[cfg(feature = "i18n")]
pub use sova_i18n::{
    localize_path, localized_url, mount_localized, strip_locale_prefix, template_fn, I18n, I18nExt,
    I18nRouteExt, I18nScope, Locale, PrefixMode, ROOT_SCOPE,
};

#[cfg(feature = "ws")]
pub use sova_ws::{
    origin_allowed, upgrade_ws, Hub, Message, RoomHandle, Ws, WsRouteExt, WsSession,
};

#[cfg(feature = "vld-openapi")]
pub use sova_vld::{doc_schema, DocVldExt, VldDocSchema};

#[cfg(feature = "vld-flash-templates")]
pub use sova_vld::{with_flash, with_validation_flash};

#[cfg(feature = "http-client")]
pub use sova_http::{
    FakeTransport, Http as OutboundHttp, HttpBound, HttpClient, HttpError, HttpExt, NamedClient,
    OutRequest, OutResponse, PendingRequest, StubBody,
};

#[cfg(feature = "mail")]
pub use sova_mail::{
    Content, Email, EmailSnapshot, Envelope, FakeMail, Mail, MailClient, MailExt, Mailable,
    SmtpBuilder,
};

#[cfg(feature = "ai")]
pub use sova_ai::{Ai, AiBound, AiClient, AiError, AiExt, FakeAi, SharedModel};

/// AISDK re-exports and agent helpers (`LanguageModelRequest`, `tool!`, …).
#[cfg(feature = "ai")]
pub mod ai {
    pub use sova_ai::prelude::*;
    pub use sova_ai::aisdk;
}

#[cfg(feature = "storage")]
pub use sova_storage::{
    AppStorage, BlobStore, LocalStore, MemoryStore, PutOpts, Storage, StorageError, StorageExt,
    StoredFile,
};

#[cfg(feature = "passport")]
pub use sova_passport::{
    local_strategy, passport_serialize, Auth, AuthMw, Authenticated, Credentials, Extract,
    Passport, PassportExt, Source,
};

#[cfg(feature = "passport-jwt")]
pub use sova_passport::{
    hash_password, hash_refresh_token, hash_token, issue_token_pair, token_can, verify_password,
    ApiTokenInfo, ApiTokenRow, AuthUser, Claims, CreateApiToken, CreatedApiToken, Jwt, JwtAuth,
    JwtAuthExt, JwtAuthState, JwtError, TokenPair, PAT_PREFIX,
};

#[cfg(all(feature = "passport-jwt", not(feature = "auth")))]
pub use sova_passport::AuthMigrator;

#[cfg(feature = "passport-oauth")]
pub use sova_passport::{
    oauth_drivers, Apple, Custom, Driver, Github, Google, Oauth, OauthProfile, OauthProvider,
    OauthTokens, ProfileKind,
};

#[cfg(feature = "auth")]
pub use sova_auth::{
    assign_role, create_permission, create_role, delete_permission, delete_role, find_user_by_email,
    find_user_by_id, list_permissions, list_roles, load_current_user, make_verify_token,
    mark_email_verified, parse_verify_token, register_user, revoke_role, set_avatar, set_user_roles,
    sync_role_permissions, update_permission, update_role, user_ids_with_permission,
    user_ids_with_role, AuthExt, AuthMigrator, CurrentUser, Feature as AuthFeature, Fortify,
    FortifyPaths,
};

#[cfg(feature = "auth-mail")]
pub use sova_auth::{send_reset, send_verify, ResetPasswordMail, VerifyEmailMail};

#[cfg(feature = "activity")]
pub use sova_activity::{
    list_activity, Activity, ActivityActor, ActivityEntry, ActivityExt, ActivityFilter,
    ActivityLog, ActivityMigrator, ActivityRow,
};

#[cfg(feature = "notifications")]
pub use sova_notifications::{
    list_notifications, mark_all_read, mark_read, unread_count, Channel, NotificationFilter,
    NotificationRow, NotificationService, NotificationUser, Notifications, NotificationsMigrator,
    Notify, NotifyExt, Via,
};

#[cfg(feature = "notifications-templates")]
pub use sova_notifications::{preload_unread, UnreadCount};

#[cfg(all(feature = "auth", feature = "auth-vld"))]
pub use sova_auth::{
    ConfirmPasswordForm, DisableTwoFactorForm, ForgotForm, LoginForm, PasswordForm, ProfileForm,
    RegisterForm, ResetForm, TwoFactorCodeForm,
};

#[cfg(feature = "meta")]
pub use sova_meta::{
    absolute_url, render_html, resolve_meta, strip_tracking, Article, BreadcrumbList, ChangeFreq,
    Entry, FAQPage, Meta, MetaDefaults, MetaExt, MetaOverlay, MetaPage, Organization, Product,
    ResolvedMeta, Robots, Sitemap, ToJsonLd, TrailingSlash, WebSite,
};

#[cfg(feature = "meta")]
pub mod schema {
    pub use sova_meta::schema::*;
}

#[cfg(feature = "meta-templates")]
pub use sova_meta::with_meta;

/// Install a default `tracing` subscriber (`LogConfig::from_env`).
///
/// Usually unnecessary: [`App::listen`] / [`BoundApp::serve`] call this via `try_init`.
/// Set `SOVA_LOG=off` to skip. With the `cli` feature, `ServerArgs::init_tracing` applies
/// `--log-level` / `--log-file` / rotation flags.
pub fn init_tracing() {
    sova_core::extend::ensure_tracing();
}
