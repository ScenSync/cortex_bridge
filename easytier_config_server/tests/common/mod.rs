//! Test database utilities for concurrent testing
//!
//! This module provides utilities to create isolated test databases
//! for each test function, enabling true concurrent testing.
//! 包含自动清理机制，确保测试数据库不会残留

use easytier_config_server::db::Database;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Base database URL without database name
#[allow(dead_code)]
const BASE_DB_URL: &str = "mysql://root:root123@127.0.0.1:3306";

/// Database connection cache for test databases
#[allow(dead_code)]
static DB_CACHE: once_cell::sync::Lazy<Arc<Mutex<std::collections::HashMap<String, Database>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(std::collections::HashMap::new())));

/// 跟踪已创建的测试数据库，用于清理
#[allow(dead_code)]
static CREATED_DBS: once_cell::sync::Lazy<Arc<Mutex<HashSet<String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

/// Create a unique database name for a test function
#[allow(dead_code)]
pub fn create_test_db_name(test_function_name: &str) -> String {
    format!("cortex_{}", test_function_name)
}

/// Get the database URL for a test function
#[allow(dead_code)]
pub fn get_test_database_url(test_function_name: &str) -> String {
    let db_name = create_test_db_name(test_function_name);
    format!("{}/{}", BASE_DB_URL, db_name)
}

/// Create or get a test database for a specific test function
#[allow(dead_code)]
pub async fn get_test_database(
    test_function_name: &str,
) -> Result<Database, Box<dyn std::error::Error>> {
    let db_name = create_test_db_name(test_function_name);
    let db_url = get_test_database_url(test_function_name);

    // Check cache first
    {
        let cache = DB_CACHE.lock().await;
        if let Some(db) = cache.get(&db_name) {
            return Ok(db.clone());
        }
    }

    // Create database if it doesn't exist
    create_database_if_not_exists(&db_name).await?;

    // 记录创建的数据库，用于后续清理
    {
        let mut created_dbs = CREATED_DBS.lock().await;
        created_dbs.insert(db_name.clone());
    }

    // Create database connection with test-optimized pool configuration
    let db = Database::new_for_test(&db_url)
        .await
        .map_err(|e| format!("Failed to connect to test database {}: {}", db_name, e))?;

    // Create tables
    create_tables_if_not_exist(&db).await?;

    // Cache the database connection
    {
        let mut cache = DB_CACHE.lock().await;
        cache.insert(db_name, db.clone());
    }

    Ok(db)
}

/// Create database if it doesn't exist
#[allow(dead_code)]
async fn create_database_if_not_exists(db_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm::{ConnectionTrait, Database as SeaOrmDatabase, DatabaseBackend, Statement};

    let conn = SeaOrmDatabase::connect(BASE_DB_URL)
        .await
        .map_err(|e| format!("Failed to connect to MySQL server: {}", e))?;

    // Create database if not exists
    let create_db_query = format!(
        "CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        db_name
    );
    conn.execute(Statement::from_string(
        DatabaseBackend::MySql,
        create_db_query,
    ))
    .await
    .map_err(|e| format!("Failed to create database {}: {}", db_name, e))?;

    conn.close()
        .await
        .map_err(|e| format!("Failed to close connection: {}", e))?;
    Ok(())
}

/// Create all required tables using SeaORM migrations
#[allow(dead_code)]
async fn create_tables_if_not_exist(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    use easytier_config_server::db::migrations::Migrator;
    use sea_orm_migration::MigratorTrait;

    // Run all migrations to ensure tables are created with correct schema
    Migrator::up(db.orm(), None)
        .await
        .map_err(|e| format!("Failed to run migrations: {}", e))?;

    Ok(())
}

/// Clean up test database by deleting all data from tables
#[allow(dead_code)]
pub async fn cleanup_test_database(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let cleanup_queries = [
        "SET FOREIGN_KEY_CHECKS = 0",
        "DELETE FROM devices",
        "DELETE FROM organizations",
        "SET FOREIGN_KEY_CHECKS = 1",
    ];

    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    for query in &cleanup_queries {
        db.orm()
            .execute(Statement::from_string(
                DatabaseBackend::MySql,
                query.to_string(),
            ))
            .await
            .map_err(|e| format!("Failed to cleanup table: {}", e))?;
    }

    Ok(())
}

/// Generate a test organization ID
#[allow(dead_code)]
pub fn test_organization_id() -> String {
    Uuid::new_v4().to_string()
}

/// Create a test organization in the database
/// This function ensures that the organization record exists before other operations
#[allow(dead_code)]
pub async fn create_test_organization(
    db: &Database,
    org_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let insert_org_query = format!(
        "INSERT IGNORE INTO organizations (id, name, created_at, updated_at) VALUES ('{}', 'Test Organization', NOW(), NOW())",
        org_id
    );

    db.orm()
        .execute(Statement::from_string(
            DatabaseBackend::MySql,
            insert_org_query,
        ))
        .await
        .map_err(|e| format!("Failed to create test organization: {}", e))?;

    Ok(())
}

