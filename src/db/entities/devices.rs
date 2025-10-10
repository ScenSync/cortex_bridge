//! Device entity compatible with cortex-core Device model

use sea_orm::entity::prelude::*;
// use sea_orm::Set; // Unused import
use serde::{Deserialize, Serialize};
// use uuid::Uuid; // Unused import

/// Device type enumeration - compatible with cortex-core
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "device_type")]
pub enum DeviceType {
    #[sea_orm(string_value = "robot")]
    Robot,
    #[sea_orm(string_value = "edge")]
    Edge,
}

/// Device status enumeration - Simplified to 7 statuses (compatible with cortex-core)
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "device_status")]
pub enum DeviceStatus {
    // Registration states
    #[sea_orm(string_value = "pending")]
    Pending, // Awaiting admin approval
    #[sea_orm(string_value = "rejected")]
    Rejected, // Admin rejected registration

    // Operational states (post-approval, based on connection)
    #[sea_orm(string_value = "online")]
    Online, // Approved + Connected + Available workstation
    #[sea_orm(string_value = "offline")]
    Offline, // Approved but not connected (no heartbeat)
    #[sea_orm(string_value = "busy")]
    Busy, // Data collector logged in (workstation occupied)

    // Administrative states
    #[sea_orm(string_value = "maintenance")]
    Maintenance, // Under maintenance
    #[sea_orm(string_value = "disabled")]
    Disabled, // Administratively disabled/decommissioned
}

impl DeviceStatus {
    /// Check if device is approved (can participate in networks)
    /// Includes all post-approval statuses: online, offline, busy, maintenance
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            DeviceStatus::Online
                | DeviceStatus::Offline
                | DeviceStatus::Busy
                | DeviceStatus::Maintenance
        )
    }

    /// Check if device is pending approval
    pub fn is_pending(&self) -> bool {
        matches!(self, DeviceStatus::Pending)
    }

    /// Check if device is rejected
    pub fn is_rejected(&self) -> bool {
        matches!(self, DeviceStatus::Rejected)
    }

    /// Check if device is online (based on status)
    pub fn is_online(&self) -> bool {
        matches!(self, DeviceStatus::Online | DeviceStatus::Busy)
    }
}

/// Device entity - compatible with cortex-core Device model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "devices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Char(Some(36))")]
    pub id: String,

    #[sea_orm(column_type = "Text")]
    pub name: String,

    #[sea_orm(unique, column_type = "Text")]
    pub serial_number: String,

    pub device_type: DeviceType,

    #[sea_orm(column_type = "Text", nullable)]
    pub model: Option<String>,

    #[sea_orm(default_value = "pending")]
    pub status: DeviceStatus,

    #[sea_orm(column_type = "Json", nullable)]
    pub capabilities: Option<serde_json::Value>,

    #[sea_orm(column_type = "Char(Some(36))", nullable)]
    pub organization_id: Option<String>,

    #[sea_orm(nullable)]
    pub scenario_id: Option<u32>,

    pub last_heartbeat: Option<DateTimeWithTimeZone>,

    // Robot-specific fields (only when device_type is robot)
    #[sea_orm(column_type = "Char(Some(36))", nullable)]
    pub robot_type_id: Option<String>,

    // Network configuration fields (merged from user_running_network_configs)
    #[sea_orm(unique, column_type = "Char(Some(36))", nullable)]
    pub network_instance_id: Option<String>,

    #[sea_orm(column_type = "Json", nullable)]
    pub network_config: Option<serde_json::Value>,

    #[sea_orm(default_value = false, nullable)]
    pub network_disabled: Option<bool>,

    #[sea_orm(nullable)]
    pub network_create_time: Option<DateTimeWithTimeZone>,

    #[sea_orm(nullable)]
    pub network_update_time: Option<DateTimeWithTimeZone>,

    // Virtual IP fields (extracted from network info)
    #[sea_orm(nullable)]
    pub virtual_ip: Option<u32>,

    #[sea_orm(nullable)]
    pub virtual_ip_network_length: Option<u8>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Organizations,
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Check if device is a robot
    pub fn is_robot(&self) -> bool {
        self.device_type == DeviceType::Robot
    }

    /// Check if device is an edge device
    pub fn is_edge(&self) -> bool {
        self.device_type == DeviceType::Edge
    }

    /// Check if device has network configuration
    pub fn has_network_config(&self) -> bool {
        self.network_instance_id.is_some() && self.network_config.is_some()
    }

    /// Get network configuration info
    pub fn get_network_config(&self) -> Option<NetworkConfigInfo> {
        if !self.has_network_config() {
            return None;
        }

        Some(NetworkConfigInfo {
            instance_id: self.network_instance_id.clone().unwrap(),
            config: self.network_config.as_ref().unwrap().to_string(),
            disabled: self.network_disabled.unwrap_or(false),
            create_time: self.network_create_time,
            update_time: self.network_update_time,
        })
    }
}

/// Network configuration info structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfigInfo {
    pub instance_id: String,
    pub config: String,
    pub disabled: bool,
    pub create_time: Option<DateTimeWithTimeZone>,
    pub update_time: Option<DateTimeWithTimeZone>,
}
