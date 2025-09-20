//! Comprehensive storage tests for the simplified Storage module
//!
//! This file contains test cases for the simplified Storage module,
//! focusing on the new nested DashMap structure and core functionality.

use std::sync::Arc;
use easytier_bridge::client_manager::storage::{Storage, StorageToken};
use url::Url;
use uuid::Uuid;
use tracing_subscriber;
use std::sync::Once;

// Initialize tracing once for all tests
static INIT_TRACING: Once = Once::new();

fn init_tracing() {
    INIT_TRACING.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .init();
    });
}

mod common;
use common::*;

#[tokio::test]
async fn test_storage_initialization_and_basic_operations() {
    init_tracing();
    let test_function_name = "test_storage_initialization_and_basic_operations";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    // Test that storage is properly initialized
    let db_ref = storage.db();
    assert!(db_ref.orm().ping().await.is_ok());

    // Test basic organization client listing (should be empty)
    let org_id = "test-org-001".to_string();
    let client_urls = storage.list_organization_clients(&org_id);
    assert!(client_urls.is_empty());
}

#[tokio::test]
async fn test_storage_update_and_retrieve_client() {
    init_tracing();
    let test_function_name = "test_storage_update_and_retrieve_client";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    // Create a test storage token
    let storage_token = StorageToken {
        token: "test_token_001".to_string(),
        client_url: Url::parse("udp://127.0.0.1:11001").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: "test-org-001".to_string(),
    };

    // Update client (this is how we "add" clients in the new API)
    let report_time = chrono::Utc::now().timestamp();
    storage.update_client(storage_token.clone(), report_time);

    // Verify client was added by checking if we can get its URL by device ID
    let retrieved_url = storage.get_client_url_by_device_id(
        &storage_token.organization_id,
        &storage_token.device_id,
    );
    assert!(retrieved_url.is_some());
    assert_eq!(retrieved_url.unwrap(), storage_token.client_url);

    // Verify it appears in organization client list
    let org_clients = storage.list_organization_clients(&storage_token.organization_id);
    assert_eq!(org_clients.len(), 1);
    assert_eq!(org_clients[0], storage_token.client_url);
}

#[tokio::test]
async fn test_storage_update_client_multiple_times() {
    init_tracing();
    let test_function_name = "test_storage_update_client_multiple_times";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    let storage_token = StorageToken {
        token: "test_token_002".to_string(),
        client_url: Url::parse("udp://127.0.0.1:11002").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: "test-org-002".to_string(),
    };

    // Update client multiple times with different timestamps
    let report_time1 = 1000;
    let report_time2 = 2000;
    
    storage.update_client(storage_token.clone(), report_time1);
    storage.update_client(storage_token.clone(), report_time2);

    // Should still only have one entry for this device
    let org_clients = storage.list_organization_clients(&storage_token.organization_id);
    assert_eq!(org_clients.len(), 1);
    
    let retrieved_url = storage.get_client_url_by_device_id(
        &storage_token.organization_id,
        &storage_token.device_id,
    );
    assert!(retrieved_url.is_some());
}

#[tokio::test]
async fn test_storage_remove_client() {
    init_tracing();
    let test_function_name = "test_storage_remove_client";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    let storage_token = StorageToken {
        token: "test_token_003".to_string(),
        client_url: Url::parse("udp://127.0.0.1:11003").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: "test-org-003".to_string(),
    };

    // Add client
    storage.update_client(storage_token.clone(), chrono::Utc::now().timestamp());
    
    // Verify it exists
    let retrieved_url = storage.get_client_url_by_device_id(
        &storage_token.organization_id,
        &storage_token.device_id,
    );
    assert!(retrieved_url.is_some());

    // Remove client
    storage.remove_client(&storage_token);

    // Verify it's gone
    let retrieved_url_after = storage.get_client_url_by_device_id(
        &storage_token.organization_id,
        &storage_token.device_id,
    );
    assert!(retrieved_url_after.is_none());

    // Organization client list should be empty
    let org_clients = storage.list_organization_clients(&storage_token.organization_id);
    assert!(org_clients.is_empty());
}

#[tokio::test]
async fn test_storage_multiple_organizations() {
    init_tracing();
    let test_function_name = "test_storage_multiple_organizations";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    let org_id_1 = "test-org-004".to_string();
    let org_id_2 = "test-org-005".to_string();

    // Add clients to different organizations
    let token1 = StorageToken {
        token: "test_token_004".to_string(),
        client_url: Url::parse("udp://127.0.0.1:11004").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: org_id_1.clone(),
    };

    let token2 = StorageToken {
        token: "test_token_005".to_string(),
        client_url: Url::parse("udp://127.0.0.1:11005").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: org_id_2.clone(),
    };

    storage.update_client(token1.clone(), chrono::Utc::now().timestamp());
    storage.update_client(token2.clone(), chrono::Utc::now().timestamp());

    // Verify each organization only sees its own clients
    let org1_urls = storage.list_organization_clients(&org_id_1);
    let org2_urls = storage.list_organization_clients(&org_id_2);

    assert_eq!(org1_urls.len(), 1);
    assert_eq!(org2_urls.len(), 1);
    assert_eq!(org1_urls[0], token1.client_url);
    assert_eq!(org2_urls[0], token2.client_url);
}

