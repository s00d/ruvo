//! `notifications` schema.

use sea_orm_migration::prelude::*;

pub struct NotificationsMigrator;

#[async_trait::async_trait]
impl MigratorTrait for NotificationsMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260308_000001_notifications::Migration)]
    }
}

mod m20260308_000001_notifications {
    use sea_orm_migration::prelude::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Notifications::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Notifications::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(Notifications::UserId)
                                .big_integer()
                                .not_null(),
                        )
                        .col(ColumnDef::new(Notifications::Channel).string().not_null())
                        .col(ColumnDef::new(Notifications::Event).string().not_null())
                        .col(ColumnDef::new(Notifications::Title).string().not_null())
                        .col(ColumnDef::new(Notifications::Body).text().null())
                        .col(
                            ColumnDef::new(Notifications::Data)
                                .text()
                                .not_null()
                                .default("{}"),
                        )
                        .col(
                            ColumnDef::new(Notifications::ReadAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .col(
                            ColumnDef::new(Notifications::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_notifications_user_read")
                        .table(Notifications::Table)
                        .col(Notifications::UserId)
                        .col(Notifications::ReadAt)
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_notifications_user_channel")
                        .table(Notifications::Table)
                        .col(Notifications::UserId)
                        .col(Notifications::Channel)
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_notifications_created")
                        .table(Notifications::Table)
                        .col(Notifications::CreatedAt)
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Notifications::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum Notifications {
        Table,
        Id,
        UserId,
        Channel,
        Event,
        Title,
        Body,
        Data,
        ReadAt,
        CreatedAt,
    }
}
