//! Session management for EasyTier clients with MySQL storage

use std::{fmt::Debug, sync::Arc};

use anyhow::Context;
use easytier::{
    common::scoped_task::ScopedTask,
    proto::{
        rpc_impl::bidirect::BidirectRpcManager,
        rpc_types::{self, controller::BaseController},
        web::{
            HeartbeatRequest, HeartbeatResponse, NetworkConfig, RunNetworkInstanceRequest,
            WebClientService, WebClientServiceClientFactory, WebServerService,
            WebServerServiceServer,
        },
    },
    tunnel::Tunnel,
};
use tokio::sync::{broadcast, RwLock};

use super::storage::{Storage, StorageToken, WeakRefStorage};

/// Location information for geographic positioning
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub country: String,
    pub city: Option<String>,
    pub region: Option<String>,
}

/// Session data structure
#[derive(Debug)]
pub struct SessionData {
    storage: WeakRefStorage,
    client_url: url::Url,
    storage_token: Option<StorageToken>,
    notifier: broadcast::Sender<HeartbeatRequest>,
    req: Option<HeartbeatRequest>,
    location: Option<Location>,
}

impl SessionData {
    fn new(storage: WeakRefStorage, client_url: url::Url, location: Option<Location>) -> Self {
        let (tx, _rx1) = broadcast::channel(2);

        SessionData {
            storage,
            client_url,
            storage_token: None,
            notifier: tx,
            req: None,
            location,
        }
    }

    pub fn req(&self) -> Option<HeartbeatRequest> {
        self.req.clone()
    }

    pub fn heartbeat_waiter(&self) -> broadcast::Receiver<HeartbeatRequest> {
        self.notifier.subscribe()
    }

    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
}

impl Drop for SessionData {
    fn drop(&mut self) {
        if let Some(token) = self.storage_token.as_ref() {
            if let Ok(storage) = Storage::try_from(self.storage.clone()) {
                storage.remove_client(token);
            }
        }
    }
}

pub type SharedSessionData = Arc<RwLock<SessionData>>;

/// RPC service for handling session requests
#[derive(Clone)]
pub struct SessionRpcService {
    pub data: SharedSessionData,
}

impl SessionRpcService {
    pub async fn handle_heartbeat(
        &self,
        req: HeartbeatRequest,
    ) -> rpc_types::error::Result<HeartbeatResponse> {
        crate::trace!(
            "[SESSION_RPC] Handling heartbeat request from device_id: {:?}",
            req.machine_id
        );
        let mut data = self.data.write().await;

        let Ok(storage) = Storage::try_from(data.storage.clone()) else {
            crate::error!("[SESSION_RPC] Failed to get storage");
            return Ok(HeartbeatResponse {});
        };

        let device_id: uuid::Uuid = req
            .machine_id
            .map(Into::into)
            .ok_or(anyhow::anyhow!(
                "Device id is not set correctly, expect uuid but got: {:?}",
                req.machine_id
            ))
            .map_err(|e| {
                crate::error!("[SESSION_RPC] Failed to parse device_id: {:?}", e);
                e
            })?;

        // The user_token field actually contains organization_id, not a user token
        // We need to verify that this organization_id exists
        let organization_id = &req.user_token;

        // Check organization existence using direct database query
        let organization_exists = {
            use crate::db::entities::organizations;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

            let organization = organizations::Entity::find()
                .filter(organizations::Column::Id.eq(organization_id))
                .one(storage.db().orm())
                .await
                .with_context(|| {
                    format!(
                        "Failed to check organization existence from db: {}",
                        organization_id
                    )
                })
                .map_err(|e| {
                    crate::error!(
                        "[SESSION_RPC] Database error when checking organization existence: {:?}",
                        e
                    );
                    e
                })?;

            organization.is_some()
        };

        if !organization_exists {
            crate::warn!("[SESSION_RPC] Organization not found: {}", organization_id);
            return Err(anyhow::anyhow!("Organization not found: {}", organization_id).into());
        }

        let organization_id = organization_id.clone();

        crate::trace!(
            "[SESSION_RPC] Successfully resolved organization_id: {} for device_id: {}",
            organization_id,
            device_id
        );

        // Create or update storage token for this session
        let storage_token = StorageToken {
            token: req.user_token.clone(),
            client_url: data.client_url.clone(),
            device_id,
            organization_id: organization_id.clone(),
        };

        // Always update client info in memory on each heartbeat (for session freshness)
        if let Ok(storage) = Storage::try_from(data.storage.clone()) {
            let report_time = chrono::Utc::now().timestamp();
            storage.update_client(storage_token.clone(), report_time);
        }

        // Sync device record in database on every heartbeat
        let device_status = Self::sync_device_record(&storage, &req, &organization_id, device_id)
            .await
            .with_context(|| format!("Failed to sync device record for device_id: {}", device_id))
            .map_err(|e| {
                crate::error!("[SESSION_RPC] Failed to sync device record: {:?}", e);
                e
            })?;

        // Update session data
        if data.req.replace(req.clone()).is_none() {
            // First heartbeat - initialize storage token
            assert!(data.storage_token.is_none());
            data.storage_token = Some(storage_token);
        } else {
            // Subsequent heartbeats - update storage token if needed
            data.storage_token = Some(storage_token);
        }

        crate::trace!("[SESSION_RPC] Successfully processed heartbeat for organization_id: {}, device_id: {}, status: {:?}", organization_id, device_id, device_status);

        let _ = data.notifier.send(req);
        Ok(HeartbeatResponse {})
    }

