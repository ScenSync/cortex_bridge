//! Migration to update device_status enum with new approval workflow statuses

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the existing enum and recreate it with new values
        // This is necessary because MySQL doesn't support adding enum values in the middle
        manager
            .alter_table(
                Table::alter()
                    .table(Devices::Table)
                    .modify_column(
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
                                    Alias::new("network_error")
                                ],
                            )
                            .not_null()
                            .default("pending"),
                    )
                    .to_owned(),
            )
            .await?;

        // Update existing records to map legacy statuses to approved workflow
        let update_stmt = Query::update()
            .table(Devices::Table)
            .value(Devices::Status, "approved")
            .and_where(Expr::col(Devices::Status).is_in(["available", "busy", "maintenance", "connecting"]))
            .to_owned();
        
        manager.exec_stmt(update_stmt).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert to original enum
        manager
            .alter_table(
                Table::alter()
                    .table(Devices::Table)
                    .modify_column(
                        ColumnDef::new(Devices::Status)
                            .enumeration(
                                Alias::new("device_status"),
                                [
                                    Alias::new("available"), 
                                    Alias::new("busy"), 
                                    Alias::new("maintenance"), 
                                    Alias::new("offline"), 
                                    Alias::new("connecting"), 
                                    Alias::new("network_error")
                                ],
                            )
                            .not_null()
                            .default("offline"),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Devices {
    Table,
    Status,
}
