//! Migration to create devices table compatible with cortex-core

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Devices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Devices::Id)
                            .char_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Devices::Name).string_len(100).not_null())
                    .col(
                        ColumnDef::new(Devices::SerialNumber)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Devices::DeviceType)
                            .enumeration(
                                Alias::new("device_type"),
                                [Alias::new("robot"), Alias::new("edge")],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Devices::Model).string_len(100).null())
                    .col(
                        ColumnDef::new(Devices::Status)
                            .enumeration(
                                Alias::new("device_status"),
                                [
                                    Alias::new("pending"),
                                    Alias::new("approved"),
                                    Alias::new("rejected"),
                                    Alias::new("available"),
                                    Alias::new("busy"),
                                    Alias::new("maintenance"),
                                    Alias::new("offline"),
                                    Alias::new("connecting"),
                                    Alias::new("network_error"),
                                ],
                            )
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(Devices::Capabilities).json().null())
                    .col(ColumnDef::new(Devices::OrganizationId).char_len(36).null())
                    .col(ColumnDef::new(Devices::ScenarioId).unsigned().null())
                    .col(ColumnDef::new(Devices::LastHeartbeat).timestamp().null())
                    // Robot-specific fields (only when device_type is robot)
                    .col(ColumnDef::new(Devices::RobotTypeId).char_len(36).null())
                    // Network configuration fields (merged from user_running_network_configs)
                    .col(
                        ColumnDef::new(Devices::NetworkInstanceId)
                            .char_len(36)
                            .null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Devices::NetworkConfig).json().null())
                    .col(
                        ColumnDef::new(Devices::NetworkDisabled)
                            .boolean()
                            .null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Devices::NetworkCreateTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Devices::NetworkUpdateTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    // Virtual IP fields (extracted from network info)
                    .col(ColumnDef::new(Devices::VirtualIp).unsigned().null())
                    .col(
                        ColumnDef::new(Devices::VirtualIpNetworkLength)
                            .unsigned()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Devices::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Devices::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp())
                            .extra("ON UPDATE CURRENT_TIMESTAMP".to_string()),
                    )
                    .index(
                        Index::create()
                            .name("idx_devices_organization_id")
                            .col(Devices::OrganizationId),
                    )
                    .index(
                        Index::create()
                            .name("idx_devices_scenario_id")
                            .col(Devices::ScenarioId),
                    )
                    .index(
                        Index::create()
                            .name("idx_devices_robot_type_id")
                            .col(Devices::RobotTypeId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Devices::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Devices {
    Table,
    Id,
    Name,
    SerialNumber,
    DeviceType,
    Model,
    Status,
    Capabilities,
    OrganizationId,
    ScenarioId,
    LastHeartbeat,
    RobotTypeId,
    NetworkInstanceId,
    NetworkConfig,
    NetworkDisabled,
    NetworkCreateTime,
    NetworkUpdateTime,
    VirtualIp,
    VirtualIpNetworkLength,
    CreatedAt,
    UpdatedAt,
}
