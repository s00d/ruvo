//! Engine integration tests.

use sova_db::DbHandle;
use sova_scope::{
    Ability, AuthorizeEngine, Condition, InviteStore, MatrixBuilder, MembershipStore,
    OwnerRegistry, ResourceRef, ScopeMigrator, ScopeRef, ROLE_ADMIN, ROLE_EDITOR, ROLE_OWNER,
    ROLE_VIEWER, callback_owner,
};
use sova_testing::{apply_migrations, SqliteTestDb};
use std::collections::HashMap;

fn workspace_matrix() -> HashMap<&'static str, sova_scope::RoleMatrix> {
    let mut b = MatrixBuilder::default();
    b.allow_all("workspace", ROLE_OWNER)
        .allow_all("workspace", ROLE_ADMIN)
        .allow(
            "workspace",
            ROLE_EDITOR,
            Ability::View,
            sova_scope::ResourcePattern::All,
            Condition::Always,
        )
        .allow(
            "workspace",
            ROLE_EDITOR,
            Ability::Update,
            sova_scope::ResourcePattern::Kind("page"),
            Condition::Owner,
        )
        .deny(
            "workspace",
            ROLE_EDITOR,
            Ability::Delete,
            sova_scope::ResourcePattern::All,
            Condition::Always,
        )
        .deny(
            "workspace",
            ROLE_EDITOR,
            Ability::Manage,
            sova_scope::ResourcePattern::All,
            Condition::Always,
        )
        .allow(
            "workspace",
            ROLE_VIEWER,
            Ability::View,
            sova_scope::ResourcePattern::All,
            Condition::Always,
        );
    b.build()
}

fn engine_with_page_owner(owner_id: i64, owned_page_id: i64) -> AuthorizeEngine {
    let mut owners = OwnerRegistry::default();
    owners.register(
        "page",
        callback_owner(move |_db, id| async move {
            Ok(if id == owned_page_id {
                Some(owner_id)
            } else {
                None
            })
        }),
    );
    AuthorizeEngine::new(workspace_matrix(), owners)
}

async fn test_db() -> (SqliteTestDb, DbHandle) {
    let db = SqliteTestDb::create();
    apply_migrations::<ScopeMigrator>(db.url()).await;
    let conn = db.connect().await;
    (db, DbHandle::Conn(conn))
}

#[tokio::test]
async fn viewer_can_read_not_edit() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 1);
    MembershipStore::add(&conn, scope.clone(), 10, ROLE_VIEWER)
        .await
        .unwrap();
    let engine = engine_with_page_owner(99, 5);
    let page = ResourceRef::new("page", 5);
    assert!(engine
        .authorize(&conn, scope.clone(), 10, Ability::View, Some(page))
        .await
        .unwrap()
        .view);
    assert!(engine
        .authorize(&conn, scope, 10, Ability::Update, Some(page))
        .await
        .is_err());
}

#[tokio::test]
async fn editor_edits_own_page_only() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 1);
    MembershipStore::add(&conn, scope.clone(), 20, ROLE_EDITOR)
        .await
        .unwrap();
    let engine = engine_with_page_owner(20, 1);
    let own = ResourceRef::new("page", 1);
    let foreign = ResourceRef::new("page", 2);
    engine
        .authorize(&conn, scope.clone(), 20, Ability::Update, Some(own))
        .await
        .unwrap();
    assert!(engine
        .authorize(&conn, scope, 20, Ability::Update, Some(foreign))
        .await
        .is_err());
}

#[tokio::test]
async fn admin_has_full_access() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 1);
    MembershipStore::add(&conn, scope.clone(), 30, ROLE_ADMIN)
        .await
        .unwrap();
    let engine = engine_with_page_owner(99, 1);
    let page = ResourceRef::new("page", 1);
    let caps = engine
        .authorize(&conn, scope, 30, Ability::Delete, Some(page))
        .await
        .unwrap();
    assert!(caps.delete);
    assert!(caps.manage);
}

#[tokio::test]
async fn accept_pending_invite() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 7);
    InviteStore::create(&conn, scope.clone(), "joiner@example.com", ROLE_VIEWER, 1)
        .await
        .unwrap();
    let n = InviteStore::accept_pending_for_email(&conn, 42, "joiner@example.com")
        .await
        .unwrap();
    assert_eq!(n, 1);
    let m = MembershipStore::find(&conn, scope, 42).await.unwrap().unwrap();
    assert_eq!(m.role, ROLE_VIEWER);
}
