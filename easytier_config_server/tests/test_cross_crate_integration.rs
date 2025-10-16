//! Cross-crate integration tests
//!
//! This module tests integration between different crates in the workspace.

use std::ffi::CString;
use std::ptr;

#[cfg(test)]
mod cross_crate_tests {
    use super::*;

    #[test]
    fn test_common_utilities_available() {
        // Test that common utilities are available
        assert!(!easytier_common::VERSION.is_empty());

        unsafe {
            let level = CString::new("debug").unwrap();
            let module = CString::new("test").unwrap();

            let result = easytier_common::easytier_common_init_console_logging(
                level.as_ptr(),
                module.as_ptr(),
            );
            assert_eq!(result, 0, "Logging should initialize");
        }
    }

    #[test]
    fn test_device_client_uses_common_errors() {
        // Test that device client uses common error module
        use easytier_device_client::{cortex_start_web_client, CortexWebClient};

        unsafe {
            let result = cortex_start_web_client(ptr::null());
            assert_eq!(result, -1);

            let error_msg = easytier_common::easytier_common_get_error_msg();
            assert!(!error_msg.is_null());
        }
    }

    #[test]
    fn test_gateway_uses_common_errors() {
        // Test that gateway uses common error module
        use easytier_network_gateway::start_easytier_core;

        unsafe {
            let result = start_easytier_core(ptr::null());
            assert_eq!(result, -1);

            let error_msg = easytier_common::easytier_common_get_error_msg();
            assert!(!error_msg.is_null());
        }
    }

    #[test]
    fn test_all_crate_versions() {
        // Verify all crates have version information
        let versions = vec![
            ("easytier_common", easytier_common::VERSION),
            ("easytier_device_client", easytier_device_client::VERSION),
            (
                "easytier_network_gateway",
                easytier_network_gateway::VERSION,
            ),
            ("easytier_config_server", easytier_config_server::VERSION),
        ];

        for (crate_name, version) in versions {
            assert!(!version.is_empty(), "{} should have version", crate_name);
            println!("{}: {}", crate_name, version);
        }
    }

    #[test]
    fn test_device_client_and_gateway_independent() {
        // Test that device_client and gateway can be used independently
        use easytier_device_client::{cortex_start_web_client, CortexWebClient};
        use easytier_network_gateway::start_easytier_core;

        // Both should fail with null, but not crash
        unsafe {
            let client_result = cortex_start_web_client(ptr::null());
            assert_eq!(client_result, -1);

            let gateway_result = start_easytier_core(ptr::null());
            assert_eq!(gateway_result, -1);
        }
    }
}
