//! Compose Fortify auth schema + HN tables.

use sova::{AuthMigrator, MigrationTrait, MigratorTrait};

pub struct HnMigrator;

#[async_trait::async_trait]
impl MigratorTrait for HnMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut v = AuthMigrator::migrations();
        v.push(Box::new(m20260809_hn_tables::Migration));
        v
    }
}

mod m20260809_hn_tables {
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260809_hn_tables"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Stories::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Stories::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Stories::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Stories::Title).string_len(300).not_null())
                        .col(ColumnDef::new(Stories::Url).string_len(2000).null())
                        .col(ColumnDef::new(Stories::Text).text().null())
                        .col(
                            ColumnDef::new(Stories::Points)
                                .integer()
                                .not_null()
                                .default(1),
                        )
                        .col(
                            ColumnDef::new(Stories::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Votes::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Votes::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Votes::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Votes::StoryId).big_integer().not_null())
                        .col(
                            ColumnDef::new(Votes::CreatedAt)
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
                        .name("idx_votes_user_story")
                        .table(Votes::Table)
                        .col(Votes::UserId)
                        .col(Votes::StoryId)
                        .unique()
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Comments::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Comments::Id)
                                .big_integer()
                                .not_null()
                                .auto_increment()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Comments::UserId).big_integer().not_null())
                        .col(ColumnDef::new(Comments::StoryId).big_integer().not_null())
                        .col(ColumnDef::new(Comments::Body).text().not_null())
                        .col(
                            ColumnDef::new(Comments::CreatedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default(Expr::current_timestamp()),
                        )
                        .to_owned(),
                )
                .await?;
            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Comments::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(Votes::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(Stories::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(Iden)]
    enum Stories {
        Table,
        Id,
        UserId,
        Title,
        Url,
        Text,
        Points,
        CreatedAt,
    }

    #[derive(Iden)]
    enum Votes {
        Table,
        Id,
        UserId,
        StoryId,
        CreatedAt,
    }

    #[derive(Iden)]
    enum Comments {
        Table,
        Id,
        UserId,
        StoryId,
        Body,
        CreatedAt,
    }
}
