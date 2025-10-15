//! Database module for easytier-bridge
//!
//! This module provides MySQL-based storage for client management,
//! reusing models from cortex_server where possible.

pub mod connection;
pub mod entities;
pub mod migrations;

use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use uuid::Uuid;

/// User ID type compatible with cortex_server
pub type OrgIdInDb = String;

/// Database connection wrapper
#[derive(Debug, Clone)]
pub struct Database {
    /// SeaORM database connection
    pub orm_conn: Arc<DatabaseConnection>,
}

impl Database {
    /// Create a new database instance
    pub async fn new(database_url: &str) -> Result<Self, DbErr> {
        let orm_conn = connection::establish_connection(database_url).await?;

        Ok(Self {
            orm_conn: Arc::new(orm_conn),
        })
    }

    /// Create a new database instance optimized for testing
    pub async fn new_for_test(database_url: &str) -> Result<Self, DbErr> {
        let orm_conn = connection::establish_test_connection(database_url).await?;

        Ok(Self {
            orm_conn: Arc::new(orm_conn),
        })
    }

    /// Get the SeaORM connection
    pub fn orm(&self) -> &DatabaseConnection {
        &self.orm_conn
    }
}

/// Client session storage token
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageToken {
    pub token: String,
    pub client_url: url::Url,
    pub machine_id: Uuid,
    pub user_id: OrgIdInDb,
}

/// Location information for geo-IP lookup
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub country: String,
    pub city: Option<String>,
    pub region: Option<String>,
}

/// Network configuration properties
pub enum ListNetworkProps {
    All,
    EnabledOnly,
    DisabledOnly,
}
