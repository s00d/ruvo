//! App-like integration flows: invite, cache, audit.

use sova_db::DbHandle;
use sova_scope::{
    Ability, AuditStore, AuthorizeEngine, Condition, InviteStore, MatrixBuilder, MembershipStore,
    MutateOpts, OwnerRegistry, ResourceRef, ScopeMigrator, ScopeRef, AUDIT_ADDED, AUDIT_REMOVED,
    AUDIT_ROLE_CHANGED, ROLE_ADMIN, ROLE_EDITOR, ROLE_OWNER, ROLE_VIEWER, callback_owner,
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

fn engine() -> AuthorizeEngine {
    AuthorizeEngine::new(workspace_matrix(), OwnerRegistry::default())
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
async fn invite_token_flow_authorize() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 3);
    let inv = InviteStore::create(&conn, scope.clone(), "new@ex.com", ROLE_VIEWER, 1)
        .await
        .unwrap();
    let eng = engine();
    let opts = MutateOpts::default().cache(eng.membership_cache());
    InviteStore::accept_by_token_with(&conn, 99, "new@ex.com", &inv.token, opts)
        .await
        .unwrap();
    eng.authorize(&conn, scope.clone(), 99, Ability::View, None)
        .await
        .unwrap();
    assert!(eng
        .authorize(&conn, scope, 99, Ability::Manage, None)
        .await
        .is_err());
}

#[tokio::test]
async fn membership_cache_stale_until_invalidate() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 1);
    MembershipStore::add(&conn, scope.clone(), 7, ROLE_VIEWER)
        .await
        .unwrap();
    let eng = engine();
    let m1 = eng.membership(&conn, scope.clone(), 7).await.unwrap().unwrap();
    assert_eq!(m1.role, ROLE_VIEWER);

    // Bypass store helpers: raw update via MembershipStore without cache opts leaves engine cache stale.
    let row = MembershipStore::find(&conn, scope.clone(), 7)
        .await
        .unwrap()
        .unwrap();
    MembershipStore::update_role(&conn, row.id, ROLE_EDITOR)
        .await
        .unwrap();

    let stale = eng.membership(&conn, scope.clone(), 7).await.unwrap().unwrap();
    assert_eq!(stale.role, ROLE_VIEWER, "positive cache still holds old role");

    eng.membership_cache().invalidate(&scope, 7);
    let fresh = eng.membership(&conn, scope, 7).await.unwrap().unwrap();
    assert_eq!(fresh.role, ROLE_EDITOR);
}

#[tokio::test]
async fn mutate_with_cache_keeps_engine_fresh() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 2);
    let eng = engine();
    let opts = MutateOpts::default()
        .actor(1)
        .cache(eng.membership_cache());
    MembershipStore::add_with(&conn, scope.clone(), 8, ROLE_VIEWER, opts)
        .await
        .unwrap();
    let _ = eng.membership(&conn, scope.clone(), 8).await.unwrap();
    let row = MembershipStore::find(&conn, scope.clone(), 8)
        .await
        .unwrap()
        .unwrap();
    MembershipStore::update_role_with(&conn, row.id, ROLE_ADMIN, opts)
        .await
        .unwrap();
    let m = eng.membership(&conn, scope, 8).await.unwrap().unwrap();
    assert_eq!(m.role, ROLE_ADMIN);
}

#[tokio::test]
async fn role_change_writes_audit() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 5);
    let m = MembershipStore::add_with(
        &conn,
        scope.clone(),
        11,
        ROLE_EDITOR,
        MutateOpts::default().actor(1),
    )
    .await
    .unwrap();
    MembershipStore::update_role_with(
        &conn,
        m.id,
        ROLE_ADMIN,
        MutateOpts::default().actor(1),
    )
    .await
    .unwrap();
    MembershipStore::remove_with(&conn, m.id, MutateOpts::default().actor(1))
        .await
        .unwrap();

    let rows = AuditStore::list_for_scope(&conn, scope, 20).await.unwrap();
    let actions: Vec<_> = rows.iter().map(|r| r.action.as_str()).collect();
    assert!(actions.contains(&AUDIT_ADDED));
    assert!(actions.contains(&AUDIT_ROLE_CHANGED));
    assert!(actions.contains(&AUDIT_REMOVED));
    assert_eq!(rows.iter().find(|r| r.action == AUDIT_ADDED).unwrap().actor_id, Some(1));
}

#[tokio::test]
async fn accept_pending_email_invalidates_cache() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 9);
    InviteStore::create(&conn, scope.clone(), "batch@ex.com", ROLE_EDITOR, 2)
        .await
        .unwrap();
    let eng = engine_with_page_owner(55, 1);
    let opts = MutateOpts::default().cache(eng.membership_cache());
    let n = InviteStore::accept_pending_for_email_with(&conn, 55, "batch@ex.com", opts)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let page = ResourceRef::new("page", 1);
    eng.authorize(&conn, scope, 55, Ability::Update, Some(page))
        .await
        .unwrap();
}

#[tokio::test]
async fn invite_accept_records_audit_added() {
    let (_db, conn) = test_db().await;
    let scope = ScopeRef::new("workspace", 4);
    let inv = InviteStore::create(&conn, scope.clone(), "a@b.c", ROLE_VIEWER, 3)
        .await
        .unwrap();
    InviteStore::accept_by_token(&conn, 77, "a@b.c", &inv.token)
        .await
        .unwrap();
    let rows = AuditStore::list_for_user(&conn, 77, 10).await.unwrap();
    assert!(rows.iter().any(|r| r.action == AUDIT_ADDED && r.new_role.as_deref() == Some(ROLE_VIEWER)));
    assert_eq!(
        rows.iter().find(|r| r.action == AUDIT_ADDED).unwrap().actor_id,
        Some(3)
    );
}
