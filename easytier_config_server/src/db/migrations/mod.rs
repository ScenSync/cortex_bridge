//! Database migrations for easytier_config_server

use sea_orm_migration::prelude::*;

pub mod m20240101_000002_create_devices_table;
pub mod m20240101_000005_create_organizations_table;
pub mod m20240101_000007_drop_network_configs_table;
pub mod m20240101_000008_update_device_status_enum;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000002_create_devices_table::Migration),
            Box::new(m20240101_000005_create_organizations_table::Migration),
            Box::new(m20240101_000007_drop_network_configs_table::Migration),
            Box::new(m20240101_000008_update_device_status_enum::Migration),
        ]
    }
}
