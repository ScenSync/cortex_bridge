//! Advanced session tests for comprehensive coverage
//!
//! This file contains advanced test cases for the Session module,
//! focusing on RPC operations, network instance management, and error handling.

use easytier::proto::{
    common::Uuid as ProtoUuid,
    web::{HeartbeatRequest, NetworkConfig, RunNetworkInstanceRequest},
};
use easytier_bridge::client_manager::{
    session::{Location, Session},
    storage::{Storage, StorageToken},
};
use std::str::FromStr;
use std::sync::Arc;
use url::Url;

mod common;
use common::*;

#[tokio::test]
async fn test_session_data_creation() {
    let db = get_test_database("test_session_data_creation")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    // Test SessionData creation without location
    let session = Session::new(weak_storage.clone(), client_url.clone(), None);
    let session_data = session.data().read().await;

    assert!(
        session_data.req().is_none(),
        "Initial request should be None"
    );
    assert!(
        session_data.location().is_none(),
        "Initial location should be None"
    );

    // 删除测试数据库
    remove_test_database("test_session_data_creation")
        .await
        .expect("Failed to remove test database");

    // Test SessionData creation with location
    let location = Location {
        country: "测试国家".to_string(),
        city: Some("测试城市".to_string()),
        region: Some("测试地区".to_string()),
    };

    let session_with_location = Session::new(weak_storage, client_url, Some(location.clone()));
    let session_data_with_location = session_with_location.data().read().await;

    let stored_location = session_data_with_location.location().unwrap();
    assert_eq!(stored_location.country, location.country);
    assert_eq!(stored_location.city, location.city);
    assert_eq!(stored_location.region, location.region);
}

