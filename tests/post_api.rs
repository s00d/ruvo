//! Smoke scaffold for `post` api.
//!
//! Wire your App factory (Db / Templates / modules::register), then un-ignore:
//!
//! ```ignore
//! #[tokio::test]
//! #[ignore = "requires app factory + migrations"]
//! async fn post_api_smoke() {
//!     let app = /* build App */;
//!     let c = sova::TestClient::tracked(app).unwrap();
//!     let res = c.get("/posts").await;
//!     res.assert_status(200);
//! }
//! ```
