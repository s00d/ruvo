//! Scoped RBAC: membership, role matrix, ownership hooks.

mod audit;
mod cache;
mod engine;
mod entity;
mod ext;
mod invites;
mod matrix;
mod membership;
mod migration;
mod owner;
mod plugin;
mod types;

pub use audit::{
    AuditEntry, AuditStore, AUDIT_ADDED, AUDIT_REMOVED, AUDIT_ROLE_CHANGED,
};
pub use cache::{MembershipCache, MutateOpts};
pub use engine::{AuthorizeEngine, SharedEngine};
pub use entity::{
    scope_membership, scope_membership_audit, ScopeMembership, ScopeMembershipAudit,
};
#[cfg(feature = "invites")]
pub use entity::{scope_invitation, ScopeInvitation};
pub use ext::ScopeExt;
pub use invites::InviteStore;
pub use matrix::{Condition, MatrixBuilder, ResourcePattern, RoleMatrix};
pub use membership::MembershipStore;
pub use migration::ScopeMigrator;
pub use owner::{callback_owner, FnOwnerResolver, OwnerRegistry, OwnerResolver};
pub use plugin::{ScopeAuth, ScopeAuthState};
pub use types::{
    is_privileged_role, normalize_role, Ability, Capabilities, Membership, ResourceRef, ScopeRef,
    INVITE_ACCEPTED, INVITE_PENDING, INVITE_REVOKED, ROLE_ADMIN, ROLE_EDITOR, ROLE_OWNER,
    ROLE_VIEWER,
};
