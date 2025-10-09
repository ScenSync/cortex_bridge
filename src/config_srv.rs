use std::sync::Arc;

use anyhow::Result;
use easytier::launcher::NetworkConfig;
// 移除未使用的导入
use easytier::proto::rpc_types::controller::BaseController;
use easytier::proto::web::*;

use crate::client_manager::session::{Location, Session};
use crate::client_manager::ClientManager;
// Removed DatabaseExt and ListNetworkProps - using direct database calls instead
use crate::db::OrgIdInDb;

/// 网络配置服务，提供网络配置的管理功能
pub struct NetworkConfigService {
    client_mgr: Arc<ClientManager>,
}

/// RPC 错误转换为 anyhow::Error
fn convert_rpc_error(e: impl std::fmt::Debug) -> anyhow::Error {
    anyhow::anyhow!("RPC error: {:?}", e)
}

/// 网络实例 ID 列表响应
#[derive(Debug, serde::Serialize)]
pub struct NetworkInstanceIds {
    pub running_inst_ids: Vec<uuid::Uuid>,
    pub disabled_inst_ids: Vec<uuid::Uuid>,
}

/// 设备信息项
#[derive(Debug, serde::Serialize)]
pub struct DeviceItem {
    pub client_url: Option<url::Url>,
    pub info: Option<SerializableHeartbeatRequest>,
    pub location: Option<Location>,
}

/// Serializable version of HeartbeatRequest that converts ProtoUuid to string
#[derive(Debug, serde::Serialize)]
pub struct SerializableHeartbeatRequest {
    pub machine_id: Option<String>,
    pub inst_id: Option<String>,
    pub user_token: String,
    pub easytier_version: String,
    pub report_time: String,
    pub hostname: String,
    pub running_network_instances: Vec<String>,
}

impl From<HeartbeatRequest> for SerializableHeartbeatRequest {
    fn from(req: HeartbeatRequest) -> Self {
        SerializableHeartbeatRequest {
            machine_id: req.machine_id.map(|id| id.to_string()),
            inst_id: req.inst_id.map(|id| id.to_string()),
            user_token: req.user_token,
            easytier_version: req.easytier_version,
            report_time: req.report_time,
            hostname: req.hostname,
            running_network_instances: req
                .running_network_instances
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
        }
    }
}

/// 设备列表响应
#[derive(Debug, serde::Serialize)]
pub struct DeviceList {
    pub devices: Vec<DeviceItem>,
}

