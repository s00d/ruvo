//! Scope schema migrations.

use sea_orm_migration::prelude::*;

pub struct ScopeMigrator;

#[async_trait::async_trait]
impl MigratorTrait for ScopeMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut v = vec![Box::new(m20260811_scope_memberships::Migration) as Box<dyn MigrationTrait>];
        #[cfg(feature = "invites")]
        v.push(Box::new(m20260811_scope_invitations::Migration));
        v.push(Box::new(m20260811_scope_membership_audit::Migration));
        v
    }
}

mod m20260811_scope_memberships {
    use super::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260811_scope_memberships"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(ScopeMemberships::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(ScopeMemberships::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(ScopeMemberships::ScopeKind)
                                .string_len(64)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMemberships::ScopeId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMemberships::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMemberships::Role)
                                .string_len(32)
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_scope_memberships_unique")
                        .table(ScopeMemberships::Table)
                        .col(ScopeMemberships::ScopeKind)
                        .col(ScopeMemberships::ScopeId)
                        .col(ScopeMemberships::UserId)
                        .unique()
                        .to_owned(),
                )
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(ScopeMemberships::Table).to_owned())
                .await
        }
    }

    #[derive(Iden)]
    enum ScopeMemberships {
        Table,
        Id,
        ScopeKind,
        ScopeId,
        UserId,
        Role,
    }
}

#[cfg(feature = "invites")]
mod m20260811_scope_invitations {
    use super::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260811_scope_invitations"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(ScopeInvitations::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(ScopeInvitations::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::ScopeKind)
                                .string_len(64)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::ScopeId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::Email)
                                .string_len(320)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::Role)
                                .string_len(32)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::InvitedBy)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::Token)
                                .string_len(64)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::Status)
                                .string_len(16)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeInvitations::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_scope_invite_scope_email")
                        .table(ScopeInvitations::Table)
                        .col(ScopeInvitations::ScopeKind)
                        .col(ScopeInvitations::ScopeId)
                        .col(ScopeInvitations::Email)
                        .unique()
                        .to_owned(),
                )
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(ScopeInvitations::Table).to_owned())
                .await
        }
    }

    #[derive(Iden)]
    enum ScopeInvitations {
        Table,
        Id,
        ScopeKind,
        ScopeId,
        Email,
        Role,
        InvitedBy,
        Token,
        Status,
        CreatedAt,
    }
}

mod m20260811_scope_membership_audit {
    use super::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260811_scope_membership_audit"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(ScopeMembershipAudit::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::ScopeKind)
                                .string_len(64)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::ScopeId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::ActorId)
                                .big_integer()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::Action)
                                .string_len(32)
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::OldRole)
                                .string_len(32)
                                .null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::NewRole)
                                .string_len(32)
                                .null(),
                        )
                        .col(
                            ColumnDef::new(ScopeMembershipAudit::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_scope_membership_audit_scope")
                        .table(ScopeMembershipAudit::Table)
                        .col(ScopeMembershipAudit::ScopeKind)
                        .col(ScopeMembershipAudit::ScopeId)
                        .col(ScopeMembershipAudit::CreatedAt)
                        .to_owned(),
                )
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(ScopeMembershipAudit::Table).to_owned())
                .await
        }
    }

    #[derive(Iden)]
    enum ScopeMembershipAudit {
        Table,
        Id,
        ScopeKind,
        ScopeId,
        UserId,
        ActorId,
        Action,
        OldRole,
        NewRole,
        CreatedAt,
    }
}
