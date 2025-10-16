use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add virtual_ipv4 and virtual_ipv6 columns to devices table
        manager
            .alter_table(
                Table::alter()
                    .table(Devices::Table)
                    .add_column(
                        ColumnDef::new(Devices::VirtualIpv4)
                            .string()
                            .null()
                    )
                    .add_column(
                        ColumnDef::new(Devices::VirtualIpv6)
                            .string()
                            .null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove virtual_ipv4 and virtual_ipv6 columns from devices table
        manager
            .alter_table(
                Table::alter()
                    .table(Devices::Table)
                    .drop_column(Devices::VirtualIpv4)
                    .drop_column(Devices::VirtualIpv6)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Devices {
    Table,
    VirtualIpv4,
    VirtualIpv6,
}