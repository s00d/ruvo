//! Redis shared pool: sessions, tasks, cache, pub/sub, list queues.
//!
//! ```bash
//! export REDIS_URL=redis://127.0.0.1/
//! cargo run -p redis_demo
//! # POST /publish  {"channel":"demo","message":"hi"}
//! # POST /enqueue  {"queue":"jobs","message":"work"}
//! # GET  /dequeue?queue=jobs
//! ```

use sova::prelude::*;
use sova::{
    bearer_guard, store, tasks, AppStore, Cache, Job, Redis, RedisExt, RedisPool,
    RedisSessionStore, SessionLayer, SharedStore, Tasks,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct CachedPing {
    n: u64,
}

#[derive(Deserialize)]
struct PublishBody {
    channel: String,
    message: String,
}

#[derive(Deserialize)]
struct EnqueueBody {
    queue: String,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = sova::sova_env::load();

    let mut app = App::new();
    app.install(Redis::from_env());
    let pool = app
        .try_state::<RedisPool>()
        .expect("Redis plugin inserts RedisPool")
        .as_ref()
        .clone();

    let kv = Arc::new(store::Redis::from_redis_pool(&pool));
    let task_store = Arc::new(tasks::Redis::from_redis_pool(&pool));

    app.install(SharedStore::new(Arc::clone(&kv) as Arc<dyn sova::KvStore>));
    app.install(SessionLayer::from_store(Arc::new(
        RedisSessionStore::from_redis_pool(&pool),
    )));
    app.install(
        Tasks::new(task_store)
            .job(Job::new("ping", |_task| async move {
                tracing::info!("redis task ping");
                Ok(())
            }))
            .exposed()
            .guard(bearer_guard("secret")),
    );

    // Demo subscriber: log messages on channel `demo`.
    let sub_pool = pool.clone();
    app.on_startup(move |_state| {
        let pool = sub_pool.clone();
        async move {
            tokio::spawn(async move {
                let Ok(mut sub) = pool.subscribe(["demo"]).await else {
                    tracing::warn!("pubsub subscribe demo failed");
                    return;
                };
                tracing::info!("subscribed to channel demo");
                while let Some(msg) = sub.next().await {
                    tracing::info!(
                        channel = %msg.channel,
                        payload = ?msg.payload_str(),
                        "pubsub message"
                    );
                }
            });
            Ok(())
        }
    });

    app.get("/", || async {
        "ok — GET /cache, POST /publish, POST /enqueue, GET /dequeue, POST /_tasks/enqueue"
    });

    app.get("/cache", |req: Request| async move {
        let store = req
            .try_state::<AppStore>()
            .ok_or_else(|| Error::Internal("AppStore missing".into()))?;
        let cache: Cache = store.cache();
        let hit = cache
            .remember("demo", Some(Duration::from_secs(30)), || async {
                Ok(CachedPing { n: 1 })
            })
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok::<_, Error>(Json(hit))
    });

    app.post("/publish", |mut req: Request| async move {
        let body: PublishBody = req.json().await?;
        let n = req
            .redis()
            .publish(&body.channel, body.message.as_bytes())
            .await?;
        Ok::<_, Error>(Json(serde_json::json!({ "subscribers": n })))
    });

    app.post("/enqueue", |mut req: Request| async move {
        let body: EnqueueBody = req.json().await?;
        let len = req
            .redis()
            .enqueue(&body.queue, body.message.as_bytes())
            .await?;
        Ok::<_, Error>(Json(serde_json::json!({ "length": len })))
    });

    app.get("/dequeue", |req: Request| async move {
        let queue = req
            .query("queue")
            .ok_or_else(|| Error::BadRequest("missing ?queue=".into()))?;
        let item = req.redis().dequeue(queue).await?;
        let payload = item.and_then(|b| String::from_utf8(b).ok());
        Ok::<_, Error>(Json(serde_json::json!({ "message": payload })))
    });

    app.listen(3020).await
}