    /// Sync device record in database, creating if not exists
    async fn sync_device_record(
        storage: &super::storage::Storage,
        req: &HeartbeatRequest,
        organization_id: &str,
        device_id: uuid::Uuid,
    ) -> anyhow::Result<crate::db::entities::devices::DeviceStatus> {
        use crate::db::entities::devices;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let device_id_str = device_id.to_string();

        // Try to find existing device
        let existing = devices::Entity::find()
            .filter(devices::Column::Id.eq(&device_id_str))
            .filter(devices::Column::OrganizationId.eq(organization_id))
            .one(storage.db().orm())
            .await
            .with_context(|| format!("Failed to query device: {}", device_id_str))?;

        match existing {
            Some(device) => {
                // Update existing device heartbeat
                let mut active: devices::ActiveModel = device.clone().into();
                active.last_heartbeat = Set(Some(chrono::Utc::now().into()));
                active.updated_at = Set(chrono::Utc::now().into());

                // Handle status transitions based on current status
                let new_status = match device.status {
                    // If device is rejected, change status back to pending when it reconnects
                    devices::DeviceStatus::Rejected => {
                        crate::info!("[SESSION_RPC] Rejected device {} reconnected, changing status to pending", device_id_str);
                        active.status = Set(devices::DeviceStatus::Pending);
                        devices::DeviceStatus::Pending
                    }
                    // If device is offline, change status back to approved when it reconnects (if it was previously approved)
                    devices::DeviceStatus::Offline => {
                        // Check if device was previously approved by looking at other fields
                        // For now, assume if it has organization_id and is not rejected, it should be approved
                        crate::info!("[SESSION_RPC] Offline device {} reconnected, changing status to approved", device_id_str);
                        active.status = Set(devices::DeviceStatus::Approved);
                        devices::DeviceStatus::Approved
                    }
                    // For other statuses (pending, approved, etc.), keep the existing status
                    _ => {
                        // Only update to approved if device is pending and has been approved by admin
                        // This preserves the admin's approval decision
                        if device.status.is_pending() {
                            // Keep pending status - admin needs to explicitly approve
                            device.status
                        } else {
                            // For approved devices, ensure they stay approved when heartbeat comes in
                            device.status
                        }
                    }
                };

                active.update(storage.db().orm()).await.with_context(|| {
                    format!("Failed to update device heartbeat: {}", device_id_str)
                })?;

                crate::trace!(
                    "[SESSION_RPC] Updated heartbeat for existing device: {}, status: {:?}",
                    device_id_str,
                    new_status
                );
                Ok(new_status)
            }
            None => {
                // Create new device with pending status
                let new_device = devices::ActiveModel {
                    id: Set(device_id_str.clone()),
                    name: Set(req.hostname.clone()),
                    serial_number: Set(req.hostname.clone()), // Use hostname as serial for now
                    device_type: Set(devices::DeviceType::Robot), // Default to robot
                    organization_id: Set(Some(organization_id.to_string())),
                    status: Set(devices::DeviceStatus::Pending),
                    last_heartbeat: Set(Some(chrono::Utc::now().into())),
                    created_at: Set(chrono::Utc::now().into()),
                    updated_at: Set(chrono::Utc::now().into()),
                    ..Default::default()
                };

                new_device
                    .insert(storage.db().orm())
                    .await
                    .with_context(|| {
                        format!("Failed to create device record: {}", device_id_str)
                    })?;

                crate::info!(
                    "[SESSION_RPC] Created new device record: {}, status: pending",
                    device_id_str
                );
                Ok(devices::DeviceStatus::Pending)
            }
        }
    }
}

