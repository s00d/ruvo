use crate::fake::FakeRedis;
use crate::pool::RedisPool;
use sova_core::{App, Error, Plugin};

/// Redis / Valkey pool plugin (`REDIS_URL`).
pub struct Redis {
    mode: Mode,
}

enum Mode {
    Url(String),
    Fake(FakeRedis),
}

impl Redis {
    pub fn from_env() -> Self {
        let url = std::env::var("REDIS_URL").unwrap_or_default();
        Self {
            mode: Mode::Url(url),
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.mode = Mode::Url(url.into());
        self
    }

    /// In-memory Redis for tests and demos (no TCP).
    pub fn fake(fake: FakeRedis) -> Self {
        Self {
            mode: Mode::Fake(fake),
        }
    }
}

impl Plugin for Redis {
    fn id(&self) -> &'static str {
        "redis"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Redis")
            .description(
                "Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, list queues",
            )
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        if let Mode::Fake(fake) = self.mode {
            let pool = RedisPool::new();
            pool.set_fake(fake);
            app.state(pool);
            return;
        }

        // Env wins, then builder `.url()`, then `[redis] url` in toml.
        if let Mode::Url(url) = &mut self.mode {
            if let Ok(u) = std::env::var("REDIS_URL") {
                if !u.is_empty() {
                    *url = u;
                }
            }
            if url.is_empty() {
                if let Some(u) = app
                    .config_doc()
                    .and_then(|d| d.section("redis"))
                    .and_then(|s| s.get("url").and_then(|v| v.as_str()).map(str::to_string))
                {
                    *url = u;
                }
            }
        }

        let Mode::Url(url) = self.mode else {
            return;
        };

        if url.is_empty() {
            app.on_startup(|_state| async {
                Err(Error::Internal(
                    "redis url is empty; set REDIS_URL or [redis] url in sova.toml".into(),
                ))
            });
            return;
        }

        let pool = RedisPool::new();
        app.state(pool.clone());

        let url_start = url.clone();
        let pool_start = pool.clone();
        app.on_startup(move |_state| {
            let url = url_start.clone();
            let pool = pool_start.clone();
            async move {
                pool.set_url(url.clone());
                let conn = RedisPool::connect(&url)
                    .await
                    .map_err(|e| Error::Internal(format!("redis connect: {e}")))?;
                pool.set(conn);
                Ok(())
            }
        });

        let pool_stop = pool.clone();
        app.on_shutdown(move || {
            let pool = pool_stop.clone();
            async move {
                pool.clear();
            }
        });

        let pool_check = pool.clone();
        app.register_check("redis", move |_state| {
            let pool = pool_check.clone();
            async move {
                let mut conn = pool.get().map_err(Error::from)?;
                let _: String = redis::cmd("PING")
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| Error::Internal(format!("redis ping: {e}")))?;
                Ok(())
            }
        });
    }
}
