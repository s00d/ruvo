//! Facade `testing` feature must expose the full harness (not only TestClient).

#![cfg(feature = "testing")]

use sova::{ResponseAssert, TestApp, TestClient, assert_json_snapshot};

#[tokio::test]
async fn facade_testing_reexports_test_app_and_client() {
    let (_db, mut app) = TestApp::builder().build().await;
    app.get("/ping", || async { "pong" });
    let c = TestClient::tracked(app).await.unwrap();
    let res = c.get("/ping").await;
    res.assert_status(200);
    assert_json_snapshot!("ping_text_via_facade", serde_json::json!({ "ok": true }));
}
