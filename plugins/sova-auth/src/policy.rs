//! Resource policies (Laravel-style abilities on a model).

use crate::store::CurrentUser;
use sova_core::{Error, Result};

/// CRUD-ish abilities checked by [`Policy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ability {
    View,
    Create,
    Update,
    Delete,
}

/// Authorize actions against a resource type.
pub trait Policy<R>: Send + Sync {
    fn view(&self, user: &CurrentUser, r: &R) -> bool;
    fn create(&self, _user: &CurrentUser) -> bool {
        true
    }
    fn update(&self, user: &CurrentUser, r: &R) -> bool;
    fn delete(&self, user: &CurrentUser, r: &R) -> bool;
}

/// Helpers on [`sova_core::Request`] via [`crate::AuthExt`].
pub(crate) fn can_ability<P, R>(user: &CurrentUser, ability: Ability, resource: &R) -> bool
where
    P: Policy<R> + Default,
{
    let policy = P::default();
    match ability {
        Ability::View => policy.view(user, resource),
        Ability::Create => policy.create(user),
        Ability::Update => policy.update(user, resource),
        Ability::Delete => policy.delete(user, resource),
    }
}

pub(crate) fn authorize_ability<P, R>(
    user: &CurrentUser,
    ability: Ability,
    resource: &R,
) -> Result<()>
where
    P: Policy<R> + Default,
{
    if can_ability::<P, R>(user, ability, resource) {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
