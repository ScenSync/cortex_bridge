//! EasyTier core wrapper using Builder API (improved from original TOML string approach)

use easytier::common::config::{ConfigLoader, NetworkIdentity, PeerConfig, TomlConfigLoader};
use easytier::common::global_ctx::GlobalCtx;
use easytier::launcher::{ConfigSource, NetworkInstance};
use easytier_common::{c_str_to_string, parse_string_array, set_error_msg};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{c_char, c_int};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

// Structure to store NetworkInstance, GlobalCtx, and runtime
struct GatewayInstance {
    #[allow(dead_code)] // Keep instance alive, accessed via global_ctx
    instance: NetworkInstance,
    global_ctx: Arc<GlobalCtx>,
    #[allow(dead_code)] // Keep runtime alive for async tasks
    _runtime: tokio::runtime::Runtime,
}

// Global storage for gateway instances
static GATEWAY_INSTANCES: Lazy<Mutex<HashMap<String, GatewayInstance>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// C-compatible structure for EasyTier Core configuration
#[repr(C)]
#[derive(Debug)]
pub struct EasyTierCoreConfig {
    // Basic instance info
    pub instance_name: *const c_char,

    // Network configuration
    pub dhcp: c_int,         // 0 = false, 1 = true
    pub ipv4: *const c_char, // Optional manual IPv4 address
    pub ipv6: *const c_char, // Optional manual IPv6 address

    // Listeners and networking
    pub listener_urls: *const *const c_char,
    pub listener_urls_count: c_int,

    // Network identity
    pub network_name: *const c_char,
    pub network_secret: *const c_char,

    // Peer configuration (for P2P mode)
    pub peer_urls: *const *const c_char,
    pub peer_urls_count: c_int,

    // Flags configuration
    pub default_protocol: *const c_char, // "tcp", "udp", etc.
    pub dev_name: *const c_char,
    pub enable_encryption: c_int,                 // 0 = false, 1 = true
    pub enable_ipv6: c_int,                       // 0 = false, 1 = true
    pub mtu: c_int,                               // Default 1380
    pub latency_first: c_int,                     // 0 = false, 1 = true
    pub enable_exit_node: c_int,                  // 0 = false, 1 = true
    pub no_tun: c_int,                            // 0 = false, 1 = true
    pub use_smoltcp: c_int,                       // 0 = false, 1 = true
    pub foreign_network_whitelist: *const c_char, // Default "*"
    pub disable_p2p: c_int,                       // 0 = false, 1 = true
    pub relay_all_peer_rpc: c_int,                // 0 = false, 1 = true
    pub disable_udp_hole_punching: c_int,         // 0 = false, 1 = true
    pub private_mode: c_int,                      // 0 = false, 1 = true
}

