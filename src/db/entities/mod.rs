//! Database entities for easytier-bridge
//! 
//! These entities are designed to be compatible with cortex-core models
//! while providing the necessary functionality for EasyTier client management.
//! 
//! Note: client_sessions are maintained only in memory, not in database.
//! Note: network_configs are now merged into devices table.

pub mod organizations;
pub mod devices;

// Use specific imports to avoid naming conflicts
pub use organizations::{Entity as Organizations, Model as OrganizationModel, ActiveModel as OrganizationActiveModel, Column as OrganizationColumn};
pub use devices::{Entity as Devices, Model as DeviceModel, ActiveModel as DeviceActiveModel, Column as DeviceColumn, NetworkConfigInfo};