//! Device entity compatible with cortex_server Device model
//! Network configuration fields are stored directly in the devices table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Device type enumeration
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "device_type")]
pub enum DeviceType {
    #[sea_orm(string_value = "robot")]
    Robot,
    #[sea_orm(string_value = "edge")]
    Edge,
}

/// Device status enumeration
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "device_status")]
pub enum DeviceStatus {
    // Registration states
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "rejected")]
    Rejected,

    // Operational states
    #[sea_orm(string_value = "online")]
    Online,
    #[sea_orm(string_value = "offline")]
    Offline,
    #[sea_orm(string_value = "busy")]
    Busy,

    // Administrative states
    #[sea_orm(string_value = "maintenance")]
    Maintenance,
    #[sea_orm(string_value = "disabled")]
    Disabled,
}

impl DeviceStatus {
    /// Check if device is approved (can participate in networks)
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            DeviceStatus::Online
                | DeviceStatus::Offline
                | DeviceStatus::Busy
                | DeviceStatus::Maintenance
        )
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, DeviceStatus::Pending)
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, DeviceStatus::Rejected)
    }

    pub fn is_online(&self) -> bool {
        matches!(self, DeviceStatus::Online | DeviceStatus::Busy)
    }
}

/// Device entity - Stores device information and network configuration
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

    // Robot-specific fields
    #[sea_orm(column_type = "Char(Some(36))", nullable)]
    pub robot_type_id: Option<String>,

    // Network configuration (ONE network per device)
    #[sea_orm(unique, column_type = "Char(Some(36))", nullable)]
    pub network_instance_id: Option<String>,

    #[sea_orm(column_type = "Json", nullable)]
    pub network_config: Option<serde_json::Value>,

    #[sea_orm(nullable)]
    pub network_disabled: Option<bool>,

    #[sea_orm(nullable)]
    pub network_create_time: Option<DateTimeWithTimeZone>,

    #[sea_orm(nullable)]
    pub network_update_time: Option<DateTimeWithTimeZone>,

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
    pub fn is_robot(&self) -> bool {
        self.device_type == DeviceType::Robot
    }

    pub fn is_edge(&self) -> bool {
        self.device_type == DeviceType::Edge
    }
}
