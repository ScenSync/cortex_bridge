//! Migration to drop user_running_network_configs table

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the user_running_network_configs table if it exists
        // Use raw SQL to handle the case where the table doesn't exist
        let sql = "DROP TABLE IF EXISTS user_running_network_configs";
        manager.get_connection().execute_unprepared(sql).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Recreate the user_running_network_configs table
        manager
            .create_table(
                Table::create()
                    .table(UserRunningNetworkConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::OrganizationId)
                            .char_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::DeviceId)
                            .char_len(36)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::NetworkInstanceId)
                            .char_len(36)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::NetworkConfig)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::Disabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::CreateTime)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UserRunningNetworkConfigs::UpdateTime)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp())
                            .extra("ON UPDATE CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_network_configs_organization_id")
                            .from(
                                UserRunningNetworkConfigs::Table,
                                UserRunningNetworkConfigs::OrganizationId,
                            )
                            .to(Organizations::Table, Organizations::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_network_configs_organization_id")
                            .col(UserRunningNetworkConfigs::OrganizationId),
                    )
                    .index(
                        Index::create()
                            .name("idx_network_configs_device_id")
                            .col(UserRunningNetworkConfigs::DeviceId),
                    )
                    .index(
                        Index::create()
                            .name("idx_network_configs_organization_device")
                            .col(UserRunningNetworkConfigs::OrganizationId)
                            .col(UserRunningNetworkConfigs::DeviceId),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum UserRunningNetworkConfigs {
    Table,
    Id,
    OrganizationId,
    DeviceId,
    NetworkInstanceId,
    NetworkConfig,
    Disabled,
    CreateTime,
    UpdateTime,
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
}