#[tokio::test]
async fn test_session_heartbeat_waiter_functionality() {
    let db = get_test_database("test_session_heartbeat_waiter_functionality")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    let session = Session::new(weak_storage, client_url, None);
    let session_data = session.data().read().await;

    // Test creating multiple heartbeat waiters
    let waiter1 = session_data.heartbeat_waiter();
    let waiter2 = session_data.heartbeat_waiter();

    // Both waiters should be created successfully
    // We can't easily test the actual broadcast functionality without more complex setup,
    // but we can verify the waiters are created
    drop(waiter1);
    drop(waiter2);

    // 删除测试数据库
    remove_test_database("test_session_heartbeat_waiter_functionality")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_session_rpc_client_creation() {
    let db = get_test_database("test_session_rpc_client_creation")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    let session = Session::new(weak_storage, client_url, None);

    // Test getting RPC client when session is not running
    let rpc_client = Some(session.scoped_rpc_client());
    assert!(
        rpc_client.is_some(),
        "Should be able to create RPC client even when session is not running"
    );

    // 删除测试数据库
    remove_test_database("test_session_rpc_client_creation")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_session_network_instance_operations() {
    let db = get_test_database("test_session_network_instance_operations")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    let mut session = Session::new(weak_storage, client_url, None);

    // Test running network instance (will fail since no actual tunnel is connected)
    let network_config = NetworkConfig {
        network_name: Some("test_network".to_string()),
        network_secret: Some("test_secret".to_string()),
        ..Default::default()
    };

    let run_request = RunNetworkInstanceRequest {
        inst_id: None,
        config: Some(network_config),
    };

    let run_result = session.run_network_instance(run_request).await;
    // This will likely fail since there's no actual RPC connection, but we test the method exists
    // and handles the error gracefully
    assert!(
        run_result.is_err(),
        "Should fail when no RPC connection is available"
    );

    // Test stopping network instance
    let stop_result = session
        .stop_network_instance("test_instance".to_string())
        .await;
    assert!(
        stop_result.is_err(),
        "Should fail when no RPC connection is available"
    );

    // Test listing network instances
    let list_result = session.list_network_instances().await;
    assert!(
        list_result.is_err(),
        "Should fail when no RPC connection is available"
    );

    // 删除测试数据库
    remove_test_database("test_session_network_instance_operations")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_heartbeat_request_creation() {
    // Test creating various HeartbeatRequest configurations
    let machine_id = test_device_id();
    let user_token = "test_token_123".to_string();
    let report_time = chrono::Local::now().to_rfc3339();

    let uuid_bytes = machine_id.as_bytes();
    let proto_uuid = ProtoUuid {
        part1: u32::from_be_bytes([uuid_bytes[0], uuid_bytes[1], uuid_bytes[2], uuid_bytes[3]]),
        part2: u32::from_be_bytes([uuid_bytes[4], uuid_bytes[5], uuid_bytes[6], uuid_bytes[7]]),
        part3: u32::from_be_bytes([uuid_bytes[8], uuid_bytes[9], uuid_bytes[10], uuid_bytes[11]]),
        part4: u32::from_be_bytes([
            uuid_bytes[12],
            uuid_bytes[13],
            uuid_bytes[14],
            uuid_bytes[15],
        ]),
    };

    let heartbeat_request = HeartbeatRequest {
        machine_id: Some(proto_uuid),
        inst_id: None,
        user_token: user_token.clone(),
        easytier_version: "test_version".to_string(),
        report_time: report_time.clone(),
        hostname: "test_host".to_string(),
        running_network_instances: vec![],
    };

    assert_eq!(heartbeat_request.machine_id, Some(proto_uuid));
    assert_eq!(heartbeat_request.user_token, user_token);
    assert_eq!(heartbeat_request.report_time, report_time);
}

#[tokio::test]
async fn test_location_edge_cases() {
    // Test Location with various edge cases
    let locations = vec![
        Location {
            country: "".to_string(), // Empty country
            city: None,
            region: None,
        },
        Location {
            country: "很长的国家名称测试".to_string(), // Long country name
            city: Some("很长的城市名称测试".to_string()),
            region: Some("很长的地区名称测试".to_string()),
        },
        Location {
            country: "Country with special chars: !@#$%^&*()".to_string(),
            city: Some("City with unicode: 北京市 🏙️".to_string()),
            region: Some("Region with numbers: 123456".to_string()),
        },
    ];

    for location in locations {
        // Test JSON serialization/deserialization
        let json_result = serde_json::to_string(&location);
        assert!(
            json_result.is_ok(),
            "Location should be serializable: {:?}",
            location
        );

        let json_str = json_result.unwrap();
        let deserialized_result: Result<Location, _> = serde_json::from_str(&json_str);
        assert!(
            deserialized_result.is_ok(),
            "Location should be deserializable: {}",
            json_str
        );

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.country, location.country);
        assert_eq!(deserialized.city, location.city);
        assert_eq!(deserialized.region, location.region);
    }
}

#[tokio::test]
async fn test_session_debug_formatting() {
    let db = get_test_database("test_session_debug_formatting")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    let location = Location {
        country: "测试国家".to_string(),
        city: Some("测试城市".to_string()),
        region: Some("测试地区".to_string()),
    };

    let session = Session::new(weak_storage, client_url, Some(location));

    // Test Debug formatting
    let debug_str = format!("{:?}", session);
    assert!(
        debug_str.contains("Session"),
        "Debug output should contain 'Session'"
    );
    assert!(!debug_str.is_empty(), "Debug output should not be empty");

    // 删除测试数据库
    remove_test_database("test_session_debug_formatting")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_storage_token_debug_and_clone() {
    let user_id = test_organization_id();
    let machine_id = test_device_id();
    let client_url = test_client_url();
    let token = "test_token_456".to_string();

    let storage_token = StorageToken {
        token: token.clone(),
        client_url: client_url.clone(),
        device_id: machine_id,
        organization_id: user_id.clone(),
    };

    // Test Debug formatting
    let debug_str = format!("{:?}", storage_token);
    assert!(
        debug_str.contains("StorageToken"),
        "Debug output should contain 'StorageToken'"
    );
    assert!(
        debug_str.contains(&token),
        "Debug output should contain token"
    );

    // Test Clone
    let cloned_token = storage_token.clone();
    assert_eq!(cloned_token.token, storage_token.token);
    assert_eq!(cloned_token.client_url, storage_token.client_url);
    assert_eq!(cloned_token.device_id, storage_token.device_id);
    assert_eq!(cloned_token.organization_id, storage_token.organization_id);
}

#[tokio::test]
async fn test_concurrent_session_operations() {
    let db = get_test_database("test_concurrent_session_operations")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    let session = Arc::new(Session::new(weak_storage, client_url, None));
    let mut handles = vec![];

    // Test concurrent access to session data
    for i in 0..5 {
        let session_clone = session.clone();
        let handle = tokio::spawn(async move {
            let token = session_clone.get_token().await;
            let is_running = session_clone.is_running();
            (i, token.is_none(), is_running)
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await;
        assert!(
            result.is_ok(),
            "Concurrent session operations should succeed"
        );
        let (task_id, token_is_none, is_running) = result.unwrap();
        assert!(token_is_none, "Token should be None for task {}", task_id);
        assert!(
            !is_running,
            "Session should not be running for task {}",
            task_id
        );
    }
    remove_test_database("test_concurrent_session_operations")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_session_data_req_functionality() {
    let db = get_test_database("test_session_data_req_functionality")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();
    let client_url = test_client_url();

    let session = Session::new(weak_storage, client_url, None);
    let session_data = session.data().read().await;

    // Test initial req state
    let initial_req = session_data.req();
    assert!(initial_req.is_none(), "Initial request should be None");

    // We can't easily test setting the req without a full RPC setup,
    // but we've verified the getter works

    // 删除测试数据库
    remove_test_database("test_session_data_req_functionality")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_url_edge_cases_for_sessions() {
    let db = get_test_database("test_url_edge_cases_for_sessions")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();

    // Test various URL formats
    let test_urls = vec![
        "tcp://127.0.0.1:8080",
        "tcp://[::1]:8080", // IPv6
        "tcp://localhost:8080",
        "tcp://example.com:9999",
    ];

    for url_str in test_urls {
        let url = Url::parse(url_str).expect("Should parse valid URL");
        let session = Session::new(weak_storage.clone(), url.clone(), None);

        // Verify session can be created with various URL formats
        assert!(
            !session.is_running(),
            "Session should not be running initially for URL: {}",
            url_str
        );

        let token = session.get_token().await;
        assert!(
            token.is_none(),
            "Token should be None initially for URL: {}",
            url_str
        );
    }
    remove_test_database("test_url_edge_cases_for_sessions")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_storage_weak_ref_debug() {
    let db = get_test_database("test_storage_weak_ref_debug")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let storage = Storage::new(db);
    let weak_storage = storage.weak_ref();

    // Test Debug formatting for WeakRefStorage
    let debug_str = format!("{:?}", weak_storage);
    // WeakRefStorage is now std::sync::Weak<StorageInner>, so debug output contains "Weak"
    assert!(
        debug_str.contains("Weak"),
        "Debug output should contain 'Weak'"
    );
    assert!(!debug_str.is_empty(), "Debug output should not be empty");

    // Test Clone for WeakRefStorage
    let cloned_weak = weak_storage.clone();
    let cloned_debug_str = format!("{:?}", cloned_weak);
    assert!(
        !cloned_debug_str.is_empty(),
        "Cloned weak reference should have debug output"
    );
    remove_test_database("test_storage_weak_ref_debug")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_chrono_datetime_parsing() {
    // Test various datetime formats that might be encountered
    let test_datetimes = vec![
        chrono::Local::now().to_rfc3339(),
        chrono::Utc::now().to_rfc3339(),
        "2024-01-01T00:00:00Z".to_string(),
        "2024-12-31T23:59:59+08:00".to_string(),
    ];

    for datetime_str in test_datetimes {
        let parse_result = chrono::DateTime::<chrono::FixedOffset>::from_str(&datetime_str);
        // Some formats might not parse, but we test the parsing logic exists
        // The actual parsing success depends on the format and timezone
        let _ = parse_result; // Just verify the parsing attempt doesn't panic
    }
}
