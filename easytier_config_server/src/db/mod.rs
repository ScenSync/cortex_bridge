//! Database module for easytier_config_server
//!
//! This module provides MySQL-based storage for client management.

pub mod connection;
pub mod entities;
pub mod migrations;

use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;

/// Organization ID type (String UUID)
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
