//! Web client implementation for device-side connection to config server
//! This will contain the cortex_start_web_client implementation

use easytier::common::config::TomlConfigLoader;
use easytier::common::global_ctx::GlobalCtx;
use easytier::common::set_default_machine_id;
use easytier::connector::create_connector_by_url;
use easytier::tunnel::IpVersion;
use easytier::web_client::WebClient;
use easytier_common::{c_str_to_string, set_error_msg};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CString};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

use crate::MockStunInfoCollectorWrapper;

// Type alias - store GlobalCtx and current virtual IP
type WebClientInstance = (
    Arc<WebClient>,
    Arc<GlobalCtx>,
    tokio::runtime::Runtime,
    Arc<std::sync::Mutex<Option<String>>>, // Cached virtual IP
);
type WebClientMap = HashMap<String, WebClientInstance>;

// Global storage for web client instances
static WEB_CLIENT_INSTANCES: Lazy<Mutex<WebClientMap>> = Lazy::new(|| Mutex::new(HashMap::new()));

// C FFI structures
#[repr(C)]
#[derive(Debug)]
pub struct CortexWebClient {
    pub config_server_url: *const c_char,
    pub machine_id: *const c_char,
}

#[repr(C)]
#[derive(Debug)]
pub struct CortexNetworkInfo {
    pub instance_name: *const c_char,
    pub network_name: *const c_char,
    pub virtual_ipv4: *const c_char,
    pub hostname: *const c_char,
    pub version: *const c_char,
}

/// Start web client in config mode
///
/// # Safety
///
/// The caller must ensure that `client_config` is a valid pointer to a properly initialized `CortexWebClient` struct.
#[no_mangle]
pub unsafe extern "C" fn cortex_start_web_client(client_config: *const CortexWebClient) -> c_int {
    if client_config.is_null() {
        error!("cortex_start_web_client: client_config is null");
        set_error_msg("client_config is null");
        return -1;
    }

    let config = &*client_config;

    // Parse config server URL
    let config_server_url = match c_str_to_string(config.config_server_url) {
        Ok(url) => url,
        Err(e) => {
            error!("Invalid config_server_url: {}", e);
            set_error_msg(&format!("invalid config_server_url: {}", e));
            return -1;
        }
    };

    // Extract organization ID from config_server_url path
    let organization_id = match url::Url::parse(&config_server_url) {
        Ok(url) => {
            let path = url.path().trim_start_matches('/');
            if path.is_empty() {
                error!("No organization ID in config_server_url path");
                set_error_msg("no organization ID in config_server_url path");
                return -1;
            }
            path.to_string()
        }
        Err(e) => {
            error!("Invalid config_server_url format: {}", e);
            set_error_msg(&format!("invalid config_server_url format: {}", e));
            return -1;
        }
    };

    // Parse machine_id
    let machine_id = if !config.machine_id.is_null() {
        match c_str_to_string(config.machine_id) {
            Ok(id_str) => match uuid::Uuid::parse_str(&id_str) {
                Ok(id) => {
                    info!("Using persistent machine_id: {}", id);
                    Some(id)
                }
                Err(e) => {
                    warn!(
                        "Invalid machine_id '{}': {}, using system default",
                        id_str, e
                    );
                    None
                }
            },
            Err(_) => None,
        }
    } else {
        None
    };

    // Create tokio runtime
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create tokio runtime: {}", e);
            set_error_msg(&format!("failed to create tokio runtime: {}", e));
            return -1;
        }
    };

    // Execute async code
    let result = runtime.block_on(async {
        let config_server_url = match url::Url::parse(&config_server_url) {
            Ok(u) => u,
            Err(e) => return Err(format!("failed to parse URL: {}", e)),
        };

        // Extract base URL and token (organization_id) from URL path
        let mut base_url = config_server_url.clone();
        base_url.set_path("");
        let token = organization_id.clone();

        info!(
            "Connecting to config server: {}, token: {}",
            base_url, token
        );

        if token.is_empty() {
            return Err("empty organization_id".to_string());
        }

        // Set machine_id if provided
        if let Some(mid) = machine_id {
            set_default_machine_id(Some(mid.to_string()));
            info!("Set default machine_id: {}", mid);
        }

        // Create global context
        let config = TomlConfigLoader::default();
        let global_ctx = Arc::new(GlobalCtx::new(config));
        global_ctx.replace_stun_info_collector(Box::new(MockStunInfoCollectorWrapper::new()));

        let mut flags = global_ctx.get_flags();
        flags.bind_device = false;
        global_ctx.set_flags(flags);

        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        info!("Device hostname: {}", hostname);

        // Create connector
        let connector = create_connector_by_url(base_url.as_str(), &global_ctx, IpVersion::Both)
            .await
            .map_err(|e| format!("failed to create connector: {}", e))?;

        // Create WebClient
        let web_client = WebClient::new(connector, token.clone(), hostname);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        info!("Web client created successfully");

        // Create a cache for the virtual IP
        let virtual_ip_cache = Arc::new(std::sync::Mutex::new(None));

        // Spawn a task to listen for DHCP IP changes
        let global_ctx_clone = global_ctx.clone();
        let ip_cache_clone = virtual_ip_cache.clone();
        tokio::spawn(async move {
            let mut subscriber = global_ctx_clone.subscribe();
            while let Ok(event) = subscriber.recv().await {
                if let easytier::common::global_ctx::GlobalCtxEvent::DhcpIpv4Changed(_, Some(ip)) =
                    event
                {
                    let ip_str = format!("{}", ip);
                    info!("DHCP IPv4 assigned: {}", ip_str);
                    if let Ok(mut cache) = ip_cache_clone.lock() {
                        *cache = Some(ip_str);
                    }
                }
            }
        });

        Ok((web_client, global_ctx, virtual_ip_cache, token))
    });

    match result {
        Ok((web_client, global_ctx, virtual_ip_cache, instance_name)) => {
            let mut instances = WEB_CLIENT_INSTANCES.lock().unwrap();
            instances.insert(
                instance_name.clone(),
                (Arc::new(web_client), global_ctx, runtime, virtual_ip_cache),
            );
            info!("Web client instance '{}' registered", instance_name);
            0
        }
        Err(e) => {
            error!("Failed to create web client: {}", e);
            set_error_msg(&format!("failed to create web client: {}", e));
            -1
        }
    }
}

