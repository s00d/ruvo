//! Auth schema migrations for `Db::migrations::<AuthMigrator>()`.

use sea_orm_migration::prelude::*;

/// Migrator with auth tables — pass to [`ruvo_db::Db::migrations`].
///
/// Compose with app migrations:
/// ```ignore
/// fn migrations() -> Vec<Box<dyn MigrationTrait>> {
///     let mut v = ruvo_passport::AuthMigrator::migrations();
///     v.push(Box::new(m_app::Migration));
///     v
/// }
/// ```
pub struct AuthMigrator;

#[async_trait::async_trait]
impl MigratorTrait for AuthMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut v: Vec<Box<dyn MigrationTrait>> =
            vec![Box::new(m20260307_000001_auth_tables::Migration)];
        #[cfg(feature = "oauth")]
        v.push(Box::new(m20260307_000002_oauth_accounts::Migration));
        v
    }
}

mod m20260307_000001_auth_tables {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(AuthUsers::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthUsers::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(AuthUsers::Email)
                                .string()
                                .not_null()
                                .unique_key(),
                        )
                        .col(ColumnDef::new(AuthUsers::PasswordHash).string().null())
                        .col(
                            ColumnDef::new(AuthUsers::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(AuthRefreshTokens::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthRefreshTokens::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(AuthRefreshTokens::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthRefreshTokens::TokenHash)
                                .string()
                                .not_null()
                                .unique_key(),
                        )
                        .col(
                            ColumnDef::new(AuthRefreshTokens::ExpiresAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthRefreshTokens::RevokedAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(AuthRefreshTokens::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_refresh_user")
                                .from(AuthRefreshTokens::Table, AuthRefreshTokens::UserId)
                                .to(AuthUsers::Table, AuthUsers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(AuthRefreshTokens::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(AuthUsers::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum AuthUsers {
        Table,
        Id,
        Email,
        PasswordHash,
        CreatedAt,
    }

    #[derive(Iden)]
    enum AuthRefreshTokens {
        Table,
        Id,
        UserId,
        TokenHash,
        ExpiresAt,
        RevokedAt,
        CreatedAt,
    }
}

#[cfg(feature = "oauth")]
mod m20260307_000002_oauth_accounts {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            // Ensure password_hash is nullable for oauth-only users (idempotent if already null).
            manager
                .alter_table(
                    Table::alter()
                        .table(AuthUsers::Table)
                        .modify_column(ColumnDef::new(AuthUsers::PasswordHash).string().null())
                        .to_owned(),
                )
                .await
                .ok();

            manager
                .create_table(
                    Table::create()
                        .table(AuthOauthAccounts::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthOauthAccounts::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(AuthOauthAccounts::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(AuthOauthAccounts::Provider).string().not_null())
                        .col(
                            ColumnDef::new(AuthOauthAccounts::ProviderUserId)
                                .string()
                                .not_null(),
                        )
                        .col(ColumnDef::new(AuthOauthAccounts::AccessToken).text().null())
                        .col(ColumnDef::new(AuthOauthAccounts::RefreshToken).text().null())
                        .col(ColumnDef::new(AuthOauthAccounts::ProfileJson).text().null())
                        .col(
                            ColumnDef::new(AuthOauthAccounts::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthOauthAccounts::UpdatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_oauth_user")
                                .from(AuthOauthAccounts::Table, AuthOauthAccounts::UserId)
                                .to(AuthUsers::Table, AuthUsers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .name("uk_auth_oauth_provider_uid")
                        .table(AuthOauthAccounts::Table)
                        .col(AuthOauthAccounts::Provider)
                        .col(AuthOauthAccounts::ProviderUserId)
                        .unique()
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(AuthOauthAccounts::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum AuthUsers {
        Table,
        Id,
        PasswordHash,
    }

    #[derive(Iden)]
    enum AuthOauthAccounts {
        Table,
        Id,
        UserId,
        Provider,
        ProviderUserId,
        AccessToken,
        RefreshToken,
        ProfileJson,
        CreatedAt,
        UpdatedAt,
    }
}
