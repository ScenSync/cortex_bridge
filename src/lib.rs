//! easytier-bridge
//!
//! This crate provides unified EasyTier integration for Cortex applications.
//! It combines both core EasyTier functionality and web client management capabilities.
//! Additionally, it provides C FFI interfaces for integration with cortex_server.

use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr;
use std::sync::Mutex;

// Shared functionality modules (always available)
mod logging;
mod stun_wrapper;

// Server functionality modules (conditional compilation)
#[cfg(feature = "server")]
pub mod client_manager;
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod config_srv;
#[cfg(feature = "server")]
pub mod db;
#[cfg(feature = "server")]
mod easytier_core_ffi;
#[cfg(feature = "server")]
pub mod network_config_srv_ffi;

// Client functionality modules (conditional compilation)
#[cfg(feature = "client")]
mod easytier_web_client;

// Re-export shared functionality
pub use logging::*;
pub use stun_wrapper::MockStunInfoCollectorWrapper;

// Re-export server functionality (conditional)
#[cfg(feature = "server")]
pub use easytier_core_ffi::{start_easytier_core, stop_easytier_core, EasyTierCoreConfig};

#[cfg(feature = "server")]
pub use client_manager::{
    session::{Location, Session},
    storage::{Storage, StorageToken},
    ClientManager,
};

#[cfg(feature = "server")]
pub use db::{entities, Database};

#[cfg(feature = "server")]
pub use network_config_srv_ffi::*;

// Re-export client functionality (conditional)
#[cfg(feature = "client")]
pub use easytier_web_client::{
    cortex_get_web_client_network_info, cortex_list_web_client_instances, cortex_start_web_client,
    cortex_stop_web_client,
};

// Global state management
#[cfg(feature = "server")]
use easytier::launcher::NetworkInstance;
#[cfg(feature = "server")]
use std::collections::HashMap;

// Core instances storage for FFI (server feature only)
#[cfg(feature = "server")]
static CLIENT_INSTANCES: once_cell::sync::Lazy<Mutex<HashMap<String, NetworkInstance>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static ERROR_MSG: once_cell::sync::Lazy<Mutex<Vec<u8>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

// C FFI structures
#[repr(C)]
#[derive(Debug)]
pub struct CortexWebClient {
    pub config_server_url: *const c_char,
    pub machine_id: *const c_char, // Persistent device UUID for stable identity
}

