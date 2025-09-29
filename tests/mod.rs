//! Integration tests for easytier-bridge
//!
//! This module contains comprehensive integration tests for the easytier-bridge library,
//! covering client management, database operations, and session handling.

pub mod common;
pub mod test_ffi_geoip;
pub mod test_device_status_updates;

// All tests are now included inline in the tests module below

// Include all test modules in this single file to avoid module path issues
mod tests {
    use super::common;
    use common::*;

    // Database tests
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_database_connection() {
        let db = get_test_database("test_database_connection")
            .await
            .expect("Failed to setup test database");
        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");

        // Test basic database connectivity using SeaORM
        use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
        let result = db
            .orm()
            .query_one(Statement::from_string(
                DatabaseBackend::MySql,
                "SELECT 1 as test_value".to_owned(),
            ))
            .await;

        assert!(result.is_ok(), "Database connection should work");
        let row = result.unwrap().unwrap();
        let test_value: i32 = row.try_get("", "test_value").unwrap();
        assert_eq!(test_value, 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_org_operations() {
        let db = get_test_database("test_org_operations")
            .await
            .expect("Failed to setup test database");
        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");

        let org_id = test_organization_id();

        // Create a test organization with required fields using SeaORM
        use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
        let insert_result = db.orm()
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                "INSERT INTO organizations (id, name, code, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, NOW(), NOW())",
                vec![
                    org_id.to_string().into(),
                    "Test Organization".into(),
                    "TEST_ORG".into(),
                    "A test organization for unit testing".into(),
                    "active".into(),
                ]
            ))
            .await;

        assert!(
            insert_result.is_ok(),
            "Organization creation should succeed"
        );

        // Query the created organization
        let query_result = db
            .orm()
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                "SELECT id, name, code, description, status FROM organizations WHERE id = ?",
                vec![org_id.to_string().into()],
            ))
            .await;

        assert!(query_result.is_ok(), "Organization query should succeed");
        let row = query_result.unwrap().unwrap();
        let retrieved_id: String = row.try_get("", "id").unwrap();
        let name: String = row.try_get("", "name").unwrap();
        let code: Option<String> = row.try_get("", "code").ok();
        let description: Option<String> = row.try_get("", "description").ok();
        let status: String = row.try_get("", "status").unwrap();

        assert_eq!(retrieved_id, org_id.to_string());
        assert_eq!(name, "Test Organization");
        assert_eq!(code, Some("TEST_ORG".to_string()));
        assert_eq!(
            description,
            Some("A test organization for unit testing".to_string())
        );
        assert_eq!(status, "active");

        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");
    }

    #[tokio::test]
    #[serial]
    async fn test_device_operations() {
        let db = get_test_database("test_device_operations")
            .await
            .expect("Failed to setup test database");
        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");

        let machine_id = test_device_id();
        let machine_id_str = machine_id.to_string();

        // Create a test device with required fields using SeaORM
        use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
        let insert_result = db.orm()
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                "INSERT INTO devices (id, name, serial_number, device_type, created_at) VALUES (?, ?, ?, ?, NOW())",
                vec![
                    machine_id_str.clone().into(),
                    "test_device".into(),
                    "SN123456".into(),
                    "robot".into(),
                ]
            ))
            .await;

        assert!(insert_result.is_ok(), "Device creation should succeed");

        // Query the created device
        let query_result = db
            .orm()
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                "SELECT id, name, serial_number, device_type FROM devices WHERE id = ?",
                vec![machine_id_str.clone().into()],
            ))
            .await;

        assert!(query_result.is_ok(), "Device query should succeed");
        let row = query_result.unwrap().unwrap();
        let retrieved_id: String = row.try_get("", "id").unwrap();
        let name: String = row.try_get("", "name").unwrap();
        let serial_number: String = row.try_get("", "serial_number").unwrap();
        let device_type: String = row.try_get("", "device_type").unwrap();

        assert_eq!(retrieved_id, machine_id_str);
        assert_eq!(name, "test_device");
        assert_eq!(serial_number, "SN123456");
        assert_eq!(device_type, "robot");

        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");
    }

    // Client Manager tests
    use easytier_bridge::client_manager::ClientManager;

    #[tokio::test]
    #[serial]
    async fn test_client_manager_initialization() {
        let db = get_test_database("test_client_manager_initialization")
            .await
            .expect("Failed to setup test database");
        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");

        // Get database URL for test
        let db_url = get_test_database_url("test_client_manager_initialization");

        // Test ClientManager creation with new signature
        let _client_manager = ClientManager::new(&db_url, None)
            .await
            .expect("Failed to create ClientManager");

        // ClientManager should be created successfully
        // We can't easily test internal state without more exposed methods
    }

    #[tokio::test]
    #[serial]
    async fn test_client_manager_sessions() {
        let db = get_test_database("test_client_manager_sessions")
            .await
            .expect("Failed to setup test database");
        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");

        // Get database URL for test
        let db_url = get_test_database_url("test_client_manager_sessions");

        // Test ClientManager creation with new signature
        let client_manager = ClientManager::new(&db_url, None)
            .await
            .expect("Failed to create ClientManager");

        // Test getting sessions (should be empty initially)
        let _sessions_list = client_manager.list_sessions().await;
        // Initially should be empty or contain existing test data
        // We just verify the method can be called without panicking
    }

    // Session tests
    use easytier_bridge::client_manager::{
        session::{Location, Session},
        storage::Storage,
    };

    #[tokio::test]
    #[serial]
    async fn test_session_creation() {
        let db = get_test_database("test_session_creation")
            .await
            .expect("Failed to setup test database");
        cleanup_test_database(&db)
            .await
            .expect("Failed to cleanup test database");
        let storage = Storage::new(db.clone());
        let weak_storage = storage.weak_ref();
        let client_url = test_client_url();
        let location = Some(Location {
            country: "中国".to_string(),
            city: Some("北京".to_string()),
            region: Some("北京市".to_string()),
        });

        // Test Session creation
        let _session = Session::new(weak_storage, client_url.clone(), location.clone());

        // Session should be created successfully
    }

    #[tokio::test]
    #[serial]
    async fn test_location_serialization() {
        let location = Location {
            country: "中国".to_string(),
            city: Some("上海".to_string()),
            region: Some("上海市".to_string()),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&location).unwrap();
        assert!(json.contains("中国"));
        assert!(json.contains("上海"));

        // Test JSON deserialization
        let deserialized: Location = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.country, "中国");
        assert_eq!(deserialized.city, Some("上海".to_string()));
        assert_eq!(deserialized.region, Some("上海市".to_string()));
    }
    
    // Device Status Update Integration Test
    #[tokio::test]
    #[serial]
    async fn test_device_status_update_integration() {
        let test_name = "device_status_update_integration";
        let db = get_test_database(test_name).await
            .expect("Failed to setup test database");
        
        // Setup test organization
        let org_id = setup_test_organization(&db).await.unwrap();
        
        // Create ClientManager
        let mut client_mgr = ClientManager::new(&get_test_database_url(test_name), None).await
            .expect("Failed to create ClientManager");
        
        // Test the complete flow: device edit -> status preservation -> heartbeat -> status recovery
        let device_id = test_device_id();
        
        // Step 1: Simulate device edit (like from frontend) - should preserve status
        {
            use crate::db::entities::devices;
            use sea_orm::{ActiveModelTrait, Set};
            use chrono::Utc;
            
            // Create device with approved status
            let device = devices::ActiveModel {
                id: Set(device_id.to_string()),
                name: Set("Original Device".to_string()),
                serial_number: Set(device_id.to_string()),
                device_type: Set(devices::DeviceType::Robot),
                organization_id: Set(Some(org_id.clone())),
                status: Set(devices::DeviceStatus::Approved),
                last_heartbeat: Set(Some(Utc::now().into())),
                created_at: Set(Utc::now().into()),
                updated_at: Set(Utc::now().into()),
                ..Default::default()
            };
            
            device.insert(db.orm()).await.unwrap();
        }
        
        // Step 2: Simulate device edit (update name only, not status)
        {
            use crate::db::entities::devices;
            use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
            
            let device = devices::Entity::find()
                .filter(devices::Column::Id.eq(device_id.to_string()))
                .one(db.orm())
                .await
                .unwrap()
                .unwrap();
            
            let mut active: devices::ActiveModel = device.clone().into();
            active.name = Set("Updated Device Name".to_string()); // Only update name
            active.updated_at = Set(chrono::Utc::now().into());
            // Note: NOT updating status - this simulates the frontend edit behavior
            
            active.update(db.orm()).await.unwrap();
        }
        
        // Step 3: Verify status is preserved after edit
        {
            use crate::db::entities::devices;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            
            let device = devices::Entity::find()
                .filter(devices::Column::Id.eq(device_id.to_string()))
                .one(db.orm())
                .await
                .unwrap()
                .unwrap();
            
            assert_eq!(device.status, devices::DeviceStatus::Approved);
            assert_eq!(device.name, "Updated Device Name");
        }
        
        // Step 4: Simulate heartbeat - should maintain approved status
        let client_url = test_client_url();
        let storage = client_mgr.storage().weak_ref();
        let session = Session::new(storage, client_url, None);
        
        // The heartbeat handling should preserve the approved status
        // This is tested in the dedicated test_device_status_updates module
        
        cleanup_test_database(test_name).await.unwrap();
    }
}
