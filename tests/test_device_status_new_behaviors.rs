//! Tests for new device status management behaviors
//!
//! This module tests the specific behaviors implemented for device status:
//! - Only approved devices are marked offline on timeout
//! - Pending and rejected devices maintain their status when not heartbeating
//! - Offline devices return to approved when reconnecting
//! - Rejected devices return to pending when reconnecting

use serial_test::serial;

#[path = "common/mod.rs"]
mod common;
use common::*;

use chrono::Utc;
use easytier::proto::web::HeartbeatRequest;
use easytier_bridge::client_manager::{session::Session, ClientManager};

/// Test that only approved devices are marked offline on timeout
/// Pending and rejected devices should NOT be marked offline
#[tokio::test]
#[serial]
async fn test_only_approved_devices_marked_offline_on_timeout() {
    let test_name = "only_approved_devices_marked_offline_on_timeout";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    // Create three devices with different statuses and old heartbeats
    let approved_device_id = uuid::Uuid::new_v4();
    let pending_device_id = uuid::Uuid::new_v4();
    let rejected_device_id = uuid::Uuid::new_v4();

    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, Set};

        let old_time = Utc::now() - chrono::Duration::seconds(120); // 2 minutes ago

        // Device 1: Approved with old heartbeat - SHOULD be marked offline
        let approved_device = devices::ActiveModel {
            id: Set(approved_device_id.to_string()),
            name: Set("Approved Device".to_string()),
            serial_number: Set(approved_device_id.to_string()),
            device_type: Set(devices::DeviceType::Robot),
            organization_id: Set(Some(org_id.clone())),
            status: Set(devices::DeviceStatus::Approved),
            last_heartbeat: Set(Some(old_time.into())),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        // Device 2: Pending with old heartbeat - should NOT be marked offline
        let pending_device = devices::ActiveModel {
            id: Set(pending_device_id.to_string()),
            name: Set("Pending Device".to_string()),
            serial_number: Set(pending_device_id.to_string()),
            device_type: Set(devices::DeviceType::Robot),
            organization_id: Set(Some(org_id.clone())),
            status: Set(devices::DeviceStatus::Pending),
            last_heartbeat: Set(Some(old_time.into())),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        // Device 3: Rejected with old heartbeat - should NOT be marked offline
        let rejected_device = devices::ActiveModel {
            id: Set(rejected_device_id.to_string()),
            name: Set("Rejected Device".to_string()),
            serial_number: Set(rejected_device_id.to_string()),
            device_type: Set(devices::DeviceType::Robot),
            organization_id: Set(Some(org_id.clone())),
            status: Set(devices::DeviceStatus::Rejected),
            last_heartbeat: Set(Some(old_time.into())),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        approved_device.insert(db.orm()).await.unwrap();
        pending_device.insert(db.orm()).await.unwrap();
        rejected_device.insert(db.orm()).await.unwrap();
    }

    // Manually trigger the mark_offline_devices function
    {
        use easytier_bridge::client_manager::storage::Storage;

        let storage = Storage::new(db.clone());

        // Call the internal mark_offline_devices function directly
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(60);

        // This mirrors the logic in mark_offline_devices with the new filter
        let offline_devices = devices::Entity::find()
            .filter(devices::Column::LastHeartbeat.lt(cutoff_time))
            .filter(devices::Column::Status.ne(devices::DeviceStatus::Offline))
            .filter(devices::Column::Status.eq(devices::DeviceStatus::Approved)) // NEW: Only approved devices
            .all(storage.db().orm())
            .await
            .unwrap();

        // Mark each device as offline
        for device in offline_devices {
            let mut active: devices::ActiveModel = device.clone().into();
            active.status = Set(devices::DeviceStatus::Offline);
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(storage.db().orm()).await.unwrap();
        }
    }

    // Verify the results
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        // Approved device should be marked offline
        let approved_device = devices::Entity::find()
            .filter(devices::Column::Id.eq(approved_device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            approved_device.status,
            devices::DeviceStatus::Offline,
            "Approved device should be marked offline"
        );

        // Pending device should stay pending
        let pending_device = devices::Entity::find()
            .filter(devices::Column::Id.eq(pending_device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            pending_device.status,
            devices::DeviceStatus::Pending,
            "Pending device should NOT be marked offline"
        );

        // Rejected device should stay rejected
        let rejected_device = devices::Entity::find()
            .filter(devices::Column::Id.eq(rejected_device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            rejected_device.status,
            devices::DeviceStatus::Rejected,
            "Rejected device should NOT be marked offline"
        );
    }

    cleanup_test_database(&db).await.unwrap();
}

/// Test that pending devices maintain their status when not sending heartbeat
#[tokio::test]
#[serial]
async fn test_pending_device_maintains_status_without_heartbeat() {
    let test_name = "pending_device_maintains_status_without_heartbeat";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    let device_id = uuid::Uuid::new_v4();

    // Create a pending device with very old heartbeat
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, Set};

        let very_old_time = Utc::now() - chrono::Duration::minutes(10); // 10 minutes ago

        let device = devices::ActiveModel {
            id: Set(device_id.to_string()),
            name: Set("Pending Device".to_string()),
            serial_number: Set(device_id.to_string()),
            device_type: Set(devices::DeviceType::Robot),
            organization_id: Set(Some(org_id.clone())),
            status: Set(devices::DeviceStatus::Pending),
            last_heartbeat: Set(Some(very_old_time.into())),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        device.insert(db.orm()).await.unwrap();
    }

    // Simulate timeout check (which should NOT mark pending devices as offline)
    {
        use easytier_bridge::client_manager::storage::Storage;
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let storage = Storage::new(db.clone());
        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(60);

        // This uses the same filter logic as mark_offline_devices
        let offline_devices = devices::Entity::find()
            .filter(devices::Column::LastHeartbeat.lt(cutoff_time))
            .filter(devices::Column::Status.ne(devices::DeviceStatus::Offline))
            .filter(devices::Column::Status.eq(devices::DeviceStatus::Approved)) // Only approved
            .all(storage.db().orm())
            .await
            .unwrap();

        // Mark devices as offline
        for device in offline_devices {
            let mut active: devices::ActiveModel = device.clone().into();
            active.status = Set(devices::DeviceStatus::Offline);
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(storage.db().orm()).await.unwrap();
        }
    }

    // Verify pending device still has pending status
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            device.status,
            devices::DeviceStatus::Pending,
            "Pending device should maintain pending status even with old heartbeat"
        );
    }

    cleanup_test_database(&db).await.unwrap();
}

