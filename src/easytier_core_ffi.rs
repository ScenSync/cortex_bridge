use crate::{c_str_to_string, set_error_msg, CLIENT_INSTANCES};
use crate::{debug, error, info, warn};
use easytier::common::config::TomlConfigLoader;
use easytier::launcher::{ConfigSource, NetworkInstance};
use std::ffi::{c_char, c_int};

// Simplified C-compatible structure for EasyTier Core based on comprehensive config
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
    pub rpc_port: c_int,

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

// FFI Functions for EasyTier Core

/// Create and start an EasyTier core instance
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn start_easytier_core(core_config: *const EasyTierCoreConfig) -> c_int {
    if core_config.is_null() {
        error!("start_easytier_core: core_config is null");
        set_error_msg("core_config is null");
        return -1;
    }

    let config = unsafe { &*core_config };
    debug!("start_easytier_core: Starting core configuration parsing");

    // Parse basic configuration
    let instance_name = match c_str_to_string(config.instance_name) {
        Ok(name) => {
            info!(
                "start_easytier_core: Instance name received from Go: '{}'",
                name
            );
            name
        }
        Err(e) => {
            error!("start_easytier_core: Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    let network_name = match c_str_to_string(config.network_name) {
        Ok(name) => {
            info!(
                "start_easytier_core: Network name received from Go: '{}'",
                name
            );
            name
        }
        Err(e) => {
            error!("start_easytier_core: Invalid network_name: {}", e);
            set_error_msg(&format!("invalid network_name: {}", e));
            return -1;
        }
    };

    let network_secret = match c_str_to_string(config.network_secret) {
        Ok(secret) => {
            info!(
                "start_easytier_core: Network secret received from Go: '{}' (length: {})",
                secret,
                secret.len()
            );
            secret
        }
        Err(e) => {
            error!("start_easytier_core: Invalid network_secret: {}", e);
            set_error_msg(&format!("invalid network_secret: {}", e));
            return -1;
        }
    };

    // Parse optional IP addresses
    let ipv4 = match c_str_to_string(config.ipv4) {
        Ok(ip) => ip,
        Err(_) => String::new(),
    };

    let ipv6 = match c_str_to_string(config.ipv6) {
        Ok(ip) => ip,
        Err(_) => String::new(),
    };

    let dev_name = match c_str_to_string(config.dev_name) {
        Ok(name) => name,
        Err(_) => String::new(),
    };

    let default_protocol = match c_str_to_string(config.default_protocol) {
        Ok(protocol) => protocol,
        Err(_) => "tcp".to_string(),
    };

    let foreign_network_whitelist = match c_str_to_string(config.foreign_network_whitelist) {
        Ok(whitelist) => whitelist,
        Err(_) => "*".to_string(),
    };

    let dhcp = config.dhcp != 0;

    // Parse arrays
    let mut peer_urls = Vec::new();
    let mut listener_urls = Vec::new();

    // Determine operation mode based on peers and private_mode flag
    // P2P mode: peers not empty AND private_mode = false
    // Private mode: peers empty OR private_mode = true
    let operation_mode = if config.private_mode != 0 {
        info!("start_easytier_core: Starting in Private mode (private_mode = true)");
        "private"
    } else if config.peer_urls_count > 0 && !config.peer_urls.is_null() {
        // Validate P2P mode requirements
        if network_name.is_empty() || network_secret.is_empty() {
            error!("start_easytier_core: P2P mode requires network_name and network_secret");
            set_error_msg("P2P mode requires network_name and network_secret");
            return -1;
        }
        info!(
            "start_easytier_core: Starting in P2P mode with network: {}",
            network_name
        );
        "p2p"
    } else {
        info!("start_easytier_core: Starting in Private mode (no peers configured)");
        "private"
    };

    info!("start_easytier_core: Operation mode: {}", operation_mode);
    info!(
        "start_easytier_core: RPC port from config: {}",
        config.rpc_port
    );
    info!(
        "start_easytier_core: Listener URLs count: {}",
        config.listener_urls_count
    );

    // Parse peer URLs (for P2P mode)
    if config.peer_urls_count > 0 && !config.peer_urls.is_null() {
        debug!(
            "start_easytier_core: Parsing {} peer URLs",
            config.peer_urls_count
        );
        let peer_url_ptrs = unsafe {
            std::slice::from_raw_parts(config.peer_urls, config.peer_urls_count as usize)
        };

        for &peer_url_ptr in peer_url_ptrs {
            match c_str_to_string(peer_url_ptr) {
                Ok(peer_url) => {
                    info!("start_easytier_core: Processing peer URL: {}", peer_url);
                    peer_urls.push(peer_url);
                }
                Err(e) => {
                    error!("start_easytier_core: Failed to parse peer URL: {}", e);
                    set_error_msg(&format!("invalid peer URL: {}", e));
                    return -1;
                }
            }
        }
    }

    // Parse listener URLs
    if config.listener_urls_count > 0 && !config.listener_urls.is_null() {
        let listener_ptrs = unsafe {
            std::slice::from_raw_parts(config.listener_urls, config.listener_urls_count as usize)
        };
        for &listener_ptr in listener_ptrs {
            if let Ok(listener) = c_str_to_string(listener_ptr) {
                listener_urls.push(listener);
            }
        }
    }

    // Mode-specific logging
    match operation_mode {
        "p2p" => {
            if peer_urls.is_empty() {
                warn!("start_easytier_core: P2P mode - no peers configured");
            } else {
                info!(
                    "start_easytier_core: P2P mode - {} peers configured",
                    peer_urls.len()
                );
            }
        }
        "private" => {
            info!("start_easytier_core: Private mode - network creator mode");
        }
        _ => {
            error!(
                "start_easytier_core: Unknown operation mode: {}",
                operation_mode
            );
            set_error_msg(&format!("unknown operation mode: {}", operation_mode));
            return -1;
        }
    }

    // Create comprehensive TOML configuration for client
    debug!("start_easytier_core: Creating comprehensive TOML configuration");

    // Build listeners array - validate and use URLs from Go side
    if listener_urls.is_empty() {
        error!("start_easytier_core: No listener URLs provided");
        set_error_msg("no listener URLs provided");
        return -1;
    }

    // Ensure we have enough listeners for basic functionality
    if listener_urls.len() < 2 {
        error!(
            "start_easytier_core: Insufficient listener URLs provided (need at least 2, got {})",
            listener_urls.len()
        );
        set_error_msg(&format!(
            "insufficient listener URLs provided (need at least 2, got {})",
            listener_urls.len()
        ));
        return -1;
    }

    debug!("start_easytier_core: Listener URLs: {:?}", listener_urls);
    // Use listener URLs directly from Go side (already properly formatted)
    let final_listeners = listener_urls.clone();

    // Use peer URLs directly
    let all_peer_urls = peer_urls.clone();

    // Generate mode-specific TOML configuration
    let toml_config = match operation_mode {
        "p2p" => {
            // P2P Mode TOML - based on comprehensive config
            format!(
                r#"instance_name = "{}"
dhcp = {}{}{}
listeners = [
    {},
]
mapped_listeners = []
exit_nodes = []
rpc_portal = "0.0.0.0:{}"

[network_identity]
network_name = "{}"
network_secret = "{}"

[flags]
default_protocol = "{}"
dev_name = "{}"
enable_encryption = {}
enable_ipv6 = {}
mtu = {}
latency_first = {}
enable_exit_node = {}
no_tun = {}
use_smoltcp = {}
foreign_network_whitelist = "{}"
disable_p2p = {}
relay_all_peer_rpc = {}
disable_udp_hole_punching = {}
private_mode = false

{}"#,
                instance_name,
                dhcp,
                if ipv4.is_empty() {
                    String::new()
                } else {
                    format!("\nipv4 = \"{}\"", ipv4)
                },
                if ipv6.is_empty() {
                    String::new()
                } else {
                    format!("\nipv6 = \"{}\"", ipv6)
                },
                final_listeners
                    .iter()
                    .map(|l| format!("    \"{}\"", l))
                    .collect::<Vec<_>>()
                    .join(",\n"),
                config.rpc_port,
                network_name,
                network_secret,
                default_protocol,
                dev_name,
                config.enable_encryption != 0,
                config.enable_ipv6 != 0,
                if config.mtu <= 0 { 1380 } else { config.mtu },
                config.latency_first != 0,
                config.enable_exit_node != 0,
                config.no_tun != 0,
                config.use_smoltcp != 0,
                foreign_network_whitelist,
                config.disable_p2p != 0,
                config.relay_all_peer_rpc != 0,
                config.disable_udp_hole_punching != 0,
                all_peer_urls
                    .iter()
                    .map(|url| format!("[[peer]]\nuri = \"{}\"", url))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
        "private" => {
            // Private Mode TOML - based on comprehensive config
            format!(
                r#"instance_name = "{}"
dhcp = {}{}{}
listeners = [
    {},
]
mapped_listeners = []
exit_nodes = []
rpc_portal = "0.0.0.0:{}"

[network_identity]
network_name = "{}"
network_secret = "{}"

[flags]
default_protocol = "{}"
dev_name = "{}"
enable_encryption = {}
enable_ipv6 = {}
mtu = {}
latency_first = {}
enable_exit_node = {}
no_tun = {}
use_smoltcp = {}
foreign_network_whitelist = "{}"
disable_p2p = {}
relay_all_peer_rpc = {}
disable_udp_hole_punching = {}
private_mode = true
"#,
                instance_name,
                dhcp,
                if ipv4.is_empty() {
                    String::new()
                } else {
                    format!("\nipv4 = \"{}\"", ipv4)
                },
                if ipv6.is_empty() {
                    String::new()
                } else {
                    format!("\nipv6 = \"{}\"", ipv6)
                },
                final_listeners
                    .iter()
                    .map(|l| format!("    \"{}\"", l))
                    .collect::<Vec<_>>()
                    .join(",\n"),
                config.rpc_port,
                network_name,
                network_secret,
                default_protocol,
                dev_name,
                config.enable_encryption != 0,
                config.enable_ipv6 != 0,
                if config.mtu <= 0 { 1380 } else { config.mtu },
                config.latency_first != 0,
                config.enable_exit_node != 0,
                config.no_tun != 0,
                config.use_smoltcp != 0,
                foreign_network_whitelist,
                config.disable_p2p != 0,
                config.relay_all_peer_rpc != 0,
                config.disable_udp_hole_punching != 0
            )
        }
        _ => {
            error!(
                "start_easytier_core: Unknown operation mode for TOML generation: {}",
                operation_mode
            );
            set_error_msg(&format!(
                "unknown operation mode for TOML generation: {}",
                operation_mode
            ));
            return -1;
        }
    };
    info!(
        "start_easytier_core: Generated TOML configuration:\n{}",
        toml_config
    );
    debug!(
        "start_easytier_core: TOML configuration created with {} peer URLs",
        all_peer_urls.len()
    );
    info!(
        "start_easytier_core: RPC port used in TOML: {}",
        config.rpc_port
    );
    info!(
        "start_easytier_core: RPC portal format: 0.0.0.0:{}",
        config.rpc_port
    );

    // Parse and start the instance
    debug!("start_easytier_core: Parsing TOML configuration");
    let cfg = match TomlConfigLoader::new_from_str(&toml_config) {
        Ok(cfg) => {
            info!("start_easytier_core: TOML configuration parsed successfully");
            cfg
        }
        Err(e) => {
            error!("start_easytier_core: Failed to parse core config: {}", e);
            set_error_msg(&format!("failed to parse core config: {}", e));
            return -1;
        }
    };

    info!("start_easytier_core: Creating and starting network instance");

    // Create the NetworkInstance
    let mut instance = NetworkInstance::new(cfg, ConfigSource::FFI);

    // Start the network instance using the launcher's start method
    match instance.start() {
        Ok(_event_subscriber) => {
            info!("start_easytier_core: Network instance started successfully");

            // Store the running instance
            if let Ok(mut instances) = CLIENT_INSTANCES.lock() {
                instances.insert(instance_name.clone(), instance);
                info!(
                    "start_easytier_core: Core instance '{}' registered and started successfully",
                    instance_name
                );
            } else {
                error!("start_easytier_core: Failed to acquire CLIENT_INSTANCES lock");
                set_error_msg("failed to acquire CLIENT_INSTANCES lock");
                return -1;
            }
        }
        Err(e) => {
            error!(
                "start_easytier_core: Failed to start network instance: {}",
                e
            );
            set_error_msg(&format!("failed to start network instance: {}", e));
            return -1;
        }
    }
    0
}

/// Stop an EasyTier core instance
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn stop_easytier_core(instance_name: *const c_char) -> c_int {
    let name = match c_str_to_string(instance_name) {
        Ok(name) => {
            info!("stop_easytier_core: Stopping core instance: {}", name);
            name
        }
        Err(e) => {
            error!("stop_easytier_core: Invalid instance_name: {}", e);
            set_error_msg(&format!("invalid instance_name: {}", e));
            return -1;
        }
    };

    if let Ok(mut instances) = CLIENT_INSTANCES.lock() {
        if let Some(_instance_id) = instances.remove(&name) {
            debug!("stop_easytier_core: Found instance ID");
            // TODO: Implement proper instance cleanup when easytier API is available
            info!(
                "stop_easytier_core: Core instance '{}' stopped successfully",
                name
            );
            0
        } else {
            warn!("stop_easytier_core: Core instance '{}' not found", name);
            set_error_msg(&format!("core instance '{}' not found", name));
            -1
        }
    } else {
        error!("stop_easytier_core: Failed to acquire CLIENT_INSTANCES lock");
        set_error_msg("failed to acquire CLIENT_INSTANCES lock");
        -1
    }
}
