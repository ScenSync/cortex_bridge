//! Database entities for easytier_config_server

pub mod devices;
pub mod organizations;

// Re-exports for convenience
pub use devices::{
    ActiveModel as DeviceActiveModel, Column as DeviceColumn, DeviceStatus, DeviceType,
    Entity as Devices, Model as DeviceModel,
};

pub use organizations::{
    ActiveModel as OrganizationActiveModel, Column as OrganizationColumn, Entity as Organizations,
    Model as OrganizationModel, OrganizationStatus,
};
