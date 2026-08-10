//! DevTools console configuration and shared session state.

#[cfg(feature = "console-redis")]
use std::sync::atomic::{AtomicU8, Ordering};

pub const DEFAULT_BODY_LIMIT: usize = 1024 * 1024;

/// Console options installed on the app (feature `console`).
pub struct DevToolsConsole {
    pub enabled: bool,
    pub allow_dangerous: bool,
    pub console_external: bool,
    pub body_limit: usize,
    #[cfg(feature = "console-redis")]
    redis_db: AtomicU8,
}

impl Clone for DevToolsConsole {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            allow_dangerous: self.allow_dangerous,
            console_external: self.console_external,
            body_limit: self.body_limit,
            #[cfg(feature = "console-redis")]
            redis_db: AtomicU8::new(self.redis_db.load(Ordering::Relaxed)),
        }
    }
}

impl DevToolsConsole {
    pub fn new(
        enabled: bool,
        allow_dangerous: bool,
        console_external: bool,
        body_limit: usize,
    ) -> Self {
        Self {
            enabled,
            allow_dangerous,
            console_external,
            body_limit,
            #[cfg(feature = "console-redis")]
            redis_db: AtomicU8::new(0),
        }
    }

    #[cfg(feature = "console-redis")]
    pub fn redis_db(&self) -> u8 {
        self.redis_db.load(Ordering::Relaxed)
    }

    #[cfg(feature = "console-redis")]
    pub fn set_redis_db(&self, db: u8) {
        self.redis_db.store(db.min(15), Ordering::Relaxed);
    }
}
