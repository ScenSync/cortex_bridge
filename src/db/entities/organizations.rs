//! Organization entity compatible with cortex-core Organization model

use sea_orm::entity::prelude::*;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Organization status enumeration
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "organization_status"
)]
pub enum OrganizationStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "suspended")]
    Suspended,
}

/// Organization entity - compatible with cortex-core Organization model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organizations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Char(Some(36))")]
    pub id: String,

    #[sea_orm(column_type = "Text")]
    pub name: String,

    #[sea_orm(column_type = "Text", nullable)]
    pub code: Option<String>,

    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    #[sea_orm(column_type = "Json", nullable)]
    pub contact_info: Option<serde_json::Value>,

    #[sea_orm(default_value = "active")]
    pub status: OrganizationStatus,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::devices::Entity")]
    Devices,
}

impl Related<super::devices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Devices.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::new_v4().to_string()),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        _insert: bool,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Self, DbErr>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        C: ConnectionTrait + 'life0,
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            self.updated_at =
                Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));
            Ok(self)
        })
    }
}

impl Model {
    /// Get organization ID as UUID
    pub fn id_uuid(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.id)
    }

    /// Check if organization is active
    pub fn is_active(&self) -> bool {
        self.status == OrganizationStatus::Active
    }

    /// Parse contact info from JSON
    pub fn contact_info_parsed<T>(&self) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.contact_info
            .as_ref()
            .and_then(|info| serde_json::from_value(info.clone()).ok())
    }
}
