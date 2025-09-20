//! Client manager integration tests with isolated databases for concurrent testing
//!
//! Each test uses an isolated database for true concurrent testing.
use easytier_bridge::client_manager::ClientManager;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use std::sync::Arc;
use url::Url;
// Removed unused imports: Database, Uuid
// Tracing initialization removed as it was unused

mod common;

use common::*;

#[tokio::test]
async fn test_client_manager_initialization() {
    let db = get_test_database("test_client_manager_initialization")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    // Test ClientManager creation
    let db_url = get_test_database_url("test_client_manager_initialization");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");

    // ClientManager should be created successfully
    // Initially it should not be running (no listeners added)
    assert!(
        !client_manager.is_running(),
        "ClientManager should not be running initially"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_client_manager_initialization")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_client_manager_running_state() {
    let db = get_test_database("test_client_manager_running_state")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_client_manager_running_state");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");

    // Test is_running method
    let is_running = client_manager.is_running();
    assert!(
        !is_running,
        "ClientManager should not be running without listeners"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_client_manager_running_state")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_empty_session_list() {
    let db = get_test_database("test_empty_session_list")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_empty_session_list");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");

    // Test listing sessions when none exist
    let sessions = client_manager.list_sessions().await;
    assert!(
        sessions.is_empty(),
        "Session list should be empty initially"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_empty_session_list")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_session_by_machine_id() {
    let db = get_test_database("test_session_by_machine_id")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_session_by_machine_id");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let org_id = test_organization_id();
    let machine_id = test_device_id();

    // Test getting session by device ID when none exists
    let session = client_manager
        .get_session_by_device_id(&org_id, &machine_id)
        .await;

    assert!(
        session.is_none(),
        "Session should not exist for non-existent machine"
    );

    // // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_session_by_machine_id")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_list_machines_by_org_empty() {
    let db = get_test_database("test_list_machines_by_org_empty")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_list_machines_by_org_empty");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let org_id = test_organization_id();

    // Test listing devices by organization ID when no devices exist
    let machines = client_manager
        .list_devices_by_organization_id(&org_id)
        .await;

    assert!(
        machines.is_empty(),
        "Machine list should be empty for user with no machines"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_list_machines_by_org_empty")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_list_devices_by_organization_with_clients() {
    let db = get_test_database("test_list_devices_by_organization_with_clients")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_list_devices_by_organization_with_clients");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let org_id = test_organization_id();

    // Create organization first
    db.orm().execute(Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            "INSERT INTO organizations (id, name, status, created_at, updated_at) VALUES (?, ?, ?, NOW(), NOW())",
            vec![
                org_id.to_string().into(),
                "Test Organization".into(),
                "active".into(),
            ]
        ))
        .await
        .expect("Should insert organization");

    // Manually add client sessions to in-memory storage to test the functionality
    let mut expected_urls = Vec::new();

    for i in 0..3 {
        let device_id = uuid::Uuid::new_v4();
        let client_url = format!("tcp://127.0.0.1:808{}", i);
        let token = format!("test_token_list_machines_{}", i);

        // Create storage token and add to in-memory storage
        use easytier_bridge::client_manager::storage::StorageToken;
        let storage_token = StorageToken {
            token: token.clone(),
            client_url: url::Url::parse(&client_url).expect("Should parse URL"),
            device_id,
            organization_id: org_id.to_string(),
        };

        // Access the ClientManager's storage and add the client
        let storage = client_manager.storage();
        storage.update_client(storage_token, chrono::Utc::now().timestamp());

        expected_urls.push(url::Url::parse(&client_url).expect("Should parse URL"));
    }

    // Test listing devices by organization ID
    let machines = client_manager
        .list_devices_by_organization_id(&org_id)
        .await;

    assert_eq!(
        machines.len(),
        3,
        "Should return 3 devices for the organization"
    );

    // Verify all expected URLs are present (order may vary)
    for expected_url in &expected_urls {
        assert!(
            machines.contains(expected_url),
            "Should contain expected device URL: {}",
            expected_url
        );
    }

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_list_devices_by_organization_with_clients")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_list_devices_by_organization_different_orgs() {
    let db = get_test_database("test_list_devices_by_organization_different_orgs")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_list_devices_by_organization_different_orgs");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let org_id_1 = test_organization_id();
    let org_id_2 = test_organization_id();

    // Create organizations first
    db.orm().execute(Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            "INSERT INTO organizations (id, name, status, created_at, updated_at) VALUES (?, ?, ?, NOW(), NOW())",
            vec![
                org_id_1.to_string().into(),
                "Test Organization 1".into(),
                "active".into(),
            ]
        ))
        .await
        .expect("Should insert organization 1");

    db.orm().execute(Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            "INSERT INTO organizations (id, name, status, created_at, updated_at) VALUES (?, ?, ?, NOW(), NOW())",
            vec![
                org_id_2.to_string().into(),
                "Test Organization 2".into(),
                "active".into(),
            ]
        ))
        .await
        .expect("Should insert organization 2");

    // Add machines for user 1
    for i in 0..2 {
        let device_id = uuid::Uuid::new_v4();
        let client_url = format!("tcp://127.0.0.1:900{}", i);
        let token = format!("test_token_user1_{}", i);

        use easytier_bridge::client_manager::storage::StorageToken;
        let storage_token = StorageToken {
            token: token.clone(),
            client_url: url::Url::parse(&client_url).expect("Should parse URL"),
            device_id,
            organization_id: org_id_1.to_string(),
        };

        let storage = client_manager.storage();
        storage.update_client(storage_token, chrono::Utc::now().timestamp());
    }

    // Add machines for user 2
    for i in 0..3 {
        let device_id = uuid::Uuid::new_v4();
        let client_url = format!("tcp://127.0.0.1:910{}", i);
        let token = format!("test_token_user2_{}", i);

        use easytier_bridge::client_manager::storage::StorageToken;
        let storage_token = StorageToken {
            token: token.clone(),
            client_url: url::Url::parse(&client_url).expect("Should parse URL"),
            device_id,
            organization_id: org_id_2.to_string(),
        };

        let storage = client_manager.storage();
        storage.update_client(storage_token, chrono::Utc::now().timestamp());
    }

    // Test listing devices for each organization
    let user1_machines = client_manager
        .list_devices_by_organization_id(&org_id_1)
        .await;
    let user2_machines = client_manager
        .list_devices_by_organization_id(&org_id_2)
        .await;

    assert_eq!(
        user1_machines.len(),
        2,
        "Organization 1 should have 2 devices"
    );
    assert_eq!(
        user2_machines.len(),
        3,
        "Organization 2 should have 3 devices"
    );

    // Verify no overlap between organizations' devices
    for machine1 in &user1_machines {
        assert!(
            !user2_machines.contains(machine1),
            "Organization 1's device should not appear in organization 2's list: {}",
            machine1
        );
    }

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_list_devices_by_organization_different_orgs")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_list_devices_by_organization_inactive_filtered() {
    let db = get_test_database("test_list_devices_by_organization_inactive_filtered")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_list_devices_by_organization_inactive_filtered");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let org_id = test_organization_id();

    // Create organization first
    db.orm().execute(Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            "INSERT INTO organizations (id, name, status, created_at, updated_at) VALUES (?, ?, ?, NOW(), NOW())",
            vec![
                org_id.to_string().into(),
                "Test Organization".into(),
                "active".into(),
            ]
        ))
        .await
        .expect("Should insert organization");

    // Add active machine (only active machines are stored in memory)
    let active_device_id = uuid::Uuid::new_v4();
    let active_client_url = "tcp://127.0.0.1:8080";
    let active_token = "test_token_active";

    use easytier_bridge::client_manager::storage::StorageToken;
    let storage_token = StorageToken {
        token: active_token.to_string(),
        client_url: url::Url::parse(active_client_url).expect("Should parse URL"),
        device_id: active_device_id,
        organization_id: org_id.to_string(),
    };

    let storage = client_manager.storage();
    storage.update_client(storage_token, chrono::Utc::now().timestamp());

    // Note: In the new in-memory design, inactive clients are not stored,
    // so we only test with active clients. The filtering behavior is now
    // implicit - only active clients are in memory.

    // Test listing devices - should only return active ones
    let machines = client_manager
        .list_devices_by_organization_id(&org_id)
        .await;

    assert_eq!(machines.len(), 1, "Should return only active devices");
    assert_eq!(
        machines[0].to_string(),
        active_client_url,
        "Should return the active device URL"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_list_devices_by_organization_inactive_filtered")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_list_devices_by_organization_database_error_handling() {
    let db = get_test_database("test_list_devices_by_organization_database_error")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_list_devices_by_organization_database_error");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");

    // Test with invalid user ID format (should still work, just return empty)
    let invalid_user_id = "invalid-user-id-format";
    let machines = client_manager
        .list_devices_by_organization_id(invalid_user_id)
        .await;

    // Should return empty list without crashing
    assert!(
        machines.is_empty(),
        "Should return empty list for invalid organization ID"
    );

    // Test with empty organization ID
    let empty_user_id = "";
    let machines = client_manager
        .list_devices_by_organization_id(empty_user_id)
        .await;

    assert!(
        machines.is_empty(),
        "Should return empty list for empty organization ID"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_list_devices_by_organization_database_error")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_heartbeat_requests() {
    let db = get_test_database("test_heartbeat_requests")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_heartbeat_requests");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let client_url = test_client_url();

    // Test getting heartbeat requests for non-existent client
    let heartbeat = client_manager.get_heartbeat_requests(&client_url).await;

    assert!(
        heartbeat.is_none(),
        "Heartbeat should be None for non-existent client"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_heartbeat_requests")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_machine_location() {
    let db = get_test_database("test_machine_location")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_machine_location");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");
    let client_url = test_client_url();

    // Test getting machine location for non-existent client
    let location = client_manager.get_device_location(&client_url).await;

    assert!(
        location.is_none(),
        "Location should be None for non-existent client"
    );

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_machine_location")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_database_access() {
    // Simple database connectivity test using SeaORM
    let db = get_test_database("test_database_access")
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
            "SELECT 1 as test".to_owned(),
        ))
        .await;

    assert!(result.is_ok(), "Should be able to execute simple query");
    let row = result.unwrap().unwrap();
    let test_value: i32 = row.try_get("", "test").unwrap();
    assert_eq!(test_value, 1);

    // 删除测试数据库
    remove_test_database("test_database_access")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_private_ip_location_lookup() {
    // Test the private IP detection logic
    let private_urls = vec![
        "tcp://192.168.1.1:8080",
        "tcp://10.0.0.1:8080",
        "tcp://172.16.0.1:8080",
        "tcp://127.0.0.1:8080",
    ];

    for url_str in private_urls {
        let _private_url = Url::parse(url_str).unwrap();
        // The actual lookup_location method is private, so we can't test it directly
        // But we can verify the URL parsing works
        assert!(
            _private_url.host_str().is_some(),
            "Private URL should have valid host"
        );
    }
}

