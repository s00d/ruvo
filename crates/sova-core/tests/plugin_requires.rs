//! Plugin `requires()` is checked on `App::build`.

use sova_core::{App, Plugin};

struct NeedsDb;

impl Plugin for NeedsDb {
    fn id(&self) -> &'static str {
        "needs-db"
    }

    fn requires(&self) -> &'static [&'static str] {
        &["db"]
    }

    fn install(self, _app: &mut App) {}
}

struct FakeDb;

impl Plugin for FakeDb {
    fn id(&self) -> &'static str {
        "db"
    }

    fn install(self, _app: &mut App) {}
}

#[test]
fn missing_requires_fails_build() {
    let mut app = App::new();
    app.install(NeedsDb);
    match app.build() {
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("needs-db") && msg.contains("db"),
                "unexpected error: {msg}"
            );
        }
        Ok(_) => panic!("should fail without db"),
    }
}

#[test]
fn requires_satisfied_when_dep_installed_first() {
    let mut app = App::new();
    app.install(FakeDb);
    app.install(NeedsDb);
    assert!(app.build().is_ok());
}

#[test]
fn requires_fails_when_dep_installed_after() {
    let mut app = App::new();
    app.install(NeedsDb);
    app.install(FakeDb);
    match app.build() {
        Err(err) => assert!(err.to_string().contains("requires `db`")),
        Ok(_) => panic!("order matters at install"),
    }
}
