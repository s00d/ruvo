//! End-to-end smoke: register → submit → vote → comment.

use hackernews::{build_app_with_db, HnMigrator};
use sova::{ResponseAssert, TestClient};
use sova_testing::SqliteTestDb;

fn csrf_from_html(body: &str) -> String {
    let marker = "name=\"csrf\" value=\"";
    let start = body
        .find(marker)
        .map(|i| i + marker.len())
        .expect("csrf hidden field");
    let rest = &body[start..];
    let end = rest.find('"').expect("csrf value end");
    rest[..end].to_string()
}

fn body_str(res: &sova::Response) -> String {
    String::from_utf8_lossy(res.body_bytes().unwrap()).into_owned()
}

async fn client() -> (SqliteTestDb, TestClient) {
    let db = SqliteTestDb::migrate::<HnMigrator>().await;
    std::env::set_var("FORTIFY_SECRET", "test-hn-secret-please-change!!");
    std::env::set_var("PUBLIC_URL", "http://127.0.0.1:3000");
    std::env::set_var("SOVA_LOG", "off");
    let app = build_app_with_db(Some(db.url())).expect("build_app");
    let c = TestClient::boot(app).await.expect("client");
    (db, c)
}

#[tokio::test]
async fn register_submit_vote_comment() {
    let (_db, c) = client().await;

    let home = c.get("/").await;
    home.assert_status(200);
    let home_body = body_str(&home);
    assert!(
        home_body.contains("Sova News") || home_body.contains("No stories"),
        "{home_body}"
    );

    let reg = c.get("/register").await;
    reg.assert_status(200);
    let csrf = csrf_from_html(&body_str(&reg));

    let registered = c
        .post("/register")
        .form(&[
            ("csrf", csrf.as_str()),
            ("name", "Ada"),
            ("email", "ada@sova.news"),
            ("password", "secret123"),
            ("password_confirmation", "secret123"),
        ])
        .await;
    assert_eq!(registered.status_code().as_u16(), 303, "register redirect");
    assert_eq!(
        registered
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok()),
        Some("/")
    );

    let submit_page = c.get("/submit").await;
    submit_page.assert_status(200);
    let csrf = csrf_from_html(&body_str(&submit_page));

    let submitted = c
        .post("/submit")
        .form(&[
            ("csrf", csrf.as_str()),
            ("title", "Hello Sova News"),
            ("url", "https://s00d.github.io/sova/"),
            ("text", ""),
        ])
        .await;
    assert_eq!(submitted.status_code().as_u16(), 303);
    let loc = submitted
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .expect("location")
        .to_string();
    assert!(loc.starts_with("/item/"), "loc={loc}");

    let item = c.get(&loc).await;
    item.assert_status(200);
    let item_body = body_str(&item);
    assert!(item_body.contains("Hello Sova News"), "{item_body}");
    let csrf = csrf_from_html(&item_body);

    let vote = c
        .post(format!("{loc}/vote"))
        .form(&[("csrf", csrf.as_str())])
        .await;
    assert_eq!(vote.status_code().as_u16(), 303);

    let item = c.get(&loc).await;
    let csrf = csrf_from_html(&body_str(&item));
    let commented = c
        .post(format!("{loc}/comment"))
        .form(&[("csrf", csrf.as_str()), ("body", "First!")])
        .await;
    assert_eq!(commented.status_code().as_u16(), 303);

    let item = c.get(&loc).await;
    item.assert_status(200);
    let body = body_str(&item);
    assert!(body.contains("First!"), "{body}");
    assert!(
        body.contains("2 points") || body.contains("points"),
        "{body}"
    );

    let top = c.get("/").await;
    top.assert_status(200);
    assert!(body_str(&top).contains("Hello Sova News"));
}

#[tokio::test]
async fn submit_requires_auth() {
    let (_db, c) = client().await;

    let res = c.get("/submit").await;
    assert_eq!(res.status_code().as_u16(), 303);
    assert_eq!(
        res.headers().get("location").and_then(|v| v.to_str().ok()),
        Some("/login")
    );
}
