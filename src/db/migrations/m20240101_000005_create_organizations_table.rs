//! Migration to create organizations table compatible with cortex-core

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Organizations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Organizations::Id)
                            .char_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Organizations::Name)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Organizations::Code)
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Organizations::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Organizations::ContactInfo)
                            .json()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Organizations::Status)
                            .string_len(50)
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(Organizations::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Organizations::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp())
                            .extra("ON UPDATE CURRENT_TIMESTAMP".to_string()),
                    )
                    .index(
                        Index::create()
                            .name("idx_organizations_code")
                            .col(Organizations::Code)
                            .unique(),
                    )
                    .index(
                        Index::create()
                            .name("idx_organizations_status")
                            .col(Organizations::Status),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Organizations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
    Name,
    Code,
    Description,
    ContactInfo,
    Status,
    CreatedAt,
    UpdatedAt,
}