#[repr(C)]
#[derive(Debug)]
pub struct CortexNetworkInfo {
    pub instance_name: *const c_char,
    pub network_name: *const c_char,
    pub virtual_ipv4: *const c_char,
    pub hostname: *const c_char,
    pub version: *const c_char,
    pub peer_count: c_int,
    pub route_count: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct CortexPeerInfo {
    pub peer_id: *const c_char,
    pub virtual_ipv4: *const c_char,
    pub hostname: *const c_char,
    pub latency_ms: c_int,
    pub is_connected: c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct CortexRouteInfo {
    pub destination: *const c_char,
    pub next_hop: *const c_char,
    pub metric: c_int,
}

// Utility functions
fn set_error_msg(msg: &str) {
    if let Ok(mut error_msg) = ERROR_MSG.lock() {
        error_msg.clear();
        error_msg.extend_from_slice(msg.as_bytes());
        error_msg.push(0); // null terminator
    }
}

fn c_str_to_string(c_str: *const c_char) -> Result<String, &'static str> {
    if c_str.is_null() {
        return Err("Null pointer");
    }
    unsafe {
        CStr::from_ptr(c_str)
            .to_str()
            .map(|s| s.to_string())
            .map_err(|_| "Invalid UTF-8")
    }
}

// Core FFI functions
#[no_mangle]
pub extern "C" fn cortex_get_error_msg() -> *const c_char {
    if let Ok(error_msg) = ERROR_MSG.lock() {
        if !error_msg.is_empty() {
            return error_msg.as_ptr() as *const c_char;
        }
    }
    ptr::null()
}

#[no_mangle]
pub extern "C" fn cortex_core_free_string(s: *const c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s as *mut c_char);
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cortex_free_instance_list(instances: *const *const c_char, count: c_int) {
    if !instances.is_null() && count > 0 {
        unsafe {
            for i in 0..count {
                let instance_ptr = *instances.offset(i as isize);
                if !instance_ptr.is_null() {
                    let _ = CString::from_raw(instance_ptr as *mut c_char);
                }
            }
            let _ = Vec::from_raw_parts(
                instances as *mut *const c_char,
                count as usize,
                count as usize,
            );
        }
    }
}

// Logging FFI functions
#[no_mangle]
pub extern "C" fn cortex_core_set_and_init_console_logging(
    level: *const c_char,
    module_name: *const c_char,
) -> c_int {
    let level_str = match c_str_to_string(level) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let module_str = match c_str_to_string(module_name) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    set_and_init_console_logging(&level_str, &module_str);
    0
}

#[no_mangle]
pub extern "C" fn cortex_core_set_and_init_file_logging(
    level: *const c_char,
    module_name: *const c_char,
    log_path: *const c_char,
) -> c_int {
    let level_str = match c_str_to_string(level) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let module_str = match c_str_to_string(module_name) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let path_str = match c_str_to_string(log_path) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let _ = set_and_init_file_logging(&level_str, &module_str, &path_str);
    0
}

// Server-specific FFI functions (conditional compilation)
#[cfg(feature = "server")]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cortex_web_set_and_init_console_logging(
    level: *const c_char,
    module_name: *const c_char,
) -> i32 {
    if level.is_null() || module_name.is_null() {
        eprintln!("[RUST ERROR] cortex_web_set_and_init_console_logging: null parameter");
        return -1;
    }

    let level_str = unsafe {
        match CStr::from_ptr(level).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "[RUST ERROR] cortex_web_set_and_init_console_logging: invalid UTF-8 in level"
                );
                return -1;
            }
        }
    };

    let module_str = unsafe {
        match CStr::from_ptr(module_name).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[RUST ERROR] cortex_web_set_and_init_console_logging: invalid UTF-8 in module_name");
                return -1;
            }
        }
    };

    set_and_init_console_logging(level_str, module_str);
    0
}

#[cfg(feature = "server")]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cortex_web_set_and_init_file_logging(
    level: *const c_char,
    module_name: *const c_char,
    log_path: *const c_char,
) -> i32 {
    if level.is_null() || module_name.is_null() || log_path.is_null() {
        eprintln!("[RUST ERROR] cortex_web_set_and_init_file_logging: null parameter");
        return -1;
    }

    let level_str = unsafe {
        match CStr::from_ptr(level).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "[RUST ERROR] cortex_web_set_and_init_file_logging: invalid UTF-8 in level"
                );
                return -1;
            }
        }
    };

    let module_str = unsafe {
        match CStr::from_ptr(module_name).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[RUST ERROR] cortex_web_set_and_init_file_logging: invalid UTF-8 in module_name");
                return -1;
            }
        }
    };

    let path_str = unsafe {
        match CStr::from_ptr(log_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "[RUST ERROR] cortex_web_set_and_init_file_logging: invalid UTF-8 in log_path"
                );
                return -1;
            }
        }
    };

    match set_and_init_file_logging(level_str, module_str, path_str) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("[RUST ERROR] cortex_web_set_and_init_file_logging: {}", e);
            -1
        }
    }
}

// Panic recovery functions
#[no_mangle]
pub extern "C" fn cortex_core_get_last_panic() -> *const c_char {
    // Implementation placeholder
    ptr::null()
}

#[no_mangle]
pub extern "C" fn cortex_core_clear_last_panic() {
    // Implementation placeholder
}

#[no_mangle]
pub extern "C" fn cortex_core_init_panic_recovery() {
    // Implementation placeholder
}

#[cfg(feature = "server")]
#[no_mangle]
pub extern "C" fn cortex_web_get_last_panic() -> *mut c_char {
    // Implementation placeholder
    ptr::null_mut()
}

#[cfg(feature = "server")]
#[no_mangle]
pub extern "C" fn cortex_web_clear_last_panic() {
    // Implementation placeholder
}

#[cfg(feature = "server")]
#[no_mangle]
pub extern "C" fn cortex_web_init_panic_recovery() {
    // Implementation placeholder
}

#[cfg(feature = "server")]
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn cortex_easytier_web_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        #[allow(clippy::const_is_empty)]
        {
            assert!(!VERSION.is_empty());
        }
    }
}