#[tokio::test]
async fn test_storage_multiple_devices_same_organization() {
    init_tracing();
    let test_function_name = "test_storage_multiple_devices_same_organization";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    let org_id = "test-org-006".to_string();
    let mut tokens = Vec::new();

    // Add multiple devices to same organization
    for i in 0..5 {
        let token = StorageToken {
            token: format!("test_token_{:03}", i + 100),
            client_url: Url::parse(&format!("udp://127.0.0.1:{}", 12000 + i)).unwrap(),
            device_id: Uuid::new_v4(),
            organization_id: org_id.clone(),
        };
        storage.update_client(token.clone(), chrono::Utc::now().timestamp());
        tokens.push(token);
    }

    // Verify all devices are listed for the organization
    let org_clients = storage.list_organization_clients(&org_id);
    assert_eq!(org_clients.len(), 5);

    // Verify each device can be retrieved by its ID
    for token in &tokens {
        let retrieved_url = storage.get_client_url_by_device_id(&org_id, &token.device_id);
        assert!(retrieved_url.is_some());
        assert_eq!(retrieved_url.unwrap(), token.client_url);
    }
}

#[tokio::test]
async fn test_storage_weak_reference() {
    init_tracing();
    let test_function_name = "test_storage_weak_reference";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    // Create weak reference
    let weak_ref = storage.weak_ref();

    // Verify we can upgrade it back to strong reference
    let strong_ref = Storage::try_from(weak_ref).unwrap();

    // Test that both references work the same
    let org_id = "test-org-007".to_string();
    let empty_list1 = storage.list_organization_clients(&org_id);
    let empty_list2 = strong_ref.list_organization_clients(&org_id);
    
    assert_eq!(empty_list1, empty_list2);
}

#[tokio::test]
async fn test_storage_concurrent_operations() {
    init_tracing();
    let test_function_name = "test_storage_concurrent_operations";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Arc::new(Storage::new(db));

    let org_id = "test-org-008".to_string();
    let mut handles = Vec::new();

    // Spawn multiple tasks that add clients concurrently
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let org_id_clone = org_id.clone();
        
        let handle = tokio::spawn(async move {
            let token = StorageToken {
                token: format!("concurrent_token_{:03}", i),
                client_url: Url::parse(&format!("udp://127.0.0.1:{}", 13000 + i)).unwrap(),
                device_id: Uuid::new_v4(),
                organization_id: org_id_clone,
            };
            storage_clone.update_client(token, chrono::Utc::now().timestamp());
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all clients were added
    let org_clients = storage.list_organization_clients(&org_id);
    assert_eq!(org_clients.len(), 10);
}

#[tokio::test]
async fn test_storage_edge_cases() {
    init_tracing();
    let test_function_name = "test_storage_edge_cases";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    // Test with empty organization ID
    let empty_org_id = String::new();
    let empty_org_clients = storage.list_organization_clients(&empty_org_id);
    assert!(empty_org_clients.is_empty());

    // Test getting client URL for non-existent organization
    let non_existent_org = "non-existent-org".to_string();
    let non_existent_url = storage.get_client_url_by_device_id(&non_existent_org, &Uuid::new_v4());
    assert!(non_existent_url.is_none());

    // Test getting client URL for non-existent device in existing org
    let org_id = "test-org-009".to_string();
    let token = StorageToken {
        token: "test_token_edge".to_string(),
        client_url: Url::parse("udp://127.0.0.1:14000").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: org_id.clone(),
    };
    storage.update_client(token.clone(), chrono::Utc::now().timestamp());

    let non_existent_device_url = storage.get_client_url_by_device_id(&org_id, &Uuid::new_v4());
    assert!(non_existent_device_url.is_none());
}

#[tokio::test]
async fn test_storage_remove_client_edge_cases() {
    init_tracing();
    let test_function_name = "test_storage_remove_client_edge_cases";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    // Test removing non-existent client (should not panic)
    let non_existent_token = StorageToken {
        token: "non_existent".to_string(),
        client_url: Url::parse("udp://127.0.0.1:15000").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: "non-existent-org".to_string(),
    };
    storage.remove_client(&non_existent_token); // Should not panic

    // Test removing the same client twice
    let org_id = "test-org-010".to_string();
    let token = StorageToken {
        token: "test_token_remove_twice".to_string(),
        client_url: Url::parse("udp://127.0.0.1:15001").unwrap(),
        device_id: Uuid::new_v4(),
        organization_id: org_id.clone(),
    };

    storage.update_client(token.clone(), chrono::Utc::now().timestamp());
    storage.remove_client(&token);
    storage.remove_client(&token); // Should not panic

    let org_clients = storage.list_organization_clients(&org_id);
    assert!(org_clients.is_empty());
}

#[tokio::test]
async fn test_storage_database_access() {
    init_tracing();
    let test_function_name = "test_storage_database_access";
    let db = get_test_database(test_function_name).await.unwrap();
    let storage = Storage::new(db);

    // Test direct database access
    let db_ref = storage.db();
    let ping_result = db_ref.orm().ping().await;
    assert!(ping_result.is_ok(), "Database should be accessible through storage");
}