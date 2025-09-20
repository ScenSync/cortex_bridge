//! 简化的 FFI 接口，使用单例模式在 Golang 中安全地使用 NetworkConfigService

use once_cell::sync::Lazy;
use serde_json;
use std::ffi::{c_char, CStr, CString};
use std::sync::Arc;
use urlencoding::encode;
use uuid::Uuid;

use crate::config_srv::NetworkConfigService;
use crate::db::OrgIdInDb;
use easytier::launcher::NetworkConfig;

// 全局 NetworkConfigService 单例
static NETWORK_CONFIG_SERVICE: Lazy<tokio::sync::Mutex<Option<Arc<tokio::sync::Mutex<NetworkConfigService>>>>> =
    Lazy::new(|| tokio::sync::Mutex::new(None));

// 全局 tokio runtime 管理器
struct RuntimeManager {
    runtime: tokio::runtime::Runtime,
}

impl RuntimeManager {
    fn new() -> Self {
        Self {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

static RUNTIME_MANAGER: Lazy<tokio::sync::Mutex<RuntimeManager>> =
    Lazy::new(|| tokio::sync::Mutex::new(RuntimeManager::new()));

/// 将 Go 的 DSN 字符串转换为 SeaORM 可用的连接字符串
pub fn convert_go_dsn_to_seaorm(dsn: &str) -> Result<String, String> {
    let parts: Vec<&str> = dsn.split('@').collect();
    if parts.len() != 2 {
        return Err("Invalid DSN: Must contain exactly one '@'".to_string());
    }

    let (user_pass, host_db_params) = (parts[0], parts[1]);

    // Handle username:password encoding
    let user_pass_encoded = if user_pass.contains(':') {
        let (user, pass) = user_pass.split_once(':').unwrap();
        format!("{user}:{}", encode(pass))
    } else {
        user_pass.to_string()
    };

    // Clean up host:port/db?params
    let cleaned = host_db_params
        .replace("tcp(", "")
        .replace(")", "")
        .replace(")/", "/");

    Ok(format!("mysql://{user_pass_encoded}@{cleaned}"))
}

//
// NetworkConfigService FFI 函数
//

/// 创建 NetworkConfigService 单例
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn create_network_config_service_singleton(
    db_url: *const c_char,
    geoip_path: *const c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    runtime_manager.block_on(async {
        // 检查是否已经初始化
        let already_initialized = NETWORK_CONFIG_SERVICE.lock().await.is_some();

        if already_initialized {
            return true; // 已经初始化，直接返回成功
        }

        // 解析数据库 URL
        let db_url = if !db_url.is_null() {
            match CStr::from_ptr(db_url).to_str() {
                Ok(s) => match convert_go_dsn_to_seaorm(s) {
                    Ok(converted) => converted,
                    Err(e) => {
                        if !err_msg.is_null() {
                            *err_msg = CString::new(format!("Failed to convert DSN: {}", e))
                                .unwrap_or_default()
                                .into_raw();
                        }
                        return false;
                    }
                },
                Err(e) => {
                    if !err_msg.is_null() {
                        *err_msg = CString::new(format!("Invalid db_url: {}", e))
                            .unwrap_or_default()
                            .into_raw();
                    }
                    return false;
                }
            }
        } else {
            if !err_msg.is_null() {
                *err_msg = CString::new("db_url is null")
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        };

        // 解析 GeoIP 路径
        let geoip_path = if !geoip_path.is_null() {
            match CStr::from_ptr(geoip_path).to_str() {
                Ok(s) => Some(s.to_string()),
                Err(e) => {
                    if !err_msg.is_null() {
                        *err_msg = CString::new(format!("Invalid geoip_path: {}", e))
                            .unwrap_or_default()
                            .into_raw();
                    }
                    return false;
                }
            }
        } else {
            None
        };

        // 创建 NetworkConfigService 实例
        let network_config_service = match NetworkConfigService::new(&db_url, geoip_path).await {
            Ok(service) => service,
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Failed to create NetworkConfigService: {:?}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                return false;
            }
        };

        // 存储到全局变量
        let mut service_opt = NETWORK_CONFIG_SERVICE.lock().await;
        *service_opt = Some(Arc::new(tokio::sync::Mutex::new(network_config_service)));
        true
    })
}

/// 启动 NetworkConfigService 的监听器
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_singleton_start(
    protocol: *const c_char,
    port: u16,
    err_msg: *mut *mut c_char,
) -> bool {
    // 解析协议
    let protocol = if !protocol.is_null() {
        match CStr::from_ptr(protocol).to_str() {
            Ok(s) => s.to_string(),
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Invalid protocol: {}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                return false;
            }
        }
    } else {
        if !err_msg.is_null() {
            *err_msg = CString::new("protocol is null")
                .unwrap_or_default()
                .into_raw();
        }
        return false;
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 启动监听器
    runtime_manager.block_on(async {
        // 获取全局 NetworkConfigService 实例
        let network_config_service = {
            let service_opt = NETWORK_CONFIG_SERVICE.lock().await;
            match &*service_opt {
                Some(service) => service.clone(),
                None => {
                    if !err_msg.is_null() {
                        *err_msg = CString::new("NetworkConfigService not initialized")
                            .unwrap_or_default()
                            .into_raw();
                    }
                    return false;
                }
            }
        };

        let result = {
            let mut service_guard = network_config_service.lock().await;
            service_guard.start(&protocol, port).await
        };

        match result {
            Ok(_) => true,
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Failed to start listener: {:?}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                false
            }
        }
    })
}

/// 销毁 NetworkConfigService 实例并释放资源
///
/// # Safety
///
/// 这个函数是不安全的
#[no_mangle]
pub unsafe extern "C" fn destroy_network_config_service_singleton(
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    runtime_manager.block_on(async {
        // 获取并移除全局 NetworkConfigService 实例
        let mut service_opt = NETWORK_CONFIG_SERVICE.lock().await;
        service_opt.take();
        true
    })
}

/// 释放由 C 字符串指针占用的内存
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn free_c_char(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

/// 收集单个网络实例信息
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_collect_one_network_info(
    org_id: *const c_char,
    device_id: *const c_char,
    inst_id: *const c_char,
    result_json_out: *mut *mut c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析实例ID
    let inst_id = match parse_uuid(inst_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用收集网络信息方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.collect_one_network_info(&org_id, &device_id, &inst_id).await
    }) {
        Ok(info) => {
            if !result_json_out.is_null() {
                match serde_json::to_string(&info) {
                    Ok(json) => {
                        *result_json_out = CString::new(json).unwrap_or_default().into_raw();
                        true
                    }
                    Err(e) => {
                        if !err_msg.is_null() {
                            *err_msg =
                                CString::new(format!("Failed to serialize network info: {}", e))
                                    .unwrap_or_default()
                                    .into_raw();
                        }
                        false
                    }
                }
            } else {
                true
            }
        }
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to collect network info: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 收集多个网络实例信息
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_collect_network_info(
    org_id: *const c_char,
    device_id: *const c_char,
    inst_ids_json: *const c_char,
    result_json_out: *mut *mut c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析实例ID列表
    let inst_ids = if !inst_ids_json.is_null() {
        match CStr::from_ptr(inst_ids_json).to_str() {
            Ok(s) => match serde_json::from_str::<Vec<String>>(s) {
                Ok(ids_str) => {
                    let mut ids = Vec::new();
                    for id_str in ids_str {
                        match Uuid::parse_str(&id_str) {
                            Ok(uuid) => ids.push(uuid),
                            Err(e) => {
                                if !err_msg.is_null() {
                                    *err_msg = CString::new(format!("Invalid UUID in list: {}", e))
                                        .unwrap_or_default()
                                        .into_raw();
                                }
                                return false;
                            }
                        }
                    }
                    Some(ids)
                }
                Err(e) => {
                    if !err_msg.is_null() {
                        *err_msg = CString::new(format!("Invalid inst_ids JSON: {}", e))
                            .unwrap_or_default()
                            .into_raw();
                    }
                    return false;
                }
            },
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Invalid inst_ids_json: {}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                return false;
            }
        }
    } else {
        None
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用收集网络信息方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.collect_network_info(&org_id, &device_id, inst_ids).await
    }) {
        Ok(info) => {
            if !result_json_out.is_null() {
                match serde_json::to_string(&info) {
                    Ok(json) => {
                        *result_json_out = CString::new(json).unwrap_or_default().into_raw();
                        true
                    }
                    Err(e) => {
                        if !err_msg.is_null() {
                            *err_msg =
                                CString::new(format!("Failed to serialize network info: {}", e))
                                    .unwrap_or_default()
                                    .into_raw();
                        }
                        false
                    }
                }
            } else {
                true
            }
        }
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to collect network info: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 列出网络实例 ID
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_list_network_instance_ids(
    org_id: *const c_char,
    device_id: *const c_char,
    result_json_out: *mut *mut c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用列出网络实例ID方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.list_network_instance_ids(&org_id, &device_id).await
    }) {
        Ok(ids) => {
            if !result_json_out.is_null() {
                match serde_json::to_string(&ids) {
                    Ok(json) => {
                        *result_json_out = CString::new(json).unwrap_or_default().into_raw();
                        true
                    }
                    Err(e) => {
                        if !err_msg.is_null() {
                            *err_msg = CString::new(format!(
                                "Failed to serialize network instance IDs: {}",
                                e
                            ))
                            .unwrap_or_default()
                            .into_raw();
                        }
                        false
                    }
                }
            } else {
                true
            }
        }
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to list network instance IDs: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 删除网络实例
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_remove_network_instance(
    org_id: *const c_char,
    device_id: *const c_char,
    inst_id: *const c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析实例ID
    let inst_id = match parse_uuid(inst_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用删除网络实例方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.remove_network_instance(&org_id, &device_id, &inst_id).await
    }) {
        Ok(_) => true,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to remove network instance: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 列出设备
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_list_devices(
    org_id: *const c_char,
    result_json_out: *mut *mut c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用列出设备方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.list_devices(&org_id).await
    }) {
        Ok(devices) => {
            if !result_json_out.is_null() {
                match serde_json::to_string(&devices) {
                    Ok(json) => {
                        *result_json_out = CString::new(json).unwrap_or_default().into_raw();
                        true
                    }
                    Err(e) => {
                        if !err_msg.is_null() {
                            *err_msg = CString::new(format!("Failed to serialize devices: {}", e))
                                .unwrap_or_default()
                                .into_raw();
                        }
                        false
                    }
                }
            } else {
                true
            }
        }
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to list devices: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 更新网络状态
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_update_network_state(
    org_id: *const c_char,
    device_id: *const c_char,
    inst_id: *const c_char,
    disabled: bool,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析实例ID
    let inst_id = match parse_uuid(inst_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用更新网络状态方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.update_network_state(&org_id, &device_id, &inst_id, disabled).await
    }) {
        Ok(_) => true,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to update network state: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 获取服务实例的辅助函数
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
unsafe fn get_service_instance(
    err_msg: *mut *mut c_char,
) -> Option<Arc<tokio::sync::Mutex<NetworkConfigService>>> {
    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return None;
        }
    };

    runtime_manager.block_on(async {
        let service_opt = NETWORK_CONFIG_SERVICE.lock().await;
        match &*service_opt {
            Some(service) => Some(service.clone()),
            None => {
                if !err_msg.is_null() {
                    *err_msg = CString::new("NetworkConfigService not initialized")
                        .unwrap_or_default()
                        .into_raw();
                }
                None
            }
        }
    })
}

/// 解析组织ID的辅助函数
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
unsafe fn parse_org_id(org_id: *const c_char, err_msg: *mut *mut c_char) -> Option<OrgIdInDb> {
    if !org_id.is_null() {
        match CStr::from_ptr(org_id).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Invalid org_id: {}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                None
            }
        }
    } else {
        if !err_msg.is_null() {
            *err_msg = CString::new("org_id is null")
                .unwrap_or_default()
                .into_raw();
        }
        None
    }
}

/// 解析UUID的辅助函数
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
unsafe fn parse_uuid(uuid_str: *const c_char, err_msg: *mut *mut c_char) -> Option<Uuid> {
    if !uuid_str.is_null() {
        match CStr::from_ptr(uuid_str).to_str() {
            Ok(s) => match Uuid::parse_str(s) {
                Ok(uuid) => Some(uuid),
                Err(e) => {
                    if !err_msg.is_null() {
                        *err_msg = CString::new(format!("Invalid UUID: {}", e))
                            .unwrap_or_default()
                            .into_raw();
                    }
                    None
                }
            },
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Invalid UUID string: {}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                None
            }
        }
    } else {
        if !err_msg.is_null() {
            *err_msg = CString::new("UUID is null")
                .unwrap_or_default()
                .into_raw();
        }
        None
    }
}

/// 解析网络配置的辅助函数
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
unsafe fn parse_network_config(
    config_json: *const c_char,
    err_msg: *mut *mut c_char,
) -> Option<NetworkConfig> {
    if !config_json.is_null() {
        match CStr::from_ptr(config_json).to_str() {
            Ok(s) => match serde_json::from_str::<NetworkConfig>(s) {
                Ok(config) => Some(config),
                Err(e) => {
                    if !err_msg.is_null() {
                        *err_msg = CString::new(format!("Invalid network config JSON: {}", e))
                            .unwrap_or_default()
                            .into_raw();
                    }
                    None
                }
            },
            Err(e) => {
                if !err_msg.is_null() {
                    *err_msg = CString::new(format!("Invalid config_json: {}", e))
                        .unwrap_or_default()
                        .into_raw();
                }
                None
            }
        }
    } else {
        if !err_msg.is_null() {
            *err_msg = CString::new("config_json is null")
                .unwrap_or_default()
                .into_raw();
        }
        None
    }
}

/// 验证网络配置
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_validate_config(
    org_id: *const c_char,
    device_id: *const c_char,
    config_json: *const c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析网络配置
    let config = match parse_network_config(config_json, err_msg) {
        Some(c) => c,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用验证配置方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.validate_config(&org_id, &device_id, config).await
    }) {
        Ok(_) => true,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Config validation failed: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}

/// 运行网络实例
///
/// # Safety
///
/// 这个函数是不安全的，因为它接受原始指针作为参数
#[no_mangle]
pub unsafe extern "C" fn network_config_service_run_network_instance(
    org_id: *const c_char,
    device_id: *const c_char,
    config_json: *const c_char,
    inst_id_out: *mut *mut c_char,
    err_msg: *mut *mut c_char,
) -> bool {
    // 获取服务实例
    let service = match get_service_instance(err_msg) {
        Some(s) => s,
        None => return false,
    };

    // 解析组织ID
    let org_id = match parse_org_id(org_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析设备ID
    let device_id = match parse_uuid(device_id, err_msg) {
        Some(id) => id,
        None => return false,
    };

    // 解析网络配置
    let config = match parse_network_config(config_json, err_msg) {
        Some(c) => c,
        None => return false,
    };

    // 获取 runtime 管理器
    let runtime_manager = match RUNTIME_MANAGER.try_lock() {
        Ok(manager) => manager,
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to lock runtime manager: {}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            return false;
        }
    };

    // 调用运行网络实例方法
    match runtime_manager.block_on(async {
        let service_guard = service.lock().await;
        service_guard.run_network_instance(&org_id, &device_id, config).await
    }) {
        Ok(inst_id) => {
            if !inst_id_out.is_null() {
                *inst_id_out = CString::new(inst_id.to_string())
                    .unwrap_or_default()
                    .into_raw();
            }
            true
        }
        Err(e) => {
            if !err_msg.is_null() {
                *err_msg = CString::new(format!("Failed to run network instance: {:?}", e))
                    .unwrap_or_default()
                    .into_raw();
            }
            false
        }
    }
}