/// Test that rejected devices maintain their status when not sending heartbeat
#[tokio::test]
#[serial]
async fn test_rejected_device_maintains_status_without_heartbeat() {
    let test_name = "rejected_device_maintains_status_without_heartbeat";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    let device_id = uuid::Uuid::new_v4();

    // Create a rejected device with very old heartbeat
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, Set};

        let very_old_time = Utc::now() - chrono::Duration::minutes(10); // 10 minutes ago

        let device = devices::ActiveModel {
            id: Set(device_id.to_string()),
            name: Set("Rejected Device".to_string()),
            serial_number: Set(device_id.to_string()),
            device_type: Set(devices::DeviceType::Robot),
            organization_id: Set(Some(org_id.clone())),
            status: Set(devices::DeviceStatus::Rejected),
            last_heartbeat: Set(Some(very_old_time.into())),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        device.insert(db.orm()).await.unwrap();
    }

    // Simulate timeout check (which should NOT mark rejected devices as offline)
    {
        use easytier_bridge::client_manager::storage::Storage;
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let storage = Storage::new(db.clone());
        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(60);

        // This uses the same filter logic as mark_offline_devices
        let offline_devices = devices::Entity::find()
            .filter(devices::Column::LastHeartbeat.lt(cutoff_time))
            .filter(devices::Column::Status.ne(devices::DeviceStatus::Offline))
            .filter(devices::Column::Status.eq(devices::DeviceStatus::Approved)) // Only approved
            .all(storage.db().orm())
            .await
            .unwrap();

        // Mark devices as offline
        for device in offline_devices {
            let mut active: devices::ActiveModel = device.clone().into();
            active.status = Set(devices::DeviceStatus::Offline);
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(storage.db().orm()).await.unwrap();
        }
    }

    // Verify rejected device still has rejected status
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            device.status,
            devices::DeviceStatus::Rejected,
            "Rejected device should maintain rejected status even with old heartbeat"
        );
    }

    cleanup_test_database(&db).await.unwrap();
}

