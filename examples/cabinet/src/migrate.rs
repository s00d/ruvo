//! Cabinet migrator: Fortify auth + activity + notifications + notes.

use sova::{
    ActivityMigrator, AuthMigrator, MigrationTrait, MigratorTrait, NotificationsMigrator,
};

pub struct CabinetMigrator;

#[async_trait::async_trait]
impl MigratorTrait for CabinetMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut v = AuthMigrator::migrations();
        v.extend(ActivityMigrator::migrations());
        v.extend(NotificationsMigrator::migrations());
        v.push(Box::new(m20260807_000001_notes::Migration));
        v
    }
}

mod m20260807_000001_notes {
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260807_000001_notes"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Notes::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Notes::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Notes::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Notes::Title).string().not_null())
                        .col(ColumnDef::new(Notes::Body).text().not_null())
                        .col(
                            ColumnDef::new(Notes::CreatedAt)
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
                        .name("idx_notes_user_id")
                        .table(Notes::Table)
                        .col(Notes::UserId)
                        .to_owned(),
                )
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Notes::Table).to_owned())
                .await
        }
    }

    #[derive(Iden)]
    enum Notes {
        Table,
        Id,
        UserId,
        Title,
        Body,
        CreatedAt,
    }
}