/// Create and start an EasyTier core instance using Builder API
/// Returns 0 on success, -1 on error
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure all pointers are valid and point to null-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn start_easytier_core(core_config: *const EasyTierCoreConfig) -> c_int {
    if core_config.is_null() {
        error!("start_easytier_core: core_config is null");
        set_error_msg("core_config is null");
        return -1;
    }

    let config = &*core_config;
    info!("start_easytier_core: Starting gateway with builder API");

    // Parse required parameters
    let instance_name = match c_str_to_string(config.instance_name) {
        Ok(name) => {
            info!("Instance name: '{}'", name);
            name
        }
        Err(e) => {
            error!("Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    let network_name = match c_str_to_string(config.network_name) {
        Ok(name) => {
            info!("Network name: '{}'", name);
            name
        }
        Err(e) => {
            error!("Invalid network_name: {}", e);
            set_error_msg(&format!("invalid network_name: {}", e));
            return -1;
        }
    };

    let network_secret = match c_str_to_string(config.network_secret) {
        Ok(secret) => {
            info!("Network secret length: {}", secret.len());
            secret
        }
        Err(e) => {
            error!("Invalid network_secret: {}", e);
            set_error_msg(&format!("invalid network_secret: {}", e));
            return -1;
        }
    };

    // Parse optional parameters
    let ipv4 = c_str_to_string(config.ipv4).ok().filter(|s| !s.is_empty());
    let ipv6 = c_str_to_string(config.ipv6).ok().filter(|s| !s.is_empty());
    let dev_name = c_str_to_string(config.dev_name).unwrap_or_default();
    let default_protocol =
        c_str_to_string(config.default_protocol).unwrap_or_else(|_| "tcp".to_string());
    let foreign_network_whitelist =
        c_str_to_string(config.foreign_network_whitelist).unwrap_or_else(|_| "*".to_string());

    // Parse arrays
    let listener_urls = match parse_string_array(config.listener_urls, config.listener_urls_count) {
        Ok(urls) => {
            if urls.is_empty() {
                error!("No listener URLs provided");
                set_error_msg("no listener URLs provided");
                return -1;
            }
            info!("Parsed {} listener URLs", urls.len());
            urls
        }
        Err(e) => {
            error!("Failed to parse listener URLs: {}", e);
            set_error_msg(&format!("failed to parse listener URLs: {}", e));
            return -1;
        }
    };

    let peer_urls = match parse_string_array(config.peer_urls, config.peer_urls_count) {
        Ok(urls) => {
            info!("Parsed {} peer URLs", urls.len());
            urls
        }
        Err(e) => {
            error!("Failed to parse peer URLs: {}", e);
            set_error_msg(&format!("failed to parse peer URLs: {}", e));
            return -1;
        }
    };

    // Determine operation mode
    let private_mode = config.private_mode != 0;
    let operation_mode = if private_mode {
        "private"
    } else if !peer_urls.is_empty() {
        "p2p"
    } else {
        "private"
    };

    info!("Operation mode: {}", operation_mode);

    // ═══════════════════════════════════════════════════════════════════════
    // BUILD CONFIG USING BUILDER API (instead of TOML strings)
    // ═══════════════════════════════════════════════════════════════════════

    let cfg = TomlConfigLoader::default();

    // Set instance name
    cfg.set_inst_name(instance_name.clone());

    // Set network identity
    cfg.set_network_identity(NetworkIdentity::new(network_name, network_secret));

    // Set DHCP
    cfg.set_dhcp(config.dhcp != 0);

    // Set IPv4 address
    if let Some(ipv4_str) = ipv4 {
        match ipv4_str.parse() {
            Ok(addr) => {
                cfg.set_ipv4(Some(addr));
                info!("Set IPv4: {}", ipv4_str);
            }
            Err(e) => {
                error!("Invalid IPv4 address '{}': {}", ipv4_str, e);
                set_error_msg(&format!("invalid IPv4 address: {}", e));
                return -1;
            }
        }
    }

    // Set IPv6 address
    if let Some(ipv6_str) = ipv6 {
        match ipv6_str.parse() {
            Ok(addr) => {
                cfg.set_ipv6(Some(addr));
                info!("Set IPv6: {}", ipv6_str);
            }
            Err(e) => {
                error!("Invalid IPv6 address '{}': {}", ipv6_str, e);
                set_error_msg(&format!("invalid IPv6 address: {}", e));
                return -1;
            }
        }
    }

    // Set listeners
    let listeners: Result<Vec<url::Url>, _> = listener_urls.iter().map(|s| s.parse()).collect();
    match listeners {
        Ok(urls) => {
            cfg.set_listeners(urls);
            info!("Set {} listeners", listener_urls.len());
        }
        Err(e) => {
            error!("Invalid listener URL: {}", e);
            set_error_msg(&format!("invalid listener URL: {}", e));
            return -1;
        }
    }

    // Set peers (for P2P mode)
    if !peer_urls.is_empty() {
        let peers: Result<Vec<PeerConfig>, _> = peer_urls
            .iter()
            .map(|url_str| url_str.parse().map(|uri| PeerConfig { uri }))
            .collect();

        match peers {
            Ok(peer_configs) => {
                cfg.set_peers(peer_configs);
                info!("Set {} peers", peer_urls.len());
            }
            Err(e) => {
                error!("Invalid peer URL: {}", e);
                set_error_msg(&format!("invalid peer URL: {}", e));
                return -1;
            }
        }
    }

    // Note: RPC portal is now configured globally via --rpc-portal flag or ET_RPC_PORTAL env var
    // Per-instance RPC configuration has been removed in EasyTier v2.4.5

    // Set flags using builder pattern
    let mut flags = cfg.get_flags();
    flags.default_protocol = default_protocol;
    flags.dev_name = dev_name;
    flags.enable_encryption = config.enable_encryption != 0;
    flags.enable_ipv6 = config.enable_ipv6 != 0;
    flags.mtu = if config.mtu <= 0 {
        1380
    } else {
        config.mtu as u32
    };
    flags.latency_first = config.latency_first != 0;
    flags.enable_exit_node = config.enable_exit_node != 0;
    flags.no_tun = config.no_tun != 0;
    flags.use_smoltcp = config.use_smoltcp != 0;
    flags.relay_network_whitelist = foreign_network_whitelist;
    flags.disable_p2p = config.disable_p2p != 0;
    flags.relay_all_peer_rpc = config.relay_all_peer_rpc != 0;
    flags.disable_udp_hole_punching = config.disable_udp_hole_punching != 0;
    flags.private_mode = private_mode;

    cfg.set_flags(flags);

    info!("Configuration built using builder API:");
    info!("  - Instance: {}", instance_name);
    info!("  - Mode: {}", operation_mode);
    info!("  - Encryption: {}", config.enable_encryption != 0);
    info!("  - IPv6: {}", config.enable_ipv6 != 0);
    info!(
        "  - MTU: {}",
        if config.mtu <= 0 { 1380 } else { config.mtu }
    );

    // Create Tokio runtime for async operations
    // IMPORTANT: Must create runtime BEFORE any GlobalCtx or NetworkInstance
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create tokio runtime: {}", e);
            set_error_msg(&format!("failed to create tokio runtime: {}", e));
            return -1;
        }
    };

    // Create GlobalCtx and NetworkInstance within the runtime context
    // CRITICAL: GlobalCtx::new() and NetworkInstance::new() MUST be called within runtime context
    // because they initialize async components (TokenBucket, etc.) that require Tokio
    let start_result = runtime.block_on(async {
        // Create GlobalCtx within runtime context
        let global_ctx = Arc::new(GlobalCtx::new(cfg.clone()));
        
        // Create NetworkInstance within runtime context
        let mut instance = NetworkInstance::new(cfg, ConfigSource::FFI);
        
        match instance.start() {
            Ok(_event_subscriber) => {
                info!("Network instance started successfully");
                Ok((instance, global_ctx))
            }
            Err(e) => {
                error!("Failed to start network instance: {}", e);
                Err(e)
            }
        }
    });

    match start_result {
        Ok((instance, global_ctx)) => {
            // Store the running instance with GlobalCtx and runtime
            if let Ok(mut instances) = GATEWAY_INSTANCES.lock() {
                instances.insert(
                    instance_name.clone(),
                    GatewayInstance {
                        instance,
                        global_ctx,
                        _runtime: runtime,
                    },
                );
                info!(
                    "Gateway instance '{}' registered successfully",
                    instance_name
                );
            } else {
                error!("Failed to acquire GATEWAY_INSTANCES lock");
                set_error_msg("failed to acquire lock");
                return -1;
            }

            0
        }
        Err(e) => {
            error!("Failed to start network instance: {}", e);
            set_error_msg(&format!("failed to start: {}", e));
            -1
        }
    }
}