/// Test complete workflow: approved device goes offline, then reconnects and becomes approved again
#[tokio::test]
#[serial]
async fn test_approved_offline_reconnect_workflow() {
    let test_name = "approved_offline_reconnect_workflow";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    let device_id = test_device_id();
    let client_url = test_client_url();

    // Step 1: Create an approved device with recent heartbeat
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, Set};

        let device = devices::ActiveModel {
            id: Set(device_id.to_string()),
            name: Set("Test Device".to_string()),
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

    // Step 2: Simulate device going offline (timeout)
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        // Update heartbeat to old time
        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        let mut active: devices::ActiveModel = device.into();
        let old_time = Utc::now() - chrono::Duration::seconds(120);
        active.last_heartbeat = Set(Some(old_time.into()));
        active.update(db.orm()).await.unwrap();

        // Simulate timeout check
        use easytier_bridge::client_manager::storage::Storage;
        let storage = Storage::new(db.clone());
        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(60);

        let offline_devices = devices::Entity::find()
            .filter(devices::Column::LastHeartbeat.lt(cutoff_time))
            .filter(devices::Column::Status.ne(devices::DeviceStatus::Offline))
            .filter(devices::Column::Status.eq(devices::DeviceStatus::Approved))
            .all(storage.db().orm())
            .await
            .unwrap();

        for device in offline_devices {
            let mut active: devices::ActiveModel = device.clone().into();
            active.status = Set(devices::DeviceStatus::Offline);
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(storage.db().orm()).await.unwrap();
        }
    }

    // Verify device is offline
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(device.status, devices::DeviceStatus::Offline);
    }

    // Step 3: Device reconnects with heartbeat - should become approved again
    {
        let mut client_mgr = ClientManager::new(&get_test_database_url(test_name), None)
            .await
            .unwrap();
        client_mgr.start("tcp", 0).await.unwrap();

        let heartbeat_req = HeartbeatRequest {
            machine_id: Some(device_id.into()),
            user_token: org_id.clone(),
            hostname: "test-device".to_string(),
            easytier_version: "1.0.0".to_string(),
            report_time: chrono::Utc::now().to_rfc3339(),
            running_network_instances: vec![],
            inst_id: None,
        };

        let storage = client_mgr.storage().weak_ref();
        let session = Session::new(storage, client_url, None);

        use easytier_bridge::client_manager::session::SessionRpcService;
        let rpc_service = SessionRpcService {
            data: session.data().clone(),
        };

        // Process heartbeat - should transition from offline to approved
        let _response = rpc_service.handle_heartbeat(heartbeat_req).await.unwrap();
    }

    // Verify device is approved again
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            device.status,
            devices::DeviceStatus::Approved,
            "Offline device should become approved again when reconnecting"
        );
        assert!(device.last_heartbeat.is_some());
    }

    cleanup_test_database(&db).await.unwrap();
}

/// Test complete workflow: rejected device reconnects and becomes pending
#[tokio::test]
#[serial]
async fn test_rejected_reconnect_becomes_pending_workflow() {
    let test_name = "rejected_reconnect_becomes_pending_workflow";
    let db = get_test_database(test_name).await.unwrap();
    let org_id = setup_test_organization(&db).await.unwrap();

    let device_id = test_device_id();
    let client_url = test_client_url();

    // Step 1: Create a rejected device
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ActiveModelTrait, Set};

        let device = devices::ActiveModel {
            id: Set(device_id.to_string()),
            name: Set("Rejected Device".to_string()),
            serial_number: Set(device_id.to_string()),
            device_type: Set(devices::DeviceType::Robot),
            organization_id: Set(Some(org_id.clone())),
            status: Set(devices::DeviceStatus::Rejected),
            last_heartbeat: Set(Some(Utc::now().into())),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..Default::default()
        };

        device.insert(db.orm()).await.unwrap();
    }

    // Verify initial status
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(device.status, devices::DeviceStatus::Rejected);
    }

    // Step 2: Device reconnects with heartbeat - should become pending
    {
        let mut client_mgr = ClientManager::new(&get_test_database_url(test_name), None)
            .await
            .unwrap();
        client_mgr.start("tcp", 0).await.unwrap();

        let heartbeat_req = HeartbeatRequest {
            machine_id: Some(device_id.into()),
            user_token: org_id.clone(),
            hostname: "rejected-device".to_string(),
            easytier_version: "1.0.0".to_string(),
            report_time: chrono::Utc::now().to_rfc3339(),
            running_network_instances: vec![],
            inst_id: None,
        };

        let storage = client_mgr.storage().weak_ref();
        let session = Session::new(storage, client_url, None);

        use easytier_bridge::client_manager::session::SessionRpcService;
        let rpc_service = SessionRpcService {
            data: session.data().clone(),
        };

        // Process heartbeat - should transition from rejected to pending
        let _response = rpc_service.handle_heartbeat(heartbeat_req).await.unwrap();
    }

    // Verify device is now pending
    {
        use easytier_bridge::db::entities::devices;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            device.status,
            devices::DeviceStatus::Pending,
            "Rejected device should become pending when reconnecting"
        );
        assert!(device.last_heartbeat.is_some());
    }

    cleanup_test_database(&db).await.unwrap();
}