/// Stop web client
///
/// # Safety
///
/// The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn cortex_stop_web_client(instance_name: *const c_char) -> c_int {
    let name = match c_str_to_string(instance_name) {
        Ok(name) => name,
        Err(e) => {
            error!("Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    let mut instances = WEB_CLIENT_INSTANCES.lock().unwrap();
    if instances.remove(&name).is_some() {
        info!("Web client instance '{}' stopped", name);
        0
    } else {
        warn!("Web client instance '{}' not found", name);
        set_error_msg(&format!("instance '{}' not found", name));
        -1
    }
}

/// Get network info
///
/// # Safety
///
/// The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string
/// and `info` is a valid mutable pointer.
#[no_mangle]
pub unsafe extern "C" fn cortex_get_web_client_network_info(
    instance_name: *const c_char,
    info: *mut *const CortexNetworkInfo,
) -> c_int {
    if instance_name.is_null() || info.is_null() {
        error!("Null pointer argument");
        set_error_msg("null pointer argument");
        return -1;
    }

    let name = match c_str_to_string(instance_name) {
        Ok(name) => name,
        Err(e) => {
            error!("Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    let instances = WEB_CLIENT_INSTANCES.lock().unwrap();
    let instance = match instances.get(&name) {
        Some(inst) => inst,
        None => {
            set_error_msg(&format!("instance '{}' not found", name));
            return -1;
        }
    };

    let (_web_client, global_ctx, _runtime, ip_cache) = instance;

    // Get actual network information from cached DHCP-assigned IP or GlobalCtx
    let virtual_ipv4 = if let Ok(cache) = ip_cache.lock() {
        if let Some(ref cached_ip) = *cache {
            cached_ip.clone()
        } else {
            // Fall back to GlobalCtx if cache is empty
            match global_ctx.get_ipv4() {
                Some(ipv4) => format!("{}", ipv4),
                None => {
                    warn!("Virtual IPv4 not assigned yet");
                    "0.0.0.0/0".to_string()
                }
            }
        }
    } else {
        warn!("Failed to lock IP cache");
        "0.0.0.0/0".to_string()
    };

    // Create network info with actual values
    let network_info = Box::new(CortexNetworkInfo {
        instance_name: CString::new(name.clone()).unwrap().into_raw(),
        network_name: CString::new(name).unwrap().into_raw(),
        virtual_ipv4: CString::new(virtual_ipv4).unwrap().into_raw(),
        hostname: CString::new(gethostname::gethostname().to_string_lossy().to_string())
            .unwrap()
            .into_raw(),
        version: CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw(),
    });

    *info = Box::into_raw(network_info);
    0
}

/// List web client instances
///
/// # Safety
///
/// The caller must ensure that `instances` is a valid mutable pointer.
#[no_mangle]
pub unsafe extern "C" fn cortex_list_web_client_instances(
    instances: *mut *const *const c_char,
    max_count: c_int,
) -> c_int {
    if instances.is_null() || max_count <= 0 {
        set_error_msg("invalid arguments");
        return -1;
    }

    let web_instances = WEB_CLIENT_INSTANCES.lock().unwrap();
    let instance_names: Vec<String> = web_instances.keys().cloned().collect();

    let count = std::cmp::min(instance_names.len(), max_count as usize);
    if count == 0 {
        *instances = std::ptr::null();
        return 0;
    }

    let mut c_strings = Vec::with_capacity(count);
    for instance_name in instance_names.iter().take(count) {
        c_strings.push(match CString::new(instance_name.clone()) {
            Ok(s) => s.into_raw(),
            Err(_) => {
                set_error_msg("failed to convert instance name");
                return -1;
            }
        });
    }

    let c_array = c_strings.into_boxed_slice();
    *instances = c_array.as_ptr() as *const *const c_char;
    std::mem::forget(c_array);

    count as c_int
}
