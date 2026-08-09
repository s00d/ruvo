//! `activity_log` schema.

use sea_orm_migration::prelude::*;

pub struct ActivityMigrator;

#[async_trait::async_trait]
impl MigratorTrait for ActivityMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260308_000001_activity_log::Migration)]
    }
}

mod m20260308_000001_activity_log {
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260308_000001_activity_log"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(ActivityLog::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(ActivityLog::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(ActivityLog::ActorId).big_integer().null())
                        .col(ColumnDef::new(ActivityLog::SubjectType).string().not_null())
                        .col(ColumnDef::new(ActivityLog::SubjectId).string().not_null())
                        .col(ColumnDef::new(ActivityLog::Event).string().not_null())
                        .col(
                            ColumnDef::new(ActivityLog::Properties)
                                .text()
                                .not_null()
                                .default("{}"),
                        )
                        .col(ColumnDef::new(ActivityLog::Ip).string().null())
                        .col(ColumnDef::new(ActivityLog::UserAgent).text().null())
                        .col(
                            ColumnDef::new(ActivityLog::CreatedAt)
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
                        .name("idx_activity_subject")
                        .table(ActivityLog::Table)
                        .col(ActivityLog::SubjectType)
                        .col(ActivityLog::SubjectId)
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_activity_actor")
                        .table(ActivityLog::Table)
                        .col(ActivityLog::ActorId)
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_activity_event")
                        .table(ActivityLog::Table)
                        .col(ActivityLog::Event)
                        .to_owned(),
                )
                .await?;
            manager
                .create_index(
                    Index::create()
                        .if_not_exists()
                        .name("idx_activity_created")
                        .table(ActivityLog::Table)
                        .col(ActivityLog::CreatedAt)
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(ActivityLog::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum ActivityLog {
        Table,
        Id,
        ActorId,
        SubjectType,
        SubjectId,
        Event,
        Properties,
        Ip,
        UserAgent,
        CreatedAt,
    }
}
