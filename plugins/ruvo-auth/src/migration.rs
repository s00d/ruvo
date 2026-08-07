//! Fortify-style AuthMigrator: passport tables + profile/2FA/RBAC.

use sea_orm_migration::prelude::*;

/// Compose passport + fortify migrations for `Db::migrations::<AuthMigrator>()`.
pub struct AuthMigrator;

#[async_trait::async_trait]
impl MigratorTrait for AuthMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut v = ruvo_passport::AuthMigrator::migrations();
        v.push(Box::new(m20260307_000003_fortify::Migration));
        v.push(Box::new(m20260307_000004_rbac::Migration));
        v
    }
}

mod m20260307_000003_fortify {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            // Extend auth_users with Fortify columns (idempotent-ish via IF NOT EXISTS pattern).
            for sql in [
                "ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS name varchar NOT NULL DEFAULT ''",
                "ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS avatar_path varchar NULL",
                "ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS email_verified_at timestamptz NULL",
                "ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS two_factor_secret varchar NULL",
                "ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS two_factor_recovery_codes text NULL",
                "ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS two_factor_confirmed_at timestamptz NULL",
            ] {
                manager.get_connection().execute_unprepared(sql).await?;
            }

            manager
                .create_table(
                    Table::create()
                        .table(AuthPasswordResetTokens::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthPasswordResetTokens::Email)
                                .string()
                                .not_null()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(AuthPasswordResetTokens::TokenHash)
                                .string()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthPasswordResetTokens::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(
                    Table::drop()
                        .table(AuthPasswordResetTokens::Table)
                        .to_owned(),
                )
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum AuthPasswordResetTokens {
        Table,
        Email,
        TokenHash,
        CreatedAt,
    }
}

mod m20260307_000004_rbac {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(AuthRoles::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthRoles::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(AuthRoles::Name).string().not_null())
                        .col(
                            ColumnDef::new(AuthRoles::Slug)
                                .string()
                                .not_null()
                                .unique_key(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(AuthPermissions::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthPermissions::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(AuthPermissions::Name).string().not_null())
                        .col(
                            ColumnDef::new(AuthPermissions::Slug)
                                .string()
                                .not_null()
                                .unique_key(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(AuthRoleUser::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthRoleUser::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthRoleUser::RoleId)
                                .big_integer()
                                .not_null(),
                        )
                        .primary_key(
                            Index::create()
                                .col(AuthRoleUser::UserId)
                                .col(AuthRoleUser::RoleId),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(AuthPermissionRole::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(AuthPermissionRole::RoleId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(AuthPermissionRole::PermissionId)
                                .big_integer()
                                .not_null(),
                        )
                        .primary_key(
                            Index::create()
                                .col(AuthPermissionRole::RoleId)
                                .col(AuthPermissionRole::PermissionId),
                        )
                        .to_owned(),
                )
                .await?;

            // Seed roles + permissions
            let conn = manager.get_connection();
            conn.execute_unprepared(
                "INSERT INTO auth_roles (name, slug) VALUES ('User', 'user'), ('Admin', 'admin') ON CONFLICT (slug) DO NOTHING",
            )
            .await?;
            conn.execute_unprepared(
                "INSERT INTO auth_permissions (name, slug) VALUES \
                 ('Cabinet access', 'cabinet.access'), \
                 ('Manage users', 'users.manage') \
                 ON CONFLICT (slug) DO NOTHING",
            )
            .await?;
            conn.execute_unprepared(
                "INSERT INTO auth_permission_role (role_id, permission_id) \
                 SELECT r.id, p.id FROM auth_roles r, auth_permissions p \
                 WHERE r.slug = 'user' AND p.slug = 'cabinet.access' \
                 ON CONFLICT DO NOTHING",
            )
            .await?;
            conn.execute_unprepared(
                "INSERT INTO auth_permission_role (role_id, permission_id) \
                 SELECT r.id, p.id FROM auth_roles r, auth_permissions p \
                 WHERE r.slug = 'admin' \
                 ON CONFLICT DO NOTHING",
            )
            .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(AuthPermissionRole::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(AuthRoleUser::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(AuthPermissions::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(AuthRoles::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum AuthRoles {
        Table,
        Id,
        Name,
        Slug,
    }

    #[derive(Iden)]
    enum AuthPermissions {
        Table,
        Id,
        Name,
        Slug,
    }

    #[derive(Iden)]
    enum AuthRoleUser {
        Table,
        UserId,
        RoleId,
    }

    #[derive(Iden)]
    enum AuthPermissionRole {
        Table,
        RoleId,
        PermissionId,
    }
}