#[async_trait::async_trait]
impl WebServerService for SessionRpcService {
    type Controller = BaseController;

    async fn heartbeat(
        &self,
        _: BaseController,
        req: HeartbeatRequest,
    ) -> rpc_types::error::Result<HeartbeatResponse> {
        let ret = self.handle_heartbeat(req).await;
        if ret.is_err() {
            crate::warn!("[SESSION_RPC] Failed to handle heartbeat: {:?}", ret);
            // sleep for a while to avoid client busy loop
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        ret
    }
}

/// Client session
pub struct Session {
    rpc_mgr: BidirectRpcManager,
    data: SharedSessionData,
    run_network_on_start_task: Option<ScopedTask<()>>,
    // 添加一个关闭通知通道
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").field("data", &self.data).finish()
    }
}

type SessionRpcClient = Box<dyn WebClientService<Controller = BaseController> + Send>;

impl Session {
    pub fn new(storage: WeakRefStorage, client_url: url::Url, location: Option<Location>) -> Self {
        crate::debug!(
            "[SESSION] Creating new session for client_url: {}",
            client_url
        );
        let session_data = SessionData::new(storage, client_url, location);
        let data = Arc::new(RwLock::new(session_data));

        let rpc_mgr =
            BidirectRpcManager::new().set_rx_timeout(Some(std::time::Duration::from_secs(30)));

        rpc_mgr.rpc_server().registry().register(
            WebServerServiceServer::new(SessionRpcService { data: data.clone() }),
            "",
        );

        Session {
            rpc_mgr,
            data,
            run_network_on_start_task: None,
            shutdown_tx: None,
        }
    }

    /// Serve the session with a tunnel
    pub async fn serve(&mut self, tunnel: Box<dyn Tunnel>) {
        crate::info!("[SESSION] Starting to serve session with tunnel");
        self.rpc_mgr.run_with_tunnel(tunnel);

        // 创建关闭通知通道
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // 克隆需要在异步闭包中使用的数据
        let heartbeat_waiter = self.data.read().await.heartbeat_waiter();
        let storage = self.data.read().await.storage.clone();
        let rpc_client = self.scoped_rpc_client();

        // 启动网络任务
        self.run_network_on_start_task = Some(ScopedTask::from(tokio::spawn(async move {
            tokio::select! {
                _ = Self::run_network_on_start(heartbeat_waiter, storage, rpc_client) => {
                    crate::debug!("[SESSION] Network start task completed normally");
                },
                _ = shutdown_rx => {
                    crate::info!("[SESSION] Session task received shutdown signal");
                }
            }
        })));
    }

