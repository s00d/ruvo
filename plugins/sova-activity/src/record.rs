//! Best-effort activity inserts.

use crate::entity;
use chrono::Utc;
use sova_core::{ClientAddr, Request};
use sova_db::{DbExt, DbHandle};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;

/// One activity row to persist.
#[derive(Debug, Clone)]
pub struct ActivityEntry {
    pub actor_id: Option<i64>,
    pub subject_type: String,
    pub subject_id: String,
    pub event: String,
    pub properties: Value,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Optional actor id attached to the request (e.g. by auth middleware).
#[derive(Clone, Copy, Debug)]
pub struct ActivityActor(pub i64);

/// Marker in app state — installed by [`super::Activity`] plugin.
#[derive(Clone, Debug, Default)]
pub struct ActivityLog;

impl ActivityLog {
    /// Insert a row; logs a warning on failure and never panics the caller.
    pub async fn record(db: &DbHandle, entry: ActivityEntry) {
        let props = serde_json::to_string(&entry.properties).unwrap_or_else(|_| "{}".into());
        let model = entity::ActiveModel {
            actor_id: Set(entry.actor_id),
            subject_type: Set(entry.subject_type),
            subject_id: Set(entry.subject_id),
            event: Set(entry.event),
            properties: Set(props),
            ip: Set(entry.ip),
            user_agent: Set(entry.user_agent),
            created_at: Set(Utc::now()),
            ..Default::default()
        };
        if let Err(e) = model.insert(db).await {
            tracing::warn!(error = %e, "activity_log insert failed");
        }
    }
}

/// Request helper for activity logging.
pub trait ActivityExt {
    fn activity_enabled(&self) -> bool;

    #[allow(async_fn_in_trait)]
    async fn log_activity(
        &self,
        event: &str,
        subject_type: &str,
        subject_id: impl ToString,
        properties: Value,
    );

    #[allow(async_fn_in_trait)]
    async fn log_activity_as(
        &self,
        actor_id: Option<i64>,
        event: &str,
        subject_type: &str,
        subject_id: impl ToString,
        properties: Value,
    );
}

impl ActivityExt for Request {
    fn activity_enabled(&self) -> bool {
        self.try_state::<ActivityLog>().is_some()
    }

    async fn log_activity(
        &self,
        event: &str,
        subject_type: &str,
        subject_id: impl ToString,
        properties: Value,
    ) {
        let actor = self.get::<ActivityActor>().map(|a| a.0);
        self.log_activity_as(actor, event, subject_type, subject_id, properties)
            .await;
    }

    async fn log_activity_as(
        &self,
        actor_id: Option<i64>,
        event: &str,
        subject_type: &str,
        subject_id: impl ToString,
        properties: Value,
    ) {
        if !self.activity_enabled() {
            return;
        }
        let ip = self
            .get::<ClientAddr>()
            .map(|a| a.0.ip().to_string())
            .or_else(|| {
                self.header("x-forwarded-for")
                    .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
            });
        let user_agent = self.header("user-agent").map(str::to_string);
        ActivityLog::record(
            self.db(),
            ActivityEntry {
                actor_id,
                subject_type: subject_type.into(),
                subject_id: subject_id.to_string(),
                event: event.into(),
                properties,
                ip,
                user_agent,
            },
        )
        .await;
    }
}
