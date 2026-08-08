//! Notifications insert / list / mark / to_many.

use ruvo_core::{Json, Request, ResponseAssert, TestClient};
use ruvo_notifications::{Channel, Notifications, NotificationsMigrator, Notify, Via};
use ruvo_testing::{ActingAs, TestApp};
use serde_json::json;

#[tokio::test]
async fn insert_list_mark_and_to_many() {
    let (_db, app) = TestApp::builder()
        .migrator::<NotificationsMigrator>()
        .install(
            Notifications::new()
                .channel(Channel::new("orders"))
                .mount("/notifications"),
        )
        .configure(|app| {
            app.post("/send", |req: Request| async move {
                let rows = Notify::to_many([1, 2])
                    .channel("orders")
                    .event("order.shipped")
                    .title("Shipped")
                    .data(json!({ "order_id": 9 }))
                    .via([Via::Database])
                    .send(&req)
                    .await?;
                Ok::<_, ruvo_core::Error>(Json(json!({ "sent": rows.len() })))
            });
        })
        .build()
        .await;

    let c = TestClient::tracked(app).unwrap();

    let res = c.post("/send").await;
    res.assert_status(200);
    assert_eq!(res.json_value()["sent"], 2);

    c.acting_as_id(1);
    let r1 = c.get("/notifications?unread=1").await;
    r1.assert_status(200);
    let body1: Vec<serde_json::Value> = r1.json();
    assert_eq!(body1.len(), 1);
    assert_eq!(body1[0]["event"], "order.shipped");
    let id = body1[0]["id"].as_i64().unwrap();

    c.post(format!("/notifications/{id}/read"))
        .await
        .assert_status(200);

    let count = c.get("/notifications/unread-count").await;
    count.assert_status(200);
    assert_eq!(count.json_value()["count"], 0);

    c.acting_as_id(2);
    let r2 = c.get("/notifications?unread=1").await;
    r2.assert_status(200);
    let body2: Vec<serde_json::Value> = r2.json();
    assert_eq!(body2.len(), 1);
}

#[tokio::test]
async fn mark_all_read_clears_unread() {
    let (_db, app) = TestApp::builder()
        .migrator::<NotificationsMigrator>()
        .install(
            Notifications::new()
                .channel(Channel::new("orders"))
                .mount("/notifications"),
        )
        .configure(|app| {
            app.post("/send", |req: Request| async move {
                Notify::to_many([1])
                    .channel("orders")
                    .event("a")
                    .title("A")
                    .via([Via::Database])
                    .send(&req)
                    .await?;
                Notify::to_many([1])
                    .channel("orders")
                    .event("b")
                    .title("B")
                    .via([Via::Database])
                    .send(&req)
                    .await?;
                Ok::<_, ruvo_core::Error>(Json(json!({ "ok": true })))
            });
        })
        .build()
        .await;

    let c = TestClient::tracked(app).unwrap();
    c.post("/send").await.assert_status(200);

    c.acting_as_id(1);
    let count = c.get("/notifications/unread-count").await;
    count.assert_status(200);
    assert_eq!(count.json_value()["count"], 2);

    c.post("/notifications/read-all").await.assert_status(200);
    let after = c.get("/notifications/unread-count").await;
    after.assert_status(200);
    assert_eq!(after.json_value()["count"], 0);
}

#[tokio::test]
async fn http_list_requires_user() {
    let (_db, app) = TestApp::builder()
        .migrator::<NotificationsMigrator>()
        .install(
            Notifications::new()
                .channel(Channel::new("default"))
                .mount("/notifications"),
        )
        .build()
        .await;

    let c = TestClient::tracked(app).unwrap();
    c.get("/notifications").await.assert_status(401);
}