impl NetworkConfigService {
    /// 创建新的网络配置服务，同时创建新的 ClientManager
    pub async fn new(db_url: &str, geoip_path: Option<String>) -> Result<Self> {
        let client_mgr = ClientManager::new(db_url, geoip_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create ClientManager: {:?}", e))?;

        Ok(Self {
            client_mgr: Arc::new(client_mgr),
        })
    }

    /// 启动网络配置服务的监听器
    pub async fn start(&mut self, protocol: &str, port: u16) -> Result<()> {
        let client_mgr = Arc::get_mut(&mut self.client_mgr)
            .ok_or_else(|| anyhow::anyhow!("Cannot get mutable reference to ClientManager"))?;

        client_mgr
            .start(protocol, port)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start listener: {:?}", e))
    }

    /// 根据设备 ID 获取会话
    async fn get_session_by_device_id(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
    ) -> Result<Arc<Session>> {
        let Some(result) = self
            .client_mgr
            .get_session_by_device_id(user_id, device_id)
            .await
        else {
            return Err(anyhow::anyhow!("No such session: {}", device_id));
        };

        let Some(_token) = result.get_token().await else {
            return Err(anyhow::anyhow!("No token reported"));
        };

        Ok(result)
    }

    /// 验证网络配置
    pub async fn validate_config(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        config: NetworkConfig,
    ) -> Result<ValidateConfigResponse> {
        let result = self.get_session_by_device_id(user_id, device_id).await?;

        let c = result.scoped_rpc_client();
        let ret = c
            .validate_config(
                BaseController::default(),
                ValidateConfigRequest {
                    config: Some(config),
                },
            )
            .await
            .map_err(convert_rpc_error)?;
        Ok(ret)
    }

    /// 运行网络实例
    pub async fn run_network_instance(
        &self,
        org_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        config: NetworkConfig,
    ) -> Result<uuid::Uuid> {
        // Debug: Print network config for debugging
        crate::info!("Running network instance with config: {:?}", config);

        let result = self.get_session_by_device_id(org_id, device_id).await?;

        let c = result.scoped_rpc_client();
        let resp = c
            .run_network_instance(
                BaseController::default(),
                RunNetworkInstanceRequest {
                    inst_id: None,
                    config: Some(config.clone()),
                },
            )
            .await
            .map_err(convert_rpc_error)?;

        let inst_id: uuid::Uuid = resp.inst_id.unwrap_or_default().into();

        let db = self.client_mgr.db().await;
        // Direct database operation - update device network config
        {
            use crate::db::entities::devices;
            use chrono::Utc;
            use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

            let existing_device = devices::Entity::find()
                .filter(devices::Column::Id.eq(device_id.to_string()))
                .filter(devices::Column::OrganizationId.eq(org_id))
                .one(db.orm())
                .await?;

            crate::info!("Found existing device: {:?}", existing_device.is_some());

            if let Some(existing) = existing_device {
                crate::info!("Updating existing device network config in database");
                let mut active_model: devices::ActiveModel = existing.into();
                active_model.network_instance_id = Set(Some(inst_id.to_string()));
                active_model.network_config = Set(Some(serde_json::to_value(&config)?));
                active_model.network_disabled = Set(Some(false));
                active_model.network_update_time = Set(Some(Utc::now().into()));

                active_model.update(db.orm()).await?;
                crate::info!("Successfully updated existing device network config in database");
            } else {
                crate::info!(
                    "Device not found in database, creating new device record: {}",
                    device_id
                );
                // 创建新的设备记录
                let new_device = devices::ActiveModel {
                    id: Set(device_id.to_string()),
                    organization_id: Set(Some(org_id.clone())),
                    name: Set(format!("Device-{}", device_id)),
                    serial_number: Set(device_id.to_string()),
                    device_type: Set(devices::DeviceType::Robot),
                    model: Set(Some("Unknown".to_string())),
                    status: Set(devices::DeviceStatus::Online),
                    capabilities: Set(Some(serde_json::json!("network"))),
                    network_instance_id: Set(Some(inst_id.to_string())),
                    network_config: Set(Some(serde_json::to_value(&config)?)),
                    network_disabled: Set(Some(false)),
                    network_create_time: Set(Some(Utc::now().into())),
                    network_update_time: Set(Some(Utc::now().into())),
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(Utc::now().into()),
                    ..Default::default()
                };

                new_device.insert(db.orm()).await?;
                crate::info!("Successfully created new device record in database");
            }
        }

        // Check if network instance is running before extracting virtual IP
        crate::info!(
            "Checking if network instance {} is running before extracting virtual IP",
            inst_id
        );
        match self
            .check_network_instance_running(org_id, device_id, &inst_id)
            .await
        {
            Ok(true) => {
                crate::info!(
                    "Network instance {} is running, extracting virtual IP",
                    inst_id
                );
                match self
                    .update_device_virtual_ip(org_id, device_id, &inst_id)
                    .await
                {
                    Ok(_) => crate::info!(
                        "Virtual IP update completed successfully for device {}",
                        device_id
                    ),
                    Err(e) => {
                        crate::error!("Virtual IP update failed for device {}: {:?}", device_id, e)
                    }
                }
            }
            Ok(false) => {
                crate::warn!(
                    "Network instance {} is not running, skipping virtual IP extraction",
                    inst_id
                );
            }
            Err(e) => {
                crate::error!("Failed to check network instance status: {:?}", e);
            }
        }

        Ok(inst_id)
    }

    /// 收集单个网络实例信息
    pub async fn collect_one_network_info(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_id: &uuid::Uuid,
    ) -> Result<CollectNetworkInfoResponse> {
        let result = self.get_session_by_device_id(user_id, device_id).await?;

        let c = result.scoped_rpc_client();
        let ret = c
            .collect_network_info(
                BaseController::default(),
                CollectNetworkInfoRequest {
                    inst_ids: vec![(*inst_id).into()],
                },
            )
            .await
            .map_err(convert_rpc_error)?;
        Ok(ret)
    }

    /// 收集多个网络实例信息
    pub async fn collect_network_info(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_ids: Option<Vec<uuid::Uuid>>,
    ) -> Result<CollectNetworkInfoResponse> {
        let result = self.get_session_by_device_id(user_id, device_id).await?;

        let c = result.scoped_rpc_client();
        let ret = c
            .collect_network_info(
                BaseController::default(),
                CollectNetworkInfoRequest {
                    inst_ids: inst_ids
                        .unwrap_or_default()
                        .into_iter()
                        .map(Into::into)
                        .collect(),
                },
            )
            .await
            .map_err(convert_rpc_error)?;
        Ok(ret)
    }

    /// 列出网络实例 ID
    pub async fn list_network_instance_ids(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
    ) -> Result<NetworkInstanceIds> {
        let result = self.get_session_by_device_id(user_id, device_id).await?;

        let c = result.scoped_rpc_client();
        let ret = c
            .list_network_instance(BaseController::default(), ListNetworkInstanceRequest {})
            .await
            .map_err(convert_rpc_error)?;

        let running_inst_ids = ret.inst_ids.clone().into_iter().map(Into::into).collect();

        // collect networks that are disabled
        let db = self.client_mgr.db().await;
        let disabled_inst_ids = {
            use crate::db::entities::devices;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

            devices::Entity::find()
                .filter(devices::Column::OrganizationId.eq(user_id))
                .filter(devices::Column::Id.eq(device_id.to_string()))
                .filter(devices::Column::NetworkDisabled.eq(true)) // DisabledOnly
                .all(db.orm())
                .await?
                .iter()
                .filter_map(|x| {
                    x.network_instance_id
                        .as_ref()
                        .and_then(|id| uuid::Uuid::parse_str(id).ok())
                })
                .collect::<Vec<_>>()
        };

        Ok(NetworkInstanceIds {
            running_inst_ids,
            disabled_inst_ids,
        })
    }

    /// 删除网络实例
    pub async fn remove_network_instance(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_id: &uuid::Uuid,
    ) -> Result<()> {
        let result = self.get_session_by_device_id(user_id, device_id).await?;

        let db = self.client_mgr.db().await;
        // Direct database operation - clear device network config
        {
            use crate::db::entities::devices;
            use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

            let device = devices::Entity::find()
                .filter(devices::Column::NetworkInstanceId.eq(inst_id.to_string()))
                .one(db.orm())
                .await?;

            if let Some(device) = device {
                let mut active_model: devices::ActiveModel = device.into();
                active_model.network_instance_id = Set(None);
                active_model.network_config = Set(None);
                active_model.network_disabled = Set(None);
                active_model.network_create_time = Set(None);
                active_model.network_update_time = Set(None);

                active_model.update(db.orm()).await?;
            }
        }

        let c = result.scoped_rpc_client();
        c.delete_network_instance(
            BaseController::default(),
            DeleteNetworkInstanceRequest {
                inst_ids: vec![(*inst_id).into()],
            },
        )
        .await
        .map_err(convert_rpc_error)?;
        Ok(())
    }

    /// 列出设备
    pub async fn list_devices(&self, user_id: &OrgIdInDb) -> Result<DeviceList> {
        let client_urls = self
            .client_mgr
            .list_devices_by_organization_id(user_id)
            .await;

        let mut devices = vec![];
        for item in client_urls.iter() {
            let client_url = item.clone();
            let heartbeat_request = self.client_mgr.get_heartbeat_requests(&client_url).await;
            let location = self.client_mgr.get_device_location(&client_url).await;
            devices.push(DeviceItem {
                client_url: Some(client_url),
                info: heartbeat_request.map(SerializableHeartbeatRequest::from),
                location,
            });
        }

        Ok(DeviceList { devices })
    }

    /// 更新网络状态
    pub async fn update_network_state(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_id: &uuid::Uuid,
        disabled: bool,
    ) -> Result<()> {
        let sess = self.get_session_by_device_id(user_id, device_id).await?;
        let db = self.client_mgr.db().await;
        // Direct database operation - update device network config state
        let device = {
            use crate::db::entities::devices;
            use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

            let device = devices::Entity::find()
                .filter(devices::Column::NetworkInstanceId.eq(inst_id.to_string()))
                .one(db.orm())
                .await?;

            let Some(device) = device else {
                return Err(anyhow::anyhow!("Device with network config not found"));
            };

            let mut active_model: devices::ActiveModel = device.into();
            active_model.network_disabled = Set(Some(disabled));

            active_model.update(db.orm()).await?
        };

        let c = sess.scoped_rpc_client();

        if disabled {
            c.delete_network_instance(
                BaseController::default(),
                DeleteNetworkInstanceRequest {
                    inst_ids: vec![(*inst_id).into()],
                },
            )
            .await
            .map_err(convert_rpc_error)?;
        } else {
            c.run_network_instance(
                BaseController::default(),
                RunNetworkInstanceRequest {
                    inst_id: Some((*inst_id).into()),
                    config: Some(serde_json::from_value(
                        device.network_config.as_ref().unwrap().clone(),
                    )?),
                },
            )
            .await
            .map_err(convert_rpc_error)?;
        }

        Ok(())
    }

    /// 获取网络配置
    pub async fn get_network_config(
        &self,
        _user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_id: &uuid::Uuid,
    ) -> Result<NetworkConfig> {
        let inst_id_str = inst_id.to_string();

        let db = self.client_mgr.db().await;
        // Direct database operation - get device network config
        let device = {
            use crate::db::entities::devices;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

            devices::Entity::find()
                .filter(devices::Column::Id.eq(device_id.to_string()))
                .filter(devices::Column::NetworkInstanceId.eq(&inst_id_str))
                .one(db.orm())
                .await?
        }
        .ok_or_else(|| anyhow::anyhow!("No such network instance: {}", inst_id_str))?;

        let config = serde_json::from_value::<NetworkConfig>(
            device.network_config.as_ref().unwrap().clone(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to parse network config: {:?}", e))?;

        Ok(config)
    }

    /// 批量更新网络状态
    pub async fn batch_update_network_state(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_ids: Vec<uuid::Uuid>,
        disabled: bool,
    ) -> Result<Vec<Result<(), anyhow::Error>>> {
        let mut results = Vec::with_capacity(inst_ids.len());

        for inst_id in inst_ids {
            let result = self
                .update_network_state(user_id, device_id, &inst_id, disabled)
                .await;
            results.push(result);
        }

        Ok(results)
    }

    /// 批量删除网络实例
    pub async fn batch_remove_network_instances(
        &self,
        user_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_ids: Vec<uuid::Uuid>,
    ) -> Result<Vec<Result<(), anyhow::Error>>> {
        let mut results = Vec::with_capacity(inst_ids.len());

        for inst_id in inst_ids {
            let result = self
                .remove_network_instance(user_id, device_id, &inst_id)
                .await;
            results.push(result);
        }

        Ok(results)
    }

    /// 检查网络实例是否正在运行
    async fn check_network_instance_running(
        &self,
        org_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_id: &uuid::Uuid,
    ) -> Result<bool> {
        // Collect network info to check if instance is running
        let network_info = self
            .collect_one_network_info(org_id, device_id, inst_id)
            .await?;

        // Convert to JSON and check the running status
        let json_value = serde_json::to_value(&network_info)?;
        let inst_id_str = inst_id.to_string();

        // Direct navigation using chained get() calls with fallback
        let is_running = json_value
            .get("info")
            .and_then(|info| info.get("map"))
            .and_then(|map| map.get(&inst_id_str))
            .and_then(|inst_data| inst_data.get("running"))
            .and_then(|running| running.as_bool())
            .unwrap_or(false);

        crate::debug!(
            "Network instance {} running status: {}",
            inst_id_str,
            is_running
        );
        Ok(is_running)
    }

    /// 更新设备虚拟IP（从网络信息中提取）
    async fn update_device_virtual_ip(
        &self,
        org_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
        inst_id: &uuid::Uuid,
    ) -> Result<()> {
        // Try multiple times with delays to allow virtual IP to be assigned
        for attempt in 1..=3 {
            crate::info!(
                "Attempt {}: Collecting network info for device {} instance {}",
                attempt,
                device_id,
                inst_id
            );

            // Wait a bit for the network to fully start (longer wait for first attempt)
            let wait_time = if attempt == 1 { 3 } else { 2 };
            tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;

            // Collect network info to extract virtual IP
            let network_info = self
                .collect_one_network_info(org_id, device_id, inst_id)
                .await?;

            // Extract virtual IP from the response
            crate::info!(
                "Attempt {}: Extracting virtual IP from network info for device {}",
                attempt,
                device_id
            );
            if let Some(virtual_ip_info) =
                self.extract_virtual_ip_from_network_info(&network_info, inst_id)
            {
                crate::info!(
                    "Found virtual IP: {}/{} for device {}",
                    virtual_ip_info.0,
                    virtual_ip_info.1,
                    device_id
                );
                // Update device virtual IP directly in database
                self.update_device_virtual_ip_in_db(
                    device_id,
                    virtual_ip_info.0,
                    virtual_ip_info.1,
                )
                .await?;
                crate::info!(
                    "Successfully updated device {} virtual IP: {}/{}",
                    device_id,
                    virtual_ip_info.0,
                    virtual_ip_info.1
                );
                return Ok(());
            } else {
                crate::warn!(
                    "Attempt {}: No virtual IP found in network info for device {}",
                    attempt,
                    device_id
                );
                // Debug: Print the network info structure to understand what we're getting
                crate::debug!(
                    "Attempt {}: Network info structure: {}",
                    attempt,
                    serde_json::to_string_pretty(&network_info).unwrap_or_default()
                );

                if attempt < 3 {
                    crate::info!("Waiting before retry...");
                }
            }
        }

        crate::error!(
            "Failed to extract virtual IP after 3 attempts for device {}",
            device_id
        );
        Ok(())
    }

    /// 从网络信息中提取虚拟IP
    fn extract_virtual_ip_from_network_info(
        &self,
        network_info: &CollectNetworkInfoResponse,
        inst_id: &uuid::Uuid,
    ) -> Option<(u32, u8)> {
        let inst_id_str = inst_id.to_string();
        crate::debug!("Extracting virtual IP for instance: {}", inst_id_str);

        // Convert the response to serde_json::Value for easier navigation
        let json_value = match serde_json::to_value(network_info) {
            Ok(v) => v,
            Err(e) => {
                crate::error!("Failed to convert network_info to JSON: {:?}", e);
                return None;
            }
        };

        // Validate the JSON path and extract virtual IP info
        let virtual_ip_info = match json_value
            .get("info")
            .and_then(|info| info.get("map"))
            .and_then(|map| map.get(&inst_id_str))
        {
            Some(info) => info,
            None => {
                crate::warn!("No data found for instance: {}", inst_id_str);
                return None;
            }
        };

        // Check if the network instance is running
        let running = virtual_ip_info
            .get("running")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);
        if !running {
            crate::warn!(
                "Network instance {} is not running, skipping virtual IP extraction",
                inst_id_str
            );
            return None;
        }
        // Extract virtual IP directly using chained get() calls
        let addr = virtual_ip_info
            .get("my_node_info")?
            .get("virtual_ipv4")?
            .get("address")?
            .get("addr")?
            .as_u64()?;

        let network_length = virtual_ip_info
            .get("my_node_info")?
            .get("virtual_ipv4")?
            .get("network_length")?
            .as_u64()?;

        crate::info!(
            "Successfully extracted virtual IP: {}/{}",
            addr,
            network_length
        );
        Some((addr as u32, network_length as u8))
    }

    /// 直接在数据库中更新设备虚拟IP
    async fn update_device_virtual_ip_in_db(
        &self,
        device_id: &uuid::Uuid,
        virtual_ip: u32,
        network_length: u8,
    ) -> Result<()> {
        let db = self.client_mgr.db().await;

        use crate::db::entities::devices;
        use chrono::Utc;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        // Find the device and update virtual IP fields
        let device = devices::Entity::find()
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .one(db.orm())
            .await?;

        if let Some(existing_device) = device {
            let mut active_model: devices::ActiveModel = existing_device.into();
            active_model.virtual_ip = Set(Some(virtual_ip));
            active_model.virtual_ip_network_length = Set(Some(network_length));
            active_model.updated_at = Set(Utc::now().into());

            active_model.update(db.orm()).await?;
            crate::info!(
                "Updated device {} virtual IP in database: {}/{}",
                device_id,
                virtual_ip,
                network_length
            );
        } else {
            return Err(anyhow::anyhow!("Device not found: {}", device_id));
        }

        Ok(())
    }
}
