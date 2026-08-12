//! Core scope authorization types.

use serde::Serialize;
use sova_core::Error;

pub const ROLE_OWNER: &str = "owner";
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_EDITOR: &str = "editor";
pub const ROLE_VIEWER: &str = "viewer";

pub const INVITE_PENDING: &str = "pending";
pub const INVITE_ACCEPTED: &str = "accepted";
pub const INVITE_REVOKED: &str = "revoked";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScopeRef {
    pub kind: String,
    pub id: i64,
}

impl ScopeRef {
    pub fn new(kind: impl Into<String>, id: i64) -> Self {
        Self {
            kind: kind.into(),
            id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceRef {
    pub kind: &'static str,
    pub id: i64,
}

impl ResourceRef {
    pub fn new(kind: &'static str, id: i64) -> Self {
        Self { kind, id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ability {
    View,
    Create,
    Update,
    Delete,
    Manage,
}

#[derive(Clone, Debug, Serialize)]
pub struct Membership {
    pub id: i64,
    pub scope_kind: String,
    pub scope_id: i64,
    pub user_id: i64,
    pub role: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Capabilities {
    pub view: bool,
    pub create: bool,
    pub update: bool,
    pub delete: bool,
    pub manage: bool,
}

impl Capabilities {
    pub fn full() -> Self {
        Self {
            view: true,
            create: true,
            update: true,
            delete: true,
            manage: true,
        }
    }

    pub fn read_only() -> Self {
        Self {
            view: true,
            ..Default::default()
        }
    }

    pub fn has(&self, ability: Ability) -> bool {
        match ability {
            Ability::View => self.view,
            Ability::Create => self.create,
            Ability::Update => self.update,
            Ability::Delete => self.delete,
            Ability::Manage => self.manage,
        }
    }
}

pub fn normalize_role(role: &str) -> Option<&'static str> {
    match role.trim().to_lowercase().as_str() {
        "owner" => Some(ROLE_OWNER),
        "admin" => Some(ROLE_ADMIN),
        "editor" => Some(ROLE_EDITOR),
        "viewer" | "read" => Some(ROLE_VIEWER),
        _ => None,
    }
}

pub fn is_privileged_role(role: &str) -> bool {
    role == ROLE_OWNER || role == ROLE_ADMIN
}

pub fn ability_error(ability: Ability) -> Error {
    match ability {
        Ability::View => Error::NotFound,
        _ => Error::custom(403, "forbidden"),
    }
}
