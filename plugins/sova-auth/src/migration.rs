//! Fortify AuthMigrator: passport tables + password-reset + RBAC (single init step).

use sea_orm_migration::prelude::*;

/// Compose passport + fortify migrations for `Db::migrations::<AuthMigrator>()`.
pub struct AuthMigrator;

#[async_trait::async_trait]
impl MigratorTrait for AuthMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut v = sova_passport::AuthMigrator::migrations();
        v.push(Box::new(m20260308_000002_fortify::Migration));
        v
    }
}

mod m20260308_000002_fortify {
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260308_000002_fortify"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_role_user_user")
                                .from(AuthRoleUser::Table, AuthRoleUser::UserId)
                                .to(AuthUsers::Table, AuthUsers::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_role_user_role")
                                .from(AuthRoleUser::Table, AuthRoleUser::RoleId)
                                .to(AuthRoles::Table, AuthRoles::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_auth_role_user_role_id")
                        .table(AuthRoleUser::Table)
                        .col(AuthRoleUser::RoleId)
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
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_permission_role_role")
                                .from(AuthPermissionRole::Table, AuthPermissionRole::RoleId)
                                .to(AuthRoles::Table, AuthRoles::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk_auth_permission_role_perm")
                                .from(AuthPermissionRole::Table, AuthPermissionRole::PermissionId)
                                .to(AuthPermissions::Table, AuthPermissions::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_auth_permission_role_permission_id")
                        .table(AuthPermissionRole::Table)
                        .col(AuthPermissionRole::PermissionId)
                        .to_owned(),
                )
                .await?;

            let conn = manager.get_connection();
            conn.execute_unprepared(
                "INSERT INTO auth_roles (name, slug) VALUES ('User', 'user'), ('Admin', 'admin')",
            )
            .await?;
            conn.execute_unprepared(
                "INSERT INTO auth_permissions (name, slug) VALUES \
                 ('Cabinet access', 'cabinet.access'), \
                 ('Manage users', 'users.manage')",
            )
            .await?;
            conn.execute_unprepared(
                "INSERT INTO auth_permission_role (role_id, permission_id) \
                 SELECT r.id, p.id FROM auth_roles r, auth_permissions p \
                 WHERE r.slug = 'user' AND p.slug = 'cabinet.access'",
            )
            .await?;
            conn.execute_unprepared(
                "INSERT INTO auth_permission_role (role_id, permission_id) \
                 SELECT r.id, p.id FROM auth_roles r, auth_permissions p \
                 WHERE r.slug = 'admin'",
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
    enum AuthUsers {
        Table,
        Id,
    }

    #[derive(Iden)]
    enum AuthPasswordResetTokens {
        Table,
        Email,
        TokenHash,
        CreatedAt,
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
