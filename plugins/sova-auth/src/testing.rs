//! Test helpers: `acting_as` + user/RBAC factories.
//!
//! Lives here (not `sova-testing`) so crates.io publish has no cycle with the harness.

use crate::{
    assign_role, create_permission, create_role, list_permissions, list_roles, load_current_user,
    register_user, CurrentUser,
};
use sova_core::TestClient;
use sova_db::DbHandle;

/// Inject authenticated user extensions on every [`TestClient`] request.
pub trait ActingAs {
    fn acting_as(&self, user: CurrentUser);
    fn logout(&self);
}

impl ActingAs for TestClient {
    fn acting_as(&self, user: CurrentUser) {
        self.clear_request_hooks();
        self.on_request(move |req| {
            req.set(user.clone());
        });
    }

    fn logout(&self) {
        self.clear_request_hooks();
    }
}

/// Builder that inserts a user via [`register_user`] and returns [`CurrentUser`].
#[derive(Clone, Debug)]
pub struct UserFactory {
    email: String,
    name: String,
    password: String,
    roles: Vec<String>,
}

impl Default for UserFactory {
    fn default() -> Self {
        Self {
            email: "user@example.com".into(),
            name: "Test User".into(),
            password: "password123".into(),
            roles: Vec::new(),
        }
    }
}

impl UserFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    /// Extra roles beyond the default `user` assigned by register.
    pub fn roles<I, S>(mut self, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.roles = roles.into_iter().map(Into::into).collect();
        self
    }

    pub async fn create(&self, db: &DbHandle) -> CurrentUser {
        let u = register_user(db, &self.email, &self.name, &self.password)
            .await
            .expect("register_user");
        for slug in &self.roles {
            assign_role(db, u.id, slug)
                .await
                .unwrap_or_else(|e| panic!("assign_role `{slug}`: {e}"));
        }
        load_current_user(db, u.id)
            .await
            .expect("load_current_user")
            .expect("user exists")
    }
}

/// Ensure a role row exists (create if missing).
pub async fn ensure_role(db: &DbHandle, name: &str, slug: &str) {
    let slug = slug.trim().to_lowercase();
    let roles = list_roles(db).await.expect("list_roles");
    if roles.iter().any(|r| r.slug == slug) {
        return;
    }
    create_role(db, name, &slug)
        .await
        .unwrap_or_else(|e| panic!("create_role `{slug}`: {e}"));
}

/// Ensure a permission row exists (create if missing).
pub async fn ensure_permission(db: &DbHandle, name: &str, slug: &str) {
    let slug = slug.trim().to_lowercase();
    let perms = list_permissions(db).await.expect("list_permissions");
    if perms.iter().any(|p| p.slug == slug) {
        return;
    }
    create_permission(db, name, &slug)
        .await
        .unwrap_or_else(|e| panic!("create_permission `{slug}`: {e}"));
}
