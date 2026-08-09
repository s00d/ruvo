use sova_core::{App, ResponseAssert, TestClient};
use sova_fs::{Fs, FsError, FsExt};

#[tokio::test]
async fn jail_rejects_escape() {
    let dir = tempfile::tempdir().unwrap();
    let fs = Fs::new(dir.path()).into_fs().await.unwrap();
    assert!(matches!(
        fs.read("../outside.txt").await,
        Err(FsError::Forbidden)
    ));
    assert!(matches!(
        fs.write("/tmp/x", b"no").await,
        Err(FsError::Forbidden)
    ));
}

#[tokio::test]
async fn crud_and_walk() {
    let dir = tempfile::tempdir().unwrap();
    let fs = Fs::new(dir.path())
        .max_walk_entries(100)
        .into_fs()
        .await
        .unwrap();

    fs.create_dir("notes/nested").await.unwrap();
    fs.write("notes/a.txt", b"hi").await.unwrap();
    fs.write_string("notes/nested/b.txt", "bye").await.unwrap();
    assert_eq!(fs.read_to_string("notes/a.txt").await.unwrap(), "hi");
    assert!(fs.exists("notes/a.txt").await.unwrap());

    let kids = fs.read_dir("notes").await.unwrap();
    assert!(kids.iter().any(|e| e.name == "a.txt"));
    assert!(kids.iter().any(|e| e.name == "nested" && e.is_dir));

    let tree = fs.walk("notes").await.unwrap();
    assert!(tree.len() >= 3);

    fs.copy("notes/a.txt", "notes/a.copy.txt").await.unwrap();
    fs.rename("notes/a.copy.txt", "notes/a2.txt")
        .await
        .unwrap();
    fs.append("notes/a2.txt", b"!").await.unwrap();
    assert_eq!(fs.read_to_string("notes/a2.txt").await.unwrap(), "hi!");

    fs.remove_file("notes/a2.txt").await.unwrap();
    fs.remove_dir("notes/nested").await.unwrap();
    assert!(!fs.exists("notes/nested").await.unwrap());
}

#[tokio::test]
async fn plugin_req_fs() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new();
    app.install(Fs::new(dir.path()));
    app.get("/w", |req: sova_core::Request| async move {
        req.fs().write("x.txt", b"ok").await?;
        let s = req.fs().read_to_string("x.txt").await?;
        Ok::<_, sova_core::Error>(s)
    });

    let client = TestClient::new(app).unwrap();
    let res = client.get("/w").await;
    res.assert_status(200);
    let body = String::from_utf8_lossy(res.body_bytes().expect("body"));
    assert_eq!(body, "ok");
}
