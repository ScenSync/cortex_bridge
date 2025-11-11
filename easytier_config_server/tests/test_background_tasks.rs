/// Integration tests for ClientManager background tasks
///
/// These tests verify that background tasks spawned via tokio::spawn
/// continue running even when called through the FFI boundary, ensuring
/// that device timeout monitoring and session cleanup work correctly.
use easytier_config_server::client_manager::ClientManager;
use easytier_config_server::db::entities::devices;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serial_test::serial;
use std::time::Duration;
use tokio::time::sleep;

#[path = "common/mod.rs"]
mod common;
use common::*;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init();
}

/// Test that the device timeout background task actually runs and marks devices offline
/// This is a critical test that verifies tokio::spawn works across FFI boundary
#[tokio::test]
#[serial]
async fn test_device_timeout_task_runs_continuously() {
    init_tracing();

    let test_name = "device_timeout_task_runs";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    // Create test device BEFORE creating ClientManager
    // This ensures the device exists when the background task does its first check
    let device_id = test_device_id();
    let old_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(120); // 2 minutes ago

    let device = devices::ActiveModel {
        id: Set(device_id.to_string()),
        name: Set("test-device".to_string()),
        serial_number: Set(format!("TEST-{}", device_id)), // Unique serial number
        device_type: Set(devices::DeviceType::Robot),
        status: Set(devices::DeviceStatus::Online), // Start as online
        last_heartbeat: Set(Some(old_heartbeat.into())),
        organization_id: Set(Some(org_id)),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };

    device.insert(db.orm()).await.unwrap();

    // Create the ClientManager - background task will check immediately and find our device
    let manager = ClientManager::new(&get_test_database_url(test_name), None)
        .await
        .unwrap();

    // Wait for the background task to run (it runs immediately on creation)
    sleep(Duration::from_secs(2)).await;

    // Verify device was marked offline
    let updated_device = devices::Entity::find()
        .filter(devices::Column::Id.eq(device_id.to_string()))
        .one(db.orm())
        .await
        .unwrap()
        .expect("Device should exist");

    assert_eq!(
        updated_device.status,
        devices::DeviceStatus::Offline,
        "Device with stale heartbeat should be marked offline by background task"
    );

    drop(manager);
    cleanup_test_database(&db).await.unwrap();
}

/// Test that devices with recent heartbeats are NOT marked offline
#[tokio::test]
#[serial]
async fn test_device_with_recent_heartbeat_stays_online() {
    init_tracing();

    let test_name = "device_recent_heartbeat";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    // Create device BEFORE ClientManager
    let device_id = test_device_id();
    let recent_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(30); // 30 seconds ago

    let device = devices::ActiveModel {
        id: Set(device_id.to_string()),
        name: Set("active-device".to_string()),
        serial_number: Set(format!("ACTIVE-{}", device_id)), // Unique serial number
        device_type: Set(devices::DeviceType::Robot),
        status: Set(devices::DeviceStatus::Online),
        last_heartbeat: Set(Some(recent_heartbeat.into())),
        organization_id: Set(Some(org_id)),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };

    device.insert(db.orm()).await.unwrap();

    // Create manager AFTER device
    let manager = ClientManager::new(&get_test_database_url(test_name), None)
        .await
        .unwrap();

    // Wait for background task to run
    sleep(Duration::from_secs(2)).await;

    // Verify device remains online (heartbeat is recent enough)
    let updated_device = devices::Entity::find()
        .filter(devices::Column::Id.eq(device_id.to_string()))
        .one(db.orm())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated_device.status,
        devices::DeviceStatus::Online,
        "Device with recent heartbeat should stay online"
    );

    drop(manager);
    cleanup_test_database(&db).await.unwrap();
}

/// Test that only Online and Busy devices are affected by timeout
#[tokio::test]
#[serial]
async fn test_only_online_busy_devices_can_timeout() {
    init_tracing();

    let test_name = "device_status_filtering";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    let old_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(120);

    // Create devices in different statuses BEFORE ClientManager
    let test_cases = vec![
        (devices::DeviceStatus::Online, true, "online-dev"),
        (devices::DeviceStatus::Busy, true, "busy-dev"),
        (devices::DeviceStatus::Pending, false, "pending-dev"),
        (devices::DeviceStatus::Offline, false, "offline-dev"),
        (devices::DeviceStatus::Disabled, false, "disabled-dev"),
    ];

    let mut device_ids = Vec::new();

    for (status, _should_timeout, name) in &test_cases {
        let device_id = test_device_id(); // Generate new UUID for each device
        device_ids.push((device_id.to_string(), status.clone()));

        let device = devices::ActiveModel {
            id: Set(device_id.to_string()),
            name: Set(format!("device-{:?}-{}", status, name)),
            serial_number: Set(format!("DEV-{}-{}", device_id, name)), // Unique serial
            device_type: Set(devices::DeviceType::Robot),
            status: Set(status.clone()),
            last_heartbeat: Set(Some(old_heartbeat.into())),
            organization_id: Set(Some(org_id.clone())),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        };

        device.insert(db.orm()).await.unwrap();
    }

    // Create manager AFTER devices
    let manager = ClientManager::new(&get_test_database_url(test_name), None)
        .await
        .unwrap();

    // Wait for timeout task to process devices
    sleep(Duration::from_secs(2)).await;

    // Verify only Online and Busy devices were marked offline
    for (device_id, original_status) in device_ids {
        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.clone()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        match original_status {
            devices::DeviceStatus::Online | devices::DeviceStatus::Busy => {
                assert_eq!(
                    device.status,
                    devices::DeviceStatus::Offline,
                    "{:?} device should be marked offline",
                    original_status
                );
            }
            _ => {
                assert_eq!(
                    device.status, original_status,
                    "{:?} device should maintain its status",
                    original_status
                );
            }
        }
    }

    drop(manager);
    cleanup_test_database(&db).await.unwrap();
}

/// Test that background tasks survive ClientManager being moved/stored
/// This simulates the FFI pattern where ClientManager is stored in a singleton
#[tokio::test]
#[serial]
async fn test_background_tasks_survive_manager_storage() {
    init_tracing();

    let test_name = "manager_storage";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    // Create test device FIRST
    let device_id = test_device_id();
    let device = devices::ActiveModel {
        id: Set(device_id.to_string()),
        name: Set("stored-test".to_string()),
        serial_number: Set(format!("STORED-{}", device_id)), // Unique serial number
        device_type: Set(devices::DeviceType::Robot),
        status: Set(devices::DeviceStatus::Online),
        last_heartbeat: Set(Some(
            (chrono::Utc::now() - chrono::Duration::seconds(120)).into(),
        )),
        organization_id: Set(Some(org_id)),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };

    device.insert(db.orm()).await.unwrap();

    // Simulate FFI pattern: create manager, store it in Arc<Mutex<>>
    let manager = ClientManager::new(&get_test_database_url(test_name), None)
        .await
        .unwrap();
    let manager_arc = std::sync::Arc::new(tokio::sync::Mutex::new(manager));

    // Wait for background tasks to run
    sleep(Duration::from_secs(2)).await;

    // Verify task ran successfully even though manager is in Arc<Mutex<>>
    let updated_device = devices::Entity::find()
        .filter(devices::Column::Id.eq(device_id.to_string()))
        .one(db.orm())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        updated_device.status,
        devices::DeviceStatus::Offline,
        "Background tasks should continue when ClientManager is stored in Arc<Mutex<>>"
    );

    drop(manager_arc);
    cleanup_test_database(&db).await.unwrap();
}
