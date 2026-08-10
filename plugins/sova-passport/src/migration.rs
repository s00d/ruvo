//! Passport schema — single init migration for `Db::migrations::<AuthMigrator>()`.

use sea_orm_migration::prelude::*;

/// Migrator with passport tables — pass to [`sova_db::Db::migrations`].
///
/// Compose with app migrations:
/// ```ignore
/// fn migrations() -> Vec<Box<dyn MigrationTrait>> {
///     let mut v = sova_passport::AuthMigrator::migrations();
///     v.push(Box::new(m_app::Migration));
///     v
/// }
/// ```
pub struct AuthMigrator;

#[async_trait::async_trait]
impl MigratorTrait for AuthMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260308_000001_passport::Migration)]
    }
}

mod m20260308_000001_passport {
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260308_000001_passport"
        }
    }

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
                            ColumnDef::new(AuthUsers::Name)
                                .string()
                                .not_null()
                                .default(""),
                        )
                        .col(ColumnDef::new(AuthUsers::AvatarPath).string().null())
                        .col(
                            ColumnDef::new(AuthUsers::EmailVerifiedAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .col(ColumnDef::new(AuthUsers::TwoFactorSecret).string().null())
                        .col(
                            ColumnDef::new(AuthUsers::TwoFactorRecoveryCodes)
                                .text()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(AuthUsers::TwoFactorConfirmedAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
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

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_auth_refresh_tokens_user_id")
                        .table(AuthRefreshTokens::Table)
                        .col(AuthRefreshTokens::UserId)
                        .to_owned(),
                )
                .await?;

            #[cfg(feature = "oauth")]
            {
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
                            .col(
                                ColumnDef::new(AuthOauthAccounts::Provider)
                                    .string()
                                    .not_null(),
                            )
                            .col(
                                ColumnDef::new(AuthOauthAccounts::ProviderUserId)
                                    .string()
                                    .not_null(),
                            )
                            .col(ColumnDef::new(AuthOauthAccounts::AccessToken).text().null())
                            .col(
                                ColumnDef::new(AuthOauthAccounts::RefreshToken)
                                    .text()
                                    .null(),
                            )
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
                            .if_not_exists()
                            .name("uk_auth_oauth_provider_uid")
                            .table(AuthOauthAccounts::Table)
                            .col(AuthOauthAccounts::Provider)
                            .col(AuthOauthAccounts::ProviderUserId)
                            .unique()
                            .to_owned(),
                    )
                    .await?;

                manager
                    .create_index(
                        Index::create()
                            .if_not_exists()
                            .name("idx_auth_oauth_accounts_user_id")
                            .table(AuthOauthAccounts::Table)
                            .col(AuthOauthAccounts::UserId)
                            .to_owned(),
                    )
                    .await?;
            }

            manager
                .create_table(
                    Table::create()
                        .table(AuthApiTokens::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthApiTokens::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(AuthApiTokens::Name).string().not_null())
                        .col(
                            ColumnDef::new(AuthApiTokens::TokenPrefix)
                                .string_len(16)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::TokenHash)
                                .string()
                                .not_null()
                                .unique_key(),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::Abilities)
                                .text()
                                .not_null()
                                .default("[]"),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::ExpiresAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::LastUsedAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::RevokedAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(AuthApiTokens::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_api_tokens_user")
                                .from(AuthApiTokens::Table, AuthApiTokens::UserId)
                                .to(AuthUsers::Table, AuthUsers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_auth_api_tokens_prefix")
                        .table(AuthApiTokens::Table)
                        .col(AuthApiTokens::TokenPrefix)
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_auth_api_tokens_user_id")
                        .table(AuthApiTokens::Table)
                        .col(AuthApiTokens::UserId)
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(AuthApiTokens::Table).to_owned())
                .await?;
            #[cfg(feature = "oauth")]
            manager
                .drop_table(Table::drop().table(AuthOauthAccounts::Table).to_owned())
                .await?;
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
        Name,
        AvatarPath,
        EmailVerifiedAt,
        TwoFactorSecret,
        TwoFactorRecoveryCodes,
        TwoFactorConfirmedAt,
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

    #[cfg(feature = "oauth")]
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

    #[derive(Iden)]
    enum AuthApiTokens {
        Table,
        Id,
        UserId,
        Name,
        TokenPrefix,
        TokenHash,
        Abilities,
        ExpiresAt,
        LastUsedAt,
        RevokedAt,
        CreatedAt,
    }
}
