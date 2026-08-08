//! Default tracing subscriber for `listen` / `run` / `serve`.
//!
//! Supports stdout and/or a rotating log file. Configure via env (`LogConfig::from_env`)
//! or build [`LogConfig`] in code / CLI.
//!
//! Optional [`set_log_event_hook`] lets plugins (e.g. Elasticsearch sink) observe events
//! without replacing the subscriber.

use crate::human::parse_bytes;
use file_rotate::{
    compression::Compression,
    suffix::{AppendCount, AppendTimestamp, FileLimit},
    ContentLimit, FileRotate, TimeFrequency,
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Keep non-blocking worker guards alive for the process lifetime.
static FILE_GUARDS: OnceLock<Mutex<Vec<WorkerGuard>>> = OnceLock::new();

fn retain_guard(guard: WorkerGuard) {
    FILE_GUARDS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(guard);
}

/// Structured log event for external sinks (Elasticsearch, …).
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

/// Callback invoked for every tracing event (after the local fmt layers).
pub type LogEventHook = Arc<dyn Fn(LogRecord) + Send + Sync>;

static LOG_EVENT_HOOK: OnceLock<LogEventHook> = OnceLock::new();

/// Register a global log sink hook (once). Used by `Observability::with_elasticsearch()`.
pub fn set_log_event_hook(hook: LogEventHook) -> Result<(), LogEventHook> {
    LOG_EVENT_HOOK.set(hook)
}

struct HookLayer;

impl<S> Layer<S> for HookLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let Some(hook) = LOG_EVENT_HOOK.get() else {
            return;
        };
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let meta = event.metadata();
        hook(LogRecord {
            level: meta.level().to_string(),
            target: meta.target().to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: visitor.fields,
        });
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let s = format!("{value:?}");
        // tracing often wraps Display as Debug with quotes — keep raw for message.
        if field.name() == "message" {
            let trimmed = s.trim_matches('"').to_string();
            self.message = Some(trimmed);
        } else {
            self.fields.push((field.name().to_string(), s));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

/// How to rotate the log file when [`LogConfig::file`] is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogRotate {
    /// Append forever (no rotation).
    Never,
    /// Rotate when the active file exceeds `max_bytes`; keep `keep` archived files.
    Size { max_bytes: usize, keep: usize },
    /// Rotate once per calendar day; keep `keep` archived files.
    Daily { keep: usize },
}

impl Default for LogRotate {
    fn default() -> Self {
        Self::Size {
            max_bytes: 10 * 1024 * 1024,
            keep: 5,
        }
    }
}

/// Tracing install options (stdout and/or file).
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// `EnvFilter` directive (e.g. `sova=info`, `debug`).
    pub filter: String,
    /// Write to stdout (default `true`).
    pub stdout: bool,
    /// Optional log file path.
    pub file: Option<PathBuf>,
    /// File rotation policy (used when `file` is set).
    pub rotate: LogRotate,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            filter: "sova=info".into(),
            stdout: true,
            file: None,
            rotate: LogRotate::default(),
        }
    }
}

impl LogConfig {
    /// Build from environment variables (see crate / README logging section).
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("RUST_LOG") {
            if !v.is_empty() {
                cfg.filter = v;
            }
        }
        cfg.stdout = env_truthy("SOVA_LOG_STDOUT", true);
        if let Ok(path) = std::env::var("SOVA_LOG_FILE") {
            if !path.is_empty() {
                cfg.file = Some(PathBuf::from(path));
            }
        }
        cfg.rotate = parse_rotate_from_env();
        cfg
    }

    /// Install the subscriber (`try_init`). No-op if `SOVA_LOG=off` or a subscriber already exists.
    pub fn install(&self) {
        if std::env::var_os("SOVA_LOG").is_some_and(|v| v == "off") {
            return;
        }
        let _ = self.try_install();
    }

    /// Like [`Self::install`], but returns whether init succeeded.
    pub fn try_install(&self) -> Result<(), String> {
        if !self.stdout && self.file.is_none() {
            return Err("LogConfig: enable stdout and/or set a log file".into());
        }

        let filter = EnvFilter::try_new(&self.filter)
            .or_else(|_| EnvFilter::try_new("sova=info"))
            .unwrap_or_else(|_| EnvFilter::new("info"));

        let stdout_layer = self.stdout.then(|| {
            fmt::layer()
                .with_writer(io::stdout)
                .with_target(false)
                .with_ansi(true)
        });

        let file_layer = if let Some(path) = &self.file {
            let writer = open_rotating_file(path, &self.rotate)
                .map_err(|e| format!("log file {}: {e}", path.display()))?;
            let (nb, guard) = tracing_appender::non_blocking(writer);
            retain_guard(guard);
            Some(
                fmt::layer()
                    .with_writer(nb)
                    .with_target(false)
                    .with_ansi(false),
            )
        } else {
            None
        };

        Registry::default()
            .with(filter)
            .with(stdout_layer)
            .with(file_layer)
            .with(HookLayer)
            .try_init()
            .map_err(|e| e.to_string())
    }
}