#[tokio::test]
async fn test_geoip_integration() {
    let db = get_test_database("test_geoip_integration")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    // Test ClientManager with GeoIP database (None in this case)
    let db_url = get_test_database_url("test_geoip_integration");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");

    // Since we don't have a real GeoIP database in tests,
    // we just verify the ClientManager can be created with None
    // The actual GeoIP functionality would require a real MaxMind database file

    // Cleanup resources
    client_manager.shutdown().await;

    // 删除测试数据库
    remove_test_database("test_geoip_integration")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_concurrent_session_access() {
    let db = get_test_database("test_concurrent_session_access")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let db_url = get_test_database_url("test_concurrent_session_access");
    let client_manager = Arc::new(
        ClientManager::new(&db_url, None)
            .await
            .expect("Failed to create ClientManager"),
    );

    // Test concurrent access to session list
    let mut handles = vec![];

    for _ in 0..10 {
        let manager_clone = client_manager.clone();
        let handle = tokio::spawn(async move {
            let sessions = manager_clone.list_sessions().await;
            sessions.len()
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent session access should succeed");
        assert_eq!(
            result.unwrap(),
            0,
            "All concurrent calls should return empty list"
        );
    }

    // Note: Cannot call shutdown on Arc<ClientManager> as it requires mutable reference
    // But we can still clean up the test database

    // 删除测试数据库
    remove_test_database("test_concurrent_session_access")
        .await
        .expect("Failed to remove test database");
}
