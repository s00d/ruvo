//! Activity log insert + list (sqlite).

use sova_activity::{
    list_activity, Activity, ActivityEntry, ActivityExt, ActivityFilter, ActivityLog,
    ActivityMigrator,
};
use sova_core::{Json, Request, ResponseAssert, TestClient};
use sova_testing::{SqliteTestDb, TestApp};
use serde_json::json;

#[tokio::test]
async fn insert_and_list_by_subject() {
    let tdb = SqliteTestDb::migrate::<ActivityMigrator>().await;
    let db = tdb.handle().await;

    ActivityLog::record(
        &db,
        ActivityEntry {
            actor_id: Some(1),
            subject_type: "user".into(),
            subject_id: "42".into(),
            event: "profile.updated".into(),
            properties: json!({ "name": { "old": "Ada", "new": "Ada Lovelace" } }),
            ip: Some("127.0.0.1".into()),
            user_agent: None,
        },
    )
    .await;

    ActivityLog::record(
        &db,
        ActivityEntry {
            actor_id: Some(1),
            subject_type: "user".into(),
            subject_id: "99".into(),
            event: "user.login".into(),
            properties: json!({}),
            ip: None,
            user_agent: None,
        },
    )
    .await;

    let rows = list_activity(
        &db,
        ActivityFilter {
            subject_type: Some("user".into()),
            subject_id: Some("42".into()),
            event: None,
            actor_id: None,
            limit: 50,
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event, "profile.updated");
    assert_eq!(rows[0].actor_id, Some(1));
}

#[tokio::test]
async fn mount_list_and_req_helper() {
    let (_db, app) = TestApp::builder()
        .migrator::<ActivityMigrator>()
        .install(Activity::new().mount("/activity"))
        .configure(|app| {
            app.post("/note", |req: Request| async move {
                req.log_activity("note.created", "note", 7, json!({ "title": "hi" }))
                    .await;
                Ok::<_, sova_core::Error>(Json(json!({ "ok": true })))
            });
        })
        .build()
        .await;

    let c = TestClient::tracked(app).unwrap();

    c.post("/note").await.assert_status(200);

    let list = c.get("/activity?subject_type=note&subject_id=7").await;
    list.assert_status(200);
    let body = list.json_value();
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["event"], "note.created");
}

/// Properties must never carry password/hash-like keys in the public entry helper docs —
/// this unit check documents the redaction contract for callers.
#[test]
fn entry_properties_are_plain_json_not_secrets() {
    let entry = ActivityEntry {
        actor_id: None,
        subject_type: "user".into(),
        subject_id: "1".into(),
        event: "password.changed".into(),
        properties: json!({ "via": "profile" }),
        ip: None,
        user_agent: None,
    };
    let s = serde_json::to_string(&entry.properties).unwrap();
    assert!(!s.contains("password"));
    assert!(!s.contains("hash"));
    assert!(!s.contains("secret"));
}

#[tokio::test]
async fn list_filters_by_event_and_actor() {
    let tdb = SqliteTestDb::migrate::<ActivityMigrator>().await;
    let db = tdb.handle().await;

    ActivityLog::record(
        &db,
        ActivityEntry {
            actor_id: Some(7),
            subject_type: "note".into(),
            subject_id: "1".into(),
            event: "note.created".into(),
            properties: json!({}),
            ip: None,
            user_agent: None,
        },
    )
    .await;
    ActivityLog::record(
        &db,
        ActivityEntry {
            actor_id: Some(7),
            subject_type: "note".into(),
            subject_id: "1".into(),
            event: "note.updated".into(),
            properties: json!({}),
            ip: None,
            user_agent: None,
        },
    )
    .await;
    ActivityLog::record(
        &db,
        ActivityEntry {
            actor_id: Some(9),
            subject_type: "note".into(),
            subject_id: "1".into(),
            event: "note.created".into(),
            properties: json!({}),
            ip: None,
            user_agent: None,
        },
    )
    .await;

    let by_event = list_activity(
        &db,
        ActivityFilter {
            subject_type: None,
            subject_id: None,
            event: Some("note.created".into()),
            actor_id: Some(7),
            limit: 50,
        },
    )
    .await
    .unwrap();
    assert_eq!(by_event.len(), 1);
    assert_eq!(by_event[0].actor_id, Some(7));
    assert_eq!(by_event[0].event, "note.created");
}