/// Install a default subscriber unless one is already set or `SOVA_LOG=off`.
pub fn ensure_tracing() {
    LogConfig::from_env().install();
}

fn env_truthy(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn parse_rotate_from_env() -> LogRotate {
    let keep = std::env::var("SOVA_LOG_ROTATE_KEEP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
        .max(1);

    let mode = std::env::var("SOVA_LOG_ROTATE")
        .unwrap_or_else(|_| "size".into())
        .to_ascii_lowercase();

    match mode.as_str() {
        "never" | "none" | "off" => LogRotate::Never,
        "daily" | "day" => LogRotate::Daily { keep },
        _ => {
            let max_bytes = std::env::var("SOVA_LOG_ROTATE_SIZE")
                .ok()
                .and_then(|s| parse_bytes(&s).ok())
                .unwrap_or(10 * 1024 * 1024)
                .max(1);
            LogRotate::Size { max_bytes, keep }
        }
    }
}

/// Parse rotate mode string (`size` / `daily` / `never`).
pub fn parse_log_rotate(
    mode: &str,
    size: Option<&str>,
    keep: Option<usize>,
) -> Result<LogRotate, String> {
    let keep = keep.unwrap_or(5).max(1);
    match mode.trim().to_ascii_lowercase().as_str() {
        "never" | "none" | "off" => Ok(LogRotate::Never),
        "daily" | "day" => Ok(LogRotate::Daily { keep }),
        "size" | "" => {
            let max_bytes = match size {
                Some(s) => parse_bytes(s)?,
                None => 10 * 1024 * 1024,
            }
            .max(1);
            Ok(LogRotate::Size { max_bytes, keep })
        }
        other => Err(format!("unknown log rotate mode: {other}")),
    }
}

enum RotatingWriter {
    Count(FileRotate<AppendCount>),
    Stamp(FileRotate<AppendTimestamp>),
}

impl Write for RotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Count(w) => w.write(buf),
            Self::Stamp(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Count(w) => w.flush(),
            Self::Stamp(w) => w.flush(),
        }
    }
}

fn open_rotating_file(path: &Path, rotate: &LogRotate) -> io::Result<RotatingWriter> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    Ok(match rotate {
        LogRotate::Never => RotatingWriter::Count(FileRotate::new(
            path,
            AppendCount::new(0),
            ContentLimit::None,
            Compression::None,
            None,
        )),
        LogRotate::Size { max_bytes, keep } => RotatingWriter::Count(FileRotate::new(
            path,
            AppendCount::new(*keep),
            ContentLimit::BytesSurpassed(*max_bytes),
            Compression::None,
            None,
        )),
        LogRotate::Daily { keep } => RotatingWriter::Stamp(FileRotate::new(
            path,
            AppendTimestamp::default(FileLimit::MaxFiles(*keep)),
            ContentLimit::Time(TimeFrequency::Daily),
            Compression::None,
            None,
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rotate_modes() {
        assert_eq!(
            parse_log_rotate("never", None, Some(3)).unwrap(),
            LogRotate::Never
        );
        assert_eq!(
            parse_log_rotate("daily", None, Some(7)).unwrap(),
            LogRotate::Daily { keep: 7 }
        );
        let s = parse_log_rotate("size", Some("2MB"), Some(3)).unwrap();
        assert_eq!(
            s,
            LogRotate::Size {
                max_bytes: 2 * 1024 * 1024,
                keep: 3
            }
        );
    }

    #[test]
    fn from_env_defaults() {
        let cfg = LogConfig::default();
        assert!(cfg.stdout);
        assert!(cfg.file.is_none());
        assert_eq!(
            cfg.rotate,
            LogRotate::Size {
                max_bytes: 10 * 1024 * 1024,
                keep: 5
            }
        );
    }

    #[test]
    fn open_size_rotate_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        let mut w = open_rotating_file(
            &path,
            &LogRotate::Size {
                max_bytes: 32,
                keep: 2,
            },
        )
        .unwrap();
        writeln!(w, "hello logging").unwrap();
        w.flush().unwrap();
        assert!(path.exists());
    }

    #[test]
    fn hook_layer_receives_events() {
        use std::sync::Mutex;
        use tracing_subscriber::prelude::*;

        let got = Arc::new(Mutex::new(Vec::<LogRecord>::new()));
        let got2 = Arc::clone(&got);
        let _ = set_log_event_hook(Arc::new(move |r| {
            got2.lock().unwrap().push(r);
        }));

        let _guard = tracing::subscriber::set_default(
            Registry::default().with(HookLayer).with(
                EnvFilter::new("info"),
            ),
        );
        tracing::info!(request_id = "abc", "hello es");
        let records = got.lock().unwrap();
        assert!(!records.is_empty());
        assert!(records.iter().any(|r| r.message.contains("hello es") || r.fields.iter().any(|(k,_)| k == "request_id")));
    }
}
