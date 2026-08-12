//! Scope invitation store.

#[cfg(feature = "invites")]
mod inner {
    use crate::cache::MutateOpts;
    use crate::entity::{scope_invitation, ScopeInvitation};
    use crate::membership::MembershipStore;
    use crate::types::{
        normalize_role, ScopeRef, INVITE_ACCEPTED, INVITE_PENDING, INVITE_REVOKED, ROLE_VIEWER,
    };
    use chrono::Utc;
    use rand::RngCore;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use sova_core::Result;
    use sova_db::{ActiveModelTrait, DbError, DbHandle, Set};

    pub struct InviteStore;

    fn invite_token() -> String {
        let mut buf = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut buf);
        buf.iter().map(|b| format!("{b:02x}")).collect()
    }

    impl InviteStore {
        pub async fn list_pending(
            db: &DbHandle,
            scope: ScopeRef,
        ) -> Result<Vec<scope_invitation::Model>> {
            Ok(ScopeInvitation::find()
                .filter(scope_invitation::Column::ScopeKind.eq(scope.kind))
                .filter(scope_invitation::Column::ScopeId.eq(scope.id))
                .filter(scope_invitation::Column::Status.eq(INVITE_PENDING))
                .all(db)
                .await
                .map_err(DbError::from)?)
        }

        pub async fn find_pending_by_email(
            db: &DbHandle,
            scope: ScopeRef,
            email: &str,
        ) -> Result<Option<scope_invitation::Model>> {
            Ok(ScopeInvitation::find()
                .filter(scope_invitation::Column::ScopeKind.eq(scope.kind))
                .filter(scope_invitation::Column::ScopeId.eq(scope.id))
                .filter(scope_invitation::Column::Email.eq(email.trim().to_lowercase()))
                .filter(scope_invitation::Column::Status.eq(INVITE_PENDING))
                .one(db)
                .await
                .map_err(DbError::from)?)
        }

        pub async fn create(
            db: &DbHandle,
            scope: ScopeRef,
            email: &str,
            role: &str,
            invited_by: i64,
        ) -> Result<scope_invitation::Model> {
            let role = normalize_role(role).unwrap_or(ROLE_VIEWER);
            Ok(scope_invitation::ActiveModel {
                scope_kind: Set(scope.kind),
                scope_id: Set(scope.id),
                email: Set(email.trim().to_lowercase()),
                role: Set(role.into()),
                invited_by: Set(invited_by),
                token: Set(invite_token()),
                status: Set(INVITE_PENDING.into()),
                created_at: Set(Utc::now()),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(DbError::from)?)
        }

        pub async fn revoke(db: &DbHandle, id: i64) -> Result<()> {
            let row = ScopeInvitation::find_by_id(id)
                .one(db)
                .await
                .map_err(DbError::from)?
                .ok_or(sova_core::Error::NotFound)?;
            let mut am: scope_invitation::ActiveModel = row.into();
            am.status = Set(INVITE_REVOKED.into());
            am.update(db).await.map_err(DbError::from)?;
            Ok(())
        }

        pub async fn accept_pending_for_email(
            db: &DbHandle,
            user_id: i64,
            email: &str,
        ) -> Result<u32> {
            Self::accept_pending_for_email_with(db, user_id, email, MutateOpts::default()).await
        }

        pub async fn accept_pending_for_email_with(
            db: &DbHandle,
            user_id: i64,
            email: &str,
            opts: MutateOpts<'_>,
        ) -> Result<u32> {
            let email = email.trim().to_lowercase();
            let pending = ScopeInvitation::find()
                .filter(scope_invitation::Column::Email.eq(&email))
                .filter(scope_invitation::Column::Status.eq(INVITE_PENDING))
                .all(db)
                .await
                .map_err(DbError::from)?;
            let mut count = 0u32;
            for inv in pending {
                let scope = ScopeRef::new(inv.scope_kind.clone(), inv.scope_id);
                if MembershipStore::find(db, scope.clone(), user_id)
                    .await?
                    .is_some()
                {
                    continue;
                }
                let mut add_opts = MutateOpts {
                    actor_id: opts.actor_id.or(Some(inv.invited_by)),
                    cache: opts.cache,
                };
                if add_opts.actor_id.is_none() {
                    add_opts.actor_id = Some(user_id);
                }
                MembershipStore::add_with(db, scope, user_id, &inv.role, add_opts).await?;
                let mut am: scope_invitation::ActiveModel = inv.into();
                am.status = Set(INVITE_ACCEPTED.into());
                am.update(db).await.map_err(DbError::from)?;
                count += 1;
            }
            Ok(count)
        }

        pub async fn find_by_token(
            db: &DbHandle,
            token: &str,
        ) -> Result<Option<scope_invitation::Model>> {
            let token = token.trim();
            if token.is_empty() {
                return Ok(None);
            }
            Ok(ScopeInvitation::find()
                .filter(scope_invitation::Column::Token.eq(token))
                .one(db)
                .await
                .map_err(DbError::from)?)
        }

        /// Accept a pending invite by token. Email on the invite must match `user_email`.
        pub async fn accept_by_token(
            db: &DbHandle,
            user_id: i64,
            user_email: &str,
            token: &str,
        ) -> Result<scope_invitation::Model> {
            Self::accept_by_token_with(db, user_id, user_email, token, MutateOpts::default()).await
        }

        pub async fn accept_by_token_with(
            db: &DbHandle,
            user_id: i64,
            user_email: &str,
            token: &str,
            opts: MutateOpts<'_>,
        ) -> Result<scope_invitation::Model> {
            let inv = Self::find_by_token(db, token)
                .await?
                .ok_or(sova_core::Error::NotFound)?;
            if inv.status != INVITE_PENDING {
                return Err(sova_core::Error::custom(409, "invitation is not pending"));
            }
            let email = user_email.trim().to_lowercase();
            if inv.email != email {
                return Err(sova_core::Error::custom(
                    403,
                    "invitation email does not match your account",
                ));
            }
            let scope = ScopeRef::new(inv.scope_kind.clone(), inv.scope_id);
            if MembershipStore::find(db, scope.clone(), user_id)
                .await?
                .is_none()
            {
                let add_opts = MutateOpts {
                    actor_id: opts.actor_id.or(Some(inv.invited_by)),
                    cache: opts.cache,
                };
                MembershipStore::add_with(db, scope, user_id, &inv.role, add_opts).await?;
            } else if let Some(cache) = opts.cache {
                cache.invalidate(
                    &ScopeRef::new(inv.scope_kind.clone(), inv.scope_id),
                    user_id,
                );
            }
            let mut am: scope_invitation::ActiveModel = inv.into();
            am.status = Set(INVITE_ACCEPTED.into());
            Ok(am.update(db).await.map_err(DbError::from)?)
        }
    }
}

#[cfg(feature = "invites")]
pub use inner::InviteStore;
