//! Database entities for easytier-bridge
//!
//! These entities are designed to be compatible with cortex-core models
//! while providing the necessary functionality for EasyTier client management.
//!
//! Note: client_sessions are maintained only in memory, not in database.
//! Note: network_configs are now merged into devices table.

pub mod devices;
pub mod organizations;

// Use specific imports to avoid naming conflicts
pub use devices::{
    ActiveModel as DeviceActiveModel, Column as DeviceColumn, Entity as Devices,
    Model as DeviceModel, NetworkConfigInfo,
};
pub use organizations::{
    ActiveModel as OrganizationActiveModel, Column as OrganizationColumn, Entity as Organizations,
    Model as OrganizationModel,
};