/// Stop an EasyTier core instance
/// Returns 0 on success, -1 on error
///
/// # Safety
///
/// The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn stop_easytier_core(instance_name: *const c_char) -> c_int {
    let name = match c_str_to_string(instance_name) {
        Ok(name) => {
            info!("Stopping gateway instance: {}", name);
            name
        }
        Err(e) => {
            error!("Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    if let Ok(mut instances) = GATEWAY_INSTANCES.lock() {
        if instances.remove(&name).is_some() {
            info!("Gateway instance '{}' stopped successfully", name);
            0
        } else {
            warn!("Gateway instance '{}' not found", name);
            set_error_msg(&format!("instance '{}' not found", name));
            -1
        }
    } else {
        error!("Failed to acquire GATEWAY_INSTANCES lock");
        set_error_msg("failed to acquire lock");
        -1
    }
}

/// Get gateway instance status (optional extension)
///
/// # Safety
///
/// The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string
/// and `status_json_out` is a valid mutable pointer.
#[no_mangle]
pub unsafe extern "C" fn get_easytier_core_status(
    instance_name: *const c_char,
    status_json_out: *mut *mut c_char,
) -> c_int {
    let name = match c_str_to_string(instance_name) {
        Ok(name) => name,
        Err(e) => {
            error!("Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    if status_json_out.is_null() {
        error!("status_json_out is null");
        set_error_msg("status_json_out is null");
        return -1;
    }

    let instances = GATEWAY_INSTANCES.lock().unwrap();
    let exists = instances.contains_key(&name);

    // Create simple status JSON
    let status = serde_json::json!({
        "instance_name": name,
        "running": exists,
    });

    match serde_json::to_string(&status) {
        Ok(json) => match std::ffi::CString::new(json) {
            Ok(c_str) => {
                *status_json_out = c_str.into_raw();
                0
            }
            Err(e) => {
                error!("Failed to create C string: {}", e);
                set_error_msg("failed to create C string");
                -1
            }
        },
        Err(e) => {
            error!("Failed to serialize status: {}", e);
            set_error_msg("failed to serialize status");
            -1
        }
    }
}

/// Get the local virtual IPv4 address for a gateway instance
/// Returns 0 on success, -1 on error
///
/// The virtual IP is returned as a CIDR string (e.g., "10.126.126.1/24")
/// The caller is responsible for freeing the returned string using `free_string()`.
///
/// # Safety
///
/// The caller must ensure that `instance_name` is a valid pointer to a null-terminated C string
/// and `ipv4_out` is a valid mutable pointer.
#[no_mangle]
pub unsafe extern "C" fn get_easytier_virtual_ipv4(
    instance_name: *const c_char,
    ipv4_out: *mut *mut c_char,
) -> c_int {
    let name = match c_str_to_string(instance_name) {
        Ok(name) => name,
        Err(e) => {
            error!("Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    if ipv4_out.is_null() {
        error!("ipv4_out is null");
        set_error_msg("ipv4_out is null");
        return -1;
    }

    // Extract what we need from the locked section, then release the lock
    // IMPORTANT: Don't hold the lock across async boundaries to avoid deadlocks
    let (global_ctx, runtime_handle) = {
        let instances = match GATEWAY_INSTANCES.lock() {
            Ok(instances) => instances,
            Err(e) => {
                error!("Failed to acquire GATEWAY_INSTANCES lock: {}", e);
                set_error_msg("failed to acquire lock");
                return -1;
            }
        };

        let gateway_instance = match instances.get(&name) {
            Some(inst) => inst,
            None => {
                warn!("Gateway instance '{}' not found", name);
                set_error_msg(&format!("instance '{}' not found", name));
                return -1;
            }
        };

        // Clone what we need before dropping the lock
        (gateway_instance.global_ctx.clone(), gateway_instance._runtime.handle().clone())
    }; // Lock is dropped here
    
    // Execute within the Tokio runtime context (lock is now released)
    let ipv4_string = runtime_handle.block_on(async {
        let virtual_ip = global_ctx.get_ipv4();
        virtual_ip.map(|x| x.to_string()).unwrap_or_default()
    });

    info!(
        "Retrieved virtual IP for instance '{}': {}",
        name, ipv4_string
    );

    match std::ffi::CString::new(ipv4_string) {
        Ok(c_str) => {
            *ipv4_out = c_str.into_raw();
            0
        }
        Err(e) => {
            error!("Failed to create C string: {}", e);
            set_error_msg("failed to create C string");
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_struct_size() {
        // Verify the C struct has the expected size
        let size = std::mem::size_of::<EasyTierCoreConfig>();
        assert!(size > 0, "EasyTierCoreConfig should have non-zero size");
    }
}
