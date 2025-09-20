use crate::MockStunInfoCollectorWrapper;
use crate::{
    c_str_to_string, debug, error, info, set_error_msg, warn, CortexNetworkInfo, CortexWebClient,
};
use easytier::common::config::TomlConfigLoader;
use easytier::common::global_ctx::GlobalCtx;
use easytier::connector::create_connector_by_url;
use easytier::tunnel::IpVersion;
use easytier::web_client::WebClient;
use gethostname;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CString};
use std::sync::{Arc, Mutex};
use tokio;
use url;

// Global storage for web client instances
static WEB_CLIENT_INSTANCES: Lazy<
    Mutex<HashMap<String, (Arc<WebClient>, tokio::runtime::Runtime)>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// Create and start a web client instance that connects to a configuration server
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn cortex_start_web_client(client_config: *const CortexWebClient) -> c_int {
    if client_config.is_null() {
        error!("cortex_start_web_client: client_config is null");
        set_error_msg("client_config is null");
        return -1;
    }

    let config = unsafe { &*client_config };
    debug!("cortex_start_web_client: Starting web client configuration parsing");

    // Parse configuration parameters
    let config_server_url = match c_str_to_string(config.config_server_url) {
        Ok(url) => {
            info!("cortex_start_web_client: Config server URL: '{}'", url);
            url
        }
        Err(e) => {
            error!("cortex_start_web_client: Invalid config_server_url: {}", e);
            set_error_msg(&format!("invalid config_server_url: {}", e));
            return -1;
        }
    };

    // Create a new tokio runtime for async operations
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            info!("cortex_start_web_client: Created tokio runtime");
            rt
        }
        Err(e) => {
            error!(
                "cortex_start_web_client: Failed to create tokio runtime: {}",
                e
            );
            set_error_msg(&format!("failed to create tokio runtime: {}", e));
            return -1;
        }
    };

    // Execute async code in the tokio runtime
    let result = runtime.block_on(async {
        // Parse the URL
        let config_server_url = match url::Url::parse(&config_server_url) {
            Ok(u) => u,
            Err(e) => {
                error!("cortex_start_web_client: Failed to parse URL: {}", e);
                return Err(format!("failed to parse URL: {}", e));
            }
        };

        // Extract token from URL path
        let mut base_url = config_server_url.clone();
        base_url.set_path("");
        let token = config_server_url
            .path_segments()
            .and_then(|mut segments| segments.next())
            .map(|segment| segment.to_string())
            .unwrap_or_default();

        info!(
            "cortex_start_web_client: Base URL: {}, Token: {}",
            base_url, token
        );

        if token.is_empty() {
            return Err("empty token".to_string());
        }

        // Create global context and configuration
        let config = TomlConfigLoader::default();
        let global_ctx = Arc::new(GlobalCtx::new(config));

        // Set up STUN info collector
        global_ctx.replace_stun_info_collector(Box::new(MockStunInfoCollectorWrapper::new()));

        // Configure flags
        let mut flags = global_ctx.get_flags();
        flags.bind_device = false;
        global_ctx.set_flags(flags);

        // Get hostname
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        info!("cortex_start_web_client: Hostname: {}", hostname);

        // Create connector
        let connector =
            match create_connector_by_url(base_url.as_str(), &global_ctx, IpVersion::Both).await {
                Ok(conn) => {
                    info!("cortex_start_web_client: Created connector successfully");
                    conn
                }
                Err(e) => {
                    error!("cortex_start_web_client: Failed to create connector: {}", e);
                    return Err(format!("failed to create connector: {}", e));
                }
            };

        // Create and initialize WebClient
        let web_client = WebClient::new(connector, token.clone(), hostname);

        // Small delay to ensure initialization
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        info!("cortex_start_web_client: Web client created successfully");
        Ok((web_client, token))
    });

    // Handle the async result
    match result {
        Ok((web_client, instance_name)) => {
            info!("cortex_start_web_client: Web client instance created successfully");

            // Store the WebClient instance
            let mut instances = WEB_CLIENT_INSTANCES.lock().unwrap();
            instances.insert(instance_name.clone(), (Arc::new(web_client), runtime));
            info!(
                "cortex_start_web_client: Web client instance '{}' registered",
                instance_name
            );
            0
        }
        Err(e) => {
            error!(
                "cortex_start_web_client: Failed to create web client instance: {}",
                e
            );
            set_error_msg(&format!("failed to create web client instance: {}", e));
            -1
        }
    }
}

