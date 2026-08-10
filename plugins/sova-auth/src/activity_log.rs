//! Optional activity logging (feature `activity`).

#![cfg(feature = "activity")]

use serde_json::Value;
use sova_activity::ActivityExt;
use sova_core::Request;

pub async fn log_event(
    req: &Request,
    actor_id: Option<i64>,
    event: &str,
    subject_type: &str,
    subject_id: impl ToString,
    properties: Value,
) {
    req.log_activity_as(actor_id, event, subject_type, subject_id, properties)
        .await;
}