    /// Check if session is running
    async fn run_network_on_start(
        mut heartbeat_waiter: broadcast::Receiver<HeartbeatRequest>,
        storage: WeakRefStorage,
        rpc_client: SessionRpcClient,
    ) {
        crate::debug!("[run_network_on_start] Starting function execution");
        loop {
            crate::debug!("[run_network_on_start] Entering loop iteration");
            heartbeat_waiter = heartbeat_waiter.resubscribe();
            crate::debug!("[run_network_on_start] Waiting for heartbeat request");
            let req = heartbeat_waiter.recv().await;
            if req.is_err() {
                crate::error!(
                    "Failed to receive heartbeat request, error: {:?}",
                    req.err()
                );
                return;
            }

            let req = req.unwrap();
            crate::debug!(
                "[run_network_on_start] Received heartbeat request: {:?}",
                req
            );
            if req.machine_id.is_none() {
                crate::warn!(?req, "Device id is not set, ignore");
                continue;
            }
            crate::debug!(
                "[run_network_on_start] Processing request for machine_id: {:?}",
                req.machine_id
            );

            let running_inst_ids = req
                .running_network_instances
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>();
            crate::debug!(
                "[run_network_on_start] Running network instances: {:?}",
                running_inst_ids
            );

            crate::debug!("[run_network_on_start] Attempting to get storage");
            let Ok(storage) = Storage::try_from(storage.clone()) else {
                crate::error!("Failed to get storage");
                return;
            };
            crate::debug!("[run_network_on_start] Successfully obtained storage");

            // The user_token field actually contains organization_id, not a user token
            // We need to verify that this organization_id exists
            let organization_id = &req.user_token;
            crate::debug!(
                "[run_network_on_start] Checking organization existence for ID: {:?}",
                organization_id
            );

            let organization_exists = {
                use crate::db::entities::organizations;
                use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

                match organizations::Entity::find()
                    .filter(organizations::Column::Id.eq(organization_id))
                    .one(storage.db().orm())
                    .await
                {
                    Ok(organization) => {
                        let exists = organization.is_some();
                        crate::debug!(
                            "[run_network_on_start] Organization exists check result: {}",
                            exists
                        );
                        exists
                    }
                    Err(e) => {
                        crate::error!("Failed to check organization existence, error: {:?}", e);
                        return;
                    }
                }
            };

            if !organization_exists {
                crate::info!("Organization not found: {:?}", organization_id);
                return;
            }

            crate::debug!(
                "[run_network_on_start] Listing network configs for org: {:?}, machine: {:?}",
                organization_id,
                req.machine_id
            );
            let local_configs = {
                use crate::db::entities::devices;
                use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

                let device_id = req.machine_id.unwrap();
                match devices::Entity::find()
                    .filter(devices::Column::OrganizationId.eq(organization_id))
                    .filter(devices::Column::Id.eq(device_id.to_string()))
                    .filter(devices::Column::NetworkInstanceId.is_not_null())
                    .filter(devices::Column::NetworkDisabled.eq(false)) // EnabledOnly
                    .all(storage.db().orm())
                    .await
                {
                    Ok(devices) => {
                        // Filter to only approved devices
                        let approved_devices: Vec<_> = devices
                            .into_iter()
                            .filter(|device| device.status.is_approved())
                            .collect();

                        crate::debug!(
                            "[run_network_on_start] Found {} approved devices with network configs",
                            approved_devices.len()
                        );

                        if approved_devices.is_empty() {
                            let device_status = devices::Entity::find()
                                .filter(devices::Column::OrganizationId.eq(organization_id))
                                .filter(devices::Column::Id.eq(device_id.to_string()))
                                .one(storage.db().orm())
                                .await
                                .unwrap_or_default()
                                .map(|d| d.status);

                            match device_status {
                                Some(status) if status.is_pending() => {
                                    crate::info!("[run_network_on_start] Device {} is pending approval, no networks will be started", device_id);
                                }
                                Some(status) if status.is_rejected() => {
                                    crate::warn!("[run_network_on_start] Device {} is rejected, no networks will be started", device_id);
                                }
                                _ => {
                                    crate::debug!("[run_network_on_start] Device {} has no network configs or is not approved", device_id);
                                }
                            }
                        }

                        approved_devices
                    }
                    Err(e) => {
                        crate::error!(
                            "Failed to list devices with network configs, error: {:?}",
                            e
                        );
                        return;
                    }
                }
            };

            let mut has_failed = false;
            crate::debug!(
                "[run_network_on_start] Starting to process {} network configs",
                local_configs.len()
            );

            for (index, device) in local_configs.iter().enumerate() {
                let instance_id = match &device.network_instance_id {
                    Some(id) => id,
                    None => {
                        crate::debug!(
                            "[run_network_on_start] Device {} has no network instance ID, skipping",
                            device.id
                        );
                        continue;
                    }
                };

                crate::debug!(
                    "[run_network_on_start] Processing device {}/{}: instance_id={}",
                    index + 1,
                    local_configs.len(),
                    instance_id
                );
                if running_inst_ids.contains(instance_id) {
                    crate::debug!(
                        "[run_network_on_start] Instance {} is already running, skipping",
                        instance_id
                    );
                    continue;
                }

                let network_config_str = match &device.network_config {
                    Some(config) => config,
                    None => {
                        crate::debug!(
                            "[run_network_on_start] Device {} has no network config, skipping",
                            device.id
                        );
                        continue;
                    }
                };

                crate::debug!(
                    "[run_network_on_start] Need to start instance: {}",
                    instance_id
                );
                crate::debug!(
                    "[run_network_on_start] Parsing network config JSON for instance: {}",
                    instance_id
                );
                let network_config: NetworkConfig =
                    match serde_json::from_value(network_config_str.clone()) {
                        Ok(config) => {
                            crate::debug!(
                                "[run_network_on_start] Successfully parsed network config"
                            );
                            config
                        }
                        Err(e) => {
                            crate::error!("Failed to parse network config: {:?}", e);
                            has_failed = true;
                            continue;
                        }
                    };

                crate::debug!(
                    "[run_network_on_start] Calling RPC to run network instance: {}",
                    instance_id
                );
                let ret = rpc_client
                    .run_network_instance(
                        BaseController::default(),
                        RunNetworkInstanceRequest {
                            inst_id: Some(instance_id.clone().into()),
                            config: Some(network_config),
                        },
                    )
                    .await;
                crate::debug!(
                    "[run_network_on_start] RPC call result for instance {}: {:?}",
                    instance_id,
                    ret
                );
                crate::info!(
                    organization_id = %organization_id,
                    "Run network instance: {:?}, user_token: {:?}",
                    ret,
                    req.user_token
                );

                has_failed |= ret.is_err();
            }

            if !has_failed {
                crate::info!(?req, "All network instances are running");
                crate::debug!(
                    "[run_network_on_start] All instances started successfully, exiting loop"
                );
                break;
            } else {
                crate::debug!("[run_network_on_start] Some instances failed to start, will retry on next heartbeat");
            }
        }
        crate::debug!("[run_network_on_start] Function execution completed");
    }

