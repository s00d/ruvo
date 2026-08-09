use crate::pool::RedisPool;
use sova_core::{App, Error, Plugin};

/// Redis / Valkey pool plugin (`REDIS_URL`).
pub struct Redis {
    url: String,
}

impl Redis {
    pub fn from_env() -> Self {
        let url = std::env::var("REDIS_URL").unwrap_or_default();
        Self { url }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }
}

impl Plugin for Redis {
    fn id(&self) -> &'static str {
        "redis"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Redis")
            .description("Shared Redis/Valkey connection for KvStore, tasks, cache, pub/sub, queues")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(mut self, app: &mut App) {
        // Env wins, then builder `.url()`, then `[redis] url` in toml.
        if let Ok(u) = std::env::var("REDIS_URL") {
            if !u.is_empty() {
                self.url = u;
            }
        }
        if self.url.is_empty() {
            if let Some(u) = app
                .config_doc()
                .and_then(|d| d.section("redis"))
                .and_then(|s| s.get("url").and_then(|v| v.as_str()).map(str::to_string))
            {
                self.url = u;
            }
        }

        if self.url.is_empty() {
            app.on_startup(|_state| async {
                Err(Error::Internal(
                    "redis url is empty; set REDIS_URL or [redis] url in sova.toml".into(),
                ))
            });
            return;
        }

        let pool = RedisPool::new();
        app.state(pool.clone());

        let url = self.url.clone();
        let pool_start = pool.clone();
        app.on_startup(move |_state| {
            let url = url.clone();
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
