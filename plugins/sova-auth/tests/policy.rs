//! Policy unit tests.

use sova_auth::{Ability, AuthExt, CurrentUser, Policy};
use sova_core::{Request, Result};

#[derive(Default)]
struct Note {
    user_id: i64,
}

#[derive(Default)]
struct NotePolicy;

impl Policy<Note> for NotePolicy {
    fn view(&self, user: &CurrentUser, r: &Note) -> bool {
        user.id == r.user_id || user.has_role("admin") || user.has_permission("notes.manage")
    }
    fn update(&self, user: &CurrentUser, r: &Note) -> bool {
        self.view(user, r)
    }
    fn delete(&self, user: &CurrentUser, r: &Note) -> bool {
        self.view(user, r)
    }
}

fn user(id: i64, roles: &[&str], perms: &[&str]) -> CurrentUser {
    CurrentUser {
        id,
        email: format!("{id}@t.test"),
        name: "t".into(),
        avatar_path: None,
        email_verified: true,
        two_factor_enabled: false,
        roles: roles.iter().map(|s| (*s).into()).collect(),
        permissions: perms.iter().map(|s| (*s).into()).collect(),
    }
}

#[test]
fn owner_can_delete() {
    let mut req = Request::builder().path("/").build();
    req.set(user(1, &[], &[]));
    let note = Note { user_id: 1 };
    assert!(req.can::<NotePolicy, _>(Ability::Delete, &note));
    assert!(req
        .authorize::<NotePolicy, _>(Ability::Delete, &note)
        .is_ok());
}

#[test]
fn stranger_denied() {
    let mut req = Request::builder().path("/").build();
    req.set(user(2, &[], &[]));
    let note = Note { user_id: 1 };
    assert!(!req.can::<NotePolicy, _>(Ability::Delete, &note));
    let err = req
        .authorize::<NotePolicy, _>(Ability::Delete, &note)
        .unwrap_err();
    assert!(matches!(err, sova_core::Error::Forbidden));
}

#[test]
fn admin_or_manage_ok() {
    let note = Note { user_id: 1 };
    let mut admin = Request::builder().path("/").build();
    admin.set(user(9, &["admin"], &[]));
    assert!(admin
        .authorize::<NotePolicy, _>(Ability::View, &note)
        .is_ok());

    let mut mgr = Request::builder().path("/").build();
    mgr.set(user(8, &[], &["notes.manage"]));
    assert!(mgr
        .authorize::<NotePolicy, _>(Ability::Delete, &note)
        .is_ok());
}

#[test]
fn authorize_with_predicate() -> Result<()> {
    let mut req = Request::builder().path("/").build();
    req.set(user(1, &[], &[]));
    req.authorize_with(|u| u.id == 1)?;
    assert!(req.authorize_with(|u| u.id == 99).is_err());
    Ok(())
}
