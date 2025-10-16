//! Database connection management

use crate::{error, info};
use sea_orm::{ConnectionTrait, Database as SeaOrmDatabase, DatabaseConnection, DbErr};

/// Establish SeaORM database connection
pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    info!("Connecting to MySQL database with SeaORM...");

    // Create SeaORM connection
    let orm_conn = SeaOrmDatabase::connect(database_url).await.map_err(|e| {
        error!("Failed to create SeaORM connection: {}", e);
        e
    })?;

    info!("Successfully connected to MySQL database");

    Ok(orm_conn)
}

/// Establish SeaORM database connection optimized for testing
pub async fn establish_test_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    info!("Connecting to test MySQL database with SeaORM...");

    // Create SeaORM connection
    let orm_conn = SeaOrmDatabase::connect(database_url).await.map_err(|e| {
        error!("Failed to create test SeaORM connection: {}", e);
        e
    })?;

    info!("Successfully connected to test MySQL database");

    Ok(orm_conn)
}

/// Test database connection using SeaORM
pub async fn test_connection(database_url: &str) -> Result<(), DbErr> {
    let conn = SeaOrmDatabase::connect(database_url).await.map_err(|e| {
        error!("Connection test failed: {}", e);
        e
    })?;

    // Test with a simple query using SeaORM
    use sea_orm::Statement;
    conn.execute(Statement::from_string(
        sea_orm::DatabaseBackend::MySql,
        "SELECT 1".to_owned(),
    ))
    .await
    .map_err(|e| {
        error!("Query test failed: {}", e);
        e
    })?;

    conn.close().await.map_err(|e| {
        error!("Failed to close connection: {}", e);
        e
    })?;

    info!("Database connection test successful");
    Ok(())
}