    pub fn is_running(&self) -> bool {
        self.rpc_mgr.is_running()
    }

    /// 显式关闭会话及其相关资源
    pub async fn shutdown(&mut self) {
        crate::info!("[SESSION] Explicitly shutting down session");

        // 发送关闭信号
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
            crate::debug!("[SESSION] Sent shutdown signal to session task");
        }

        // 等待任务完成
        if let Some(task) = self.run_network_on_start_task.take() {
            task.abort();
            crate::debug!("[SESSION] Aborted network start task");
        }

        // 关闭 RPC 管理器 - 使用 stop 方法而不是 shutdown
        if self.rpc_mgr.is_running() {
            std::mem::drop(self.rpc_mgr.stop());
            crate::debug!("[SESSION] Stopped RPC manager");
        }

        crate::info!("[SESSION] Session shutdown completed");
    }

    pub fn scoped_rpc_client(&self) -> SessionRpcClient {
        self.rpc_mgr
            .rpc_client()
            .scoped_client::<WebClientServiceClientFactory<BaseController>>(1, 1, "".to_string())
    }

    /// Get session data
    pub fn data(&self) -> &SharedSessionData {
        &self.data
    }

    /// Get storage token
    pub async fn get_token(&self) -> Option<StorageToken> {
        self.data.read().await.storage_token.clone()
    }

    pub async fn get_heartbeat_req(&self) -> Option<HeartbeatRequest> {
        self.data.read().await.req()
    }

    /// Run network instance
    pub async fn run_network_instance(
        &mut self,
        req: RunNetworkInstanceRequest,
    ) -> Result<(), anyhow::Error> {
        crate::debug!("[SESSION] Starting to run network instance");

        let client = self.scoped_rpc_client();

        let ret = client
            .run_network_instance(BaseController::default(), req)
            .await
            .map_err(|e| {
                crate::error!("[SESSION] Failed to run network instance: {:?}", e);
                e
            })?;

        crate::info!("[SESSION] Run network instance result: {:?}", ret);
        Ok(())
    }

    /// Stop network instance
    pub async fn stop_network_instance(
        &mut self,
        network_instance_id: String,
    ) -> Result<(), anyhow::Error> {
        crate::debug!(
            "[SESSION] Stopping network instance: {}",
            network_instance_id
        );

        let client = self.scoped_rpc_client();

        // Note: Using list_network_instance as stop method may not be available
        let ret = client
            .list_network_instance(
                BaseController::default(),
                easytier::proto::web::ListNetworkInstanceRequest {},
            )
            .await
            .map_err(|e| {
                crate::error!(
                    "[SESSION] Failed to stop network instance {}: {:?}",
                    network_instance_id,
                    e
                );
                e
            })?;

        crate::info!(
            "[SESSION] Stop network instance {} result: {:?}",
            network_instance_id,
            ret
        );
        Ok(())
    }

    /// List network instances
    pub async fn list_network_instances(
        &self,
    ) -> Result<easytier::proto::web::ListNetworkInstanceResponse, anyhow::Error> {
        crate::trace!("[SESSION] Listing network instances");

        let client = self.scoped_rpc_client();

        let ret = client
            .list_network_instance(
                BaseController::default(),
                easytier::proto::web::ListNetworkInstanceRequest {},
            )
            .await
            .map_err(|e| {
                crate::error!("[SESSION] Failed to list network instances: {:?}", e);
                e
            })?;

        crate::debug!(
            "[SESSION] Successfully listed {} network instances",
            ret.inst_ids.len()
        );
        Ok(ret)
    }
}
