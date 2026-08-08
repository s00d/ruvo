//! `[storage]` unset-fill: public_url when builder omitted it.

use ruvo_core::App;
use ruvo_storage::{AppStorage, Storage};

#[tokio::test]
async fn storage_public_url_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new();
    app.configure_from_str(
        r#"
[storage]
public_url = "https://cdn.example/assets"
"#,
    )
    .unwrap();
    app.install(Storage::local(dir.path()));

    let storage = app.try_state::<AppStorage>().expect("AppStorage");
    assert_eq!(
        storage.url("avatars/u1.png").as_deref(),
        Some("https://cdn.example/assets/avatars/u1.png")
    );
}