/// Create a test organization and return its ID
/// This is a convenience function that generates an ID and creates the organization
#[allow(dead_code)]
pub async fn setup_test_organization(db: &Database) -> Result<String, Box<dyn std::error::Error>> {
    let org_id = test_organization_id();
    create_test_organization(db, &org_id).await?;
    Ok(org_id)
}

/// Generate a test device ID
#[allow(dead_code)]
pub fn test_device_id() -> uuid::Uuid {
    Uuid::new_v4()
}

/// Generate test client URL
#[allow(dead_code)]
pub fn test_client_url() -> url::Url {
    url::Url::parse("tcp://127.0.0.1:8080").unwrap()
}

/// Generate test location JSON
#[allow(dead_code)]
pub fn test_location_json() -> serde_json::Value {
    serde_json::json!({
        "country": "中国",
        "city": "北京",
        "region": "北京市"
    })
}

/// Macro to setup test database for a test function
#[allow(unused_macros)]
macro_rules! setup_test_db {
    () => {{
        let test_name = std::thread::current()
            .name()
            .unwrap_or("unknown_test")
            .split("::")
            .last()
            .unwrap_or("unknown_test")
            .to_string();
        let db = $crate::common::get_test_database(&test_name)
            .await
            .expect("Failed to setup test database");
        $crate::common::cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");
        db
    }};
}

/// Macro to setup test database with a test organization
#[allow(unused_macros)]
macro_rules! setup_test_db_with_org {
    () => {{
        let test_name = std::thread::current()
            .name()
            .unwrap_or("unknown_test")
            .split("::")
            .last()
            .unwrap_or("unknown_test")
            .to_string();
        let db = $crate::common::get_test_database(&test_name)
            .await
            .expect("Failed to setup test database");
        $crate::common::cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");
        let org_id = $crate::common::setup_test_organization(&db)
            .await
            .expect("Failed to create test organization");
        (db, org_id)
    }};
}

/// 删除指定的测试数据库
/// 这个函数应该在测试结束时调用，确保测试数据库被正确删除
#[allow(dead_code)]
pub async fn remove_test_database(
    test_function_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm::{ConnectionTrait, Database as SeaOrmDatabase, DatabaseBackend, Statement};

    let db_name = create_test_db_name(test_function_name);

    // 从缓存中移除数据库连接
    {
        let mut cache = DB_CACHE.lock().await;
        cache.remove(&db_name);
    }

    // 连接到MySQL服务器
    let conn = SeaOrmDatabase::connect(BASE_DB_URL)
        .await
        .map_err(|e| format!("Failed to connect to MySQL server: {}", e))?;

    // 删除数据库
    println!("Dropping test database: {}", db_name);
    let drop_query = format!("DROP DATABASE IF EXISTS `{}`", db_name);

    match conn
        .execute(Statement::from_string(DatabaseBackend::MySql, drop_query))
        .await
    {
        Ok(_) => {
            // 从跟踪列表中移除
            let mut dbs = CREATED_DBS.lock().await;
            dbs.remove(&db_name);
            println!("Successfully dropped {}", db_name);
        }
        Err(e) => println!("Failed to drop {}: {}", db_name, e),
    }

    conn.close()
        .await
        .map_err(|e| format!("Failed to close connection: {}", e))?;

    Ok(())
}

/// 删除所有测试数据库
/// 这个函数用于清理所有已创建的测试数据库
#[allow(dead_code)]
pub async fn drop_all_test_databases() -> Result<(), Box<dyn std::error::Error>> {
    use sea_orm::{ConnectionTrait, Database as SeaOrmDatabase, DatabaseBackend, Statement};

    // 获取所有已创建的测试数据库
    let created_dbs = {
        let dbs = CREATED_DBS.lock().await;
        dbs.clone()
    };

    if created_dbs.is_empty() {
        println!("No test databases to clean up");
        return Ok(());
    }

    // 连接到MySQL服务器
    let conn = SeaOrmDatabase::connect(BASE_DB_URL)
        .await
        .map_err(|e| format!("Failed to connect to MySQL server: {}", e))?;

    // 删除所有已创建的测试数据库
    for db_name in created_dbs {
        println!("Dropping test database: {}", db_name);
        let drop_query = format!("DROP DATABASE IF EXISTS `{}`", db_name);

        match conn
            .execute(Statement::from_string(DatabaseBackend::MySql, drop_query))
            .await
        {
            Ok(_) => println!("Successfully dropped {}", db_name),
            Err(e) => println!("Failed to drop {}: {}", db_name, e),
        }
    }

    // 清空跟踪列表
    {
        let mut dbs = CREATED_DBS.lock().await;
        dbs.clear();
    }

    // 清空缓存
    {
        let mut cache = DB_CACHE.lock().await;
        cache.clear();
    }

    conn.close()
        .await
        .map_err(|e| format!("Failed to close connection: {}", e))?;

    Ok(())
}