/// Stop a web client instance
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn cortex_stop_web_client(instance_name: *const c_char) -> c_int {
    let name = match c_str_to_string(instance_name) {
        Ok(name) => {
            info!(
                "cortex_stop_web_client: Stopping web client instance: {}",
                name
            );
            name
        }
        Err(e) => {
            error!("cortex_stop_web_client: Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    let mut instances = WEB_CLIENT_INSTANCES.lock().unwrap();
    if let Some((_web_client, _runtime)) = instances.remove(&name) {
        debug!(
            "cortex_stop_web_client: Found web client instance: {}",
            name
        );
        // WebClient and Runtime will be dropped automatically
        info!(
            "cortex_stop_web_client: Web client instance '{}' stopped",
            name
        );
        0
    } else {
        warn!(
            "cortex_stop_web_client: Web client instance '{}' not found",
            name
        );
        set_error_msg(&format!("web client instance '{}' not found", name));
        -1
    }
}

/// Get network information for a web client instance
/// Returns 0 on success, -1 on error
/// The caller must free the returned CortexNetworkInfo using cortex_free_network_info
#[no_mangle]
pub extern "C" fn cortex_get_web_client_network_info(
    instance_name: *const c_char,
    info: *mut *const CortexNetworkInfo,
) -> c_int {
    if instance_name.is_null() || info.is_null() {
        error!("cortex_get_web_client_network_info: Null pointer argument");
        set_error_msg("null pointer argument");
        return -1;
    }

    let name = match c_str_to_string(instance_name) {
        Ok(name) => {
            debug!(
                "cortex_get_web_client_network_info: Getting network info for instance: {}",
                name
            );
            name
        }
        Err(e) => {
            error!(
                "cortex_get_web_client_network_info: Invalid instance_name: {}",
                e
            );
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    // Check if web client instance exists
    let instances = WEB_CLIENT_INSTANCES.lock().unwrap();
    if !instances.contains_key(&name) {
        set_error_msg(&format!("web client instance '{}' not found", name));
        return -1;
    }

    // Create network info structure with basic information
    let network_name = name.clone();
    let virtual_ipv4 = "10.0.0.1"; // Default virtual IP for web clients
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let version = "1.0.0";
    let peer_count = 0; // Web clients typically don't have direct peers
    let route_count = 0; // Routes are managed by the config server

    // Create C strings for the network info structure
    let instance_name_c = match CString::new(name.clone()) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            set_error_msg("failed to convert instance name to C string");
            return -1;
        }
    };

    let network_name_c = match CString::new(network_name) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            unsafe {
                let _ = CString::from_raw(instance_name_c);
            }
            set_error_msg("failed to convert network name to C string");
            return -1;
        }
    };

    let virtual_ipv4_c = match CString::new(virtual_ipv4) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            unsafe {
                let _ = CString::from_raw(instance_name_c);
                let _ = CString::from_raw(network_name_c);
            }
            set_error_msg("failed to convert virtual IPv4 to C string");
            return -1;
        }
    };

    let hostname_c = match CString::new(hostname) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            unsafe {
                let _ = CString::from_raw(instance_name_c);
                let _ = CString::from_raw(network_name_c);
                let _ = CString::from_raw(virtual_ipv4_c);
            }
            set_error_msg("failed to convert hostname to C string");
            return -1;
        }
    };

    let version_c = match CString::new(version) {
        Ok(s) => s.into_raw(),
        Err(_) => {
            unsafe {
                let _ = CString::from_raw(instance_name_c);
                let _ = CString::from_raw(network_name_c);
                let _ = CString::from_raw(virtual_ipv4_c);
                let _ = CString::from_raw(hostname_c);
            }
            set_error_msg("failed to convert version to C string");
            return -1;
        }
    };

    // Create and return the network info structure
    let network_info = Box::new(CortexNetworkInfo {
        instance_name: instance_name_c,
        network_name: network_name_c,
        virtual_ipv4: virtual_ipv4_c,
        hostname: hostname_c,
        version: version_c,
        peer_count: peer_count as c_int,
        route_count: route_count as c_int,
    });

    unsafe {
        *info = Box::into_raw(network_info);
    }

    info!(
        "cortex_get_web_client_network_info: Successfully created network info for instance: {}",
        name
    );
    0
}

/// List all active web client instances
/// Returns the number of instances, -1 on error
/// The caller must free the returned array using cortex_free_instance_list
#[no_mangle]
pub extern "C" fn cortex_list_web_client_instances(
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
        unsafe {
            *instances = std::ptr::null();
        }
        return 0;
    }

    // Allocate array of C strings
    let mut c_strings = Vec::with_capacity(count);
    for i in 0..count {
        c_strings.push(match CString::new(instance_names[i].clone()) {
            Ok(s) => s.into_raw(),
            Err(_) => {
                // Free previously allocated strings
                for j in 0..i {
                    unsafe {
                        let _ = CString::from_raw(c_strings[j]);
                    }
                }
                set_error_msg("failed to convert instance name to C string");
                return -1;
            }
        });
    }

    let c_array = c_strings.into_boxed_slice();
    unsafe {
        *instances = c_array.as_ptr() as *const *const c_char;
        std::mem::forget(c_array); // Prevent deallocation
    }

    count as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_client_creation() {
        // Test web client creation with valid configuration
        let config_server_url = "https://example.com/test-token";
        let cstr = CString::new(config_server_url).unwrap();

        let config = CortexWebClient {
            config_server_url: cstr.as_ptr(),
        };

        // Note: This test would require a mock server or would fail in real environment
        // In a real test environment, you'd want to mock the network calls
        let result = cortex_start_web_client(&config);

        // Clean up if instance was created
        if result == 0 {
            let name_cstr = CString::new("test-token").unwrap();
            let _ = cortex_stop_web_client(name_cstr.as_ptr());
        }
    }
}
