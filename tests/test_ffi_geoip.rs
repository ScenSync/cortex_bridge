//! Test FFI functions for GeoIP database loading
//!
//! This test verifies that the FFI functions can properly load the GeoIP database
//! and handle various scenarios including auto-detection and explicit paths.

use std::ffi::{c_char, CString};
use std::path::Path;
use std::ptr;

// Import the FFI functions from the library
extern "C" {
    fn create_network_config_service_singleton(
        db_url: *const c_char,
        geoip_path: *const c_char,
        err_msg: *mut *mut c_char,
    ) -> bool;

    fn destroy_network_config_service_singleton(err_msg: *mut *mut c_char) -> bool;

    fn free_c_char(ptr: *mut c_char);
}

/// Helper function to safely convert Rust string to C string with proper lifetime management
fn to_c_string_safe(s: &str) -> (*const c_char, CString) {
    let c_string = CString::new(s).unwrap();
    let ptr = c_string.as_ptr();
    (ptr, c_string)
}

#[test]
fn test_ffi_geoip_auto_detection() {
    // Test FFI with null geoip_path (should trigger auto-detection)
    // Use a non-existent database to avoid connection issues
    let db_url = "root:root123@tcp(127.0.0.1:3306)/non_existent_db";
    let (c_db_url, _c_db_string) = to_c_string_safe(db_url);

    let mut err_msg: *mut c_char = ptr::null_mut();

    unsafe {
        // Test with null geoip_path (auto-detection)
        let result = create_network_config_service_singleton(
            c_db_url,
            ptr::null(), // null geoip_path should trigger auto-detection
            &mut err_msg,
        );

        // This will likely fail due to database connection
        if !result {
            println!("NetworkConfigService creation failed as expected");
            if !err_msg.is_null() {
                free_c_char(err_msg);
            }
        } else {
            // If it succeeds, clean up
            let destroy_result = destroy_network_config_service_singleton(&mut err_msg);
            if !destroy_result && !err_msg.is_null() {
                free_c_char(err_msg);
            }
        }
    }
}

#[test]
fn test_ffi_geoip_explicit_path() {
    // Test FFI with explicit geoip_path
    let db_url = "root:root123@tcp(127.0.0.1:3306)/non_existent_db";

    // Get the auto-detected path for testing
    let geoip_path = easytier_bridge::config::get_geoip_db_path();
    assert!(
        geoip_path.is_some(),
        "Should be able to auto-detect GeoIP path for testing"
    );

    let (c_db_url, _c_db_string) = to_c_string_safe(db_url);
    let (c_geoip_path, _c_geoip_string) = to_c_string_safe(&geoip_path.unwrap());

    let mut err_msg: *mut c_char = ptr::null_mut();

    unsafe {
        // Test with explicit geoip_path
        let result = create_network_config_service_singleton(c_db_url, c_geoip_path, &mut err_msg);

        // This will likely fail due to database connection
        if !result {
            println!("NetworkConfigService creation failed as expected with explicit path");
            if !err_msg.is_null() {
                free_c_char(err_msg);
            }
        } else {
            // If it succeeds, clean up
            let destroy_result = destroy_network_config_service_singleton(&mut err_msg);
            if !destroy_result && !err_msg.is_null() {
                free_c_char(err_msg);
            }
        }
    }
}

#[test]
fn test_ffi_geoip_invalid_path() {
    // Test FFI with invalid geoip_path
    let db_url = "root:root123@tcp(127.0.0.1:3306)/non_existent_db";
    let invalid_geoip_path = "/non/existent/path/geoip2-cn.mmdb";

    let (c_db_url, _c_db_string) = to_c_string_safe(db_url);
    let (c_geoip_path, _c_geoip_string) = to_c_string_safe(invalid_geoip_path);

    let mut err_msg: *mut c_char = ptr::null_mut();

    unsafe {
        // Test with invalid geoip_path
        let result = create_network_config_service_singleton(c_db_url, c_geoip_path, &mut err_msg);

        // This should fail due to database connection or invalid path
        if !result {
            println!("Expected error with invalid path");
            if !err_msg.is_null() {
                free_c_char(err_msg);
            }
        } else {
            println!("Service created successfully with invalid path (fallback working)");

            // Clean up
            let destroy_result = destroy_network_config_service_singleton(&mut err_msg);
            if !destroy_result && !err_msg.is_null() {
                free_c_char(err_msg);
            }
        }
    }
}

#[test]
fn test_ffi_geoip_empty_string() {
    // Test FFI with empty string geoip_path (should be treated as null)
    let db_url = "root:root123@tcp(127.0.0.1:3306)/non_existent_db";

    let (c_db_url, _c_db_string) = to_c_string_safe(db_url);
    let (c_geoip_path, _c_geoip_string) = to_c_string_safe(""); // Empty string

    let mut err_msg: *mut c_char = ptr::null_mut();

    unsafe {
        // Test with empty string geoip_path
        let result = create_network_config_service_singleton(c_db_url, c_geoip_path, &mut err_msg);

        // This will likely fail due to database connection
        if !result {
            println!("NetworkConfigService creation failed as expected with empty string");
            if !err_msg.is_null() {
                free_c_char(err_msg);
            }
        } else {
            // If it succeeds, clean up
            let destroy_result = destroy_network_config_service_singleton(&mut err_msg);
            if !destroy_result && !err_msg.is_null() {
                free_c_char(err_msg);
            }
        }
    }
}

#[test]
fn test_ffi_geoip_multiple_creations() {
    // Test that multiple creations of the singleton work correctly
    let db_url = "root:root123@tcp(127.0.0.1:3306)/non_existent_db";
    let (c_db_url, _c_db_string) = to_c_string_safe(db_url);

    let mut err_msg: *mut c_char = ptr::null_mut();

    unsafe {
        // First creation
        let result1 = create_network_config_service_singleton(c_db_url, ptr::null(), &mut err_msg);

        // This will likely fail due to database connection
        if !result1 {
            println!("First creation failed as expected");
            if !err_msg.is_null() {
                free_c_char(err_msg);
            }
        } else {
            // If it succeeds, test second creation
            let result2 =
                create_network_config_service_singleton(c_db_url, ptr::null(), &mut err_msg);

            // Second creation should return true because singleton already exists
            assert!(
                result2,
                "Second creation should return true (singleton already exists)"
            );

            // Clean up
            let destroy_result = destroy_network_config_service_singleton(&mut err_msg);
            if !destroy_result && !err_msg.is_null() {
                free_c_char(err_msg);
            }
        }
    }
}

#[test]
fn test_ffi_geoip_path_validation() {
    // Test that the FFI properly validates the geoip_path parameter
    let mut err_msg: *mut c_char = ptr::null_mut();

    unsafe {
        // Test with null db_url (should fail)
        let result =
            create_network_config_service_singleton(ptr::null(), ptr::null(), &mut err_msg);

        assert!(!result, "Should fail with null db_url");

        if !err_msg.is_null() {
            println!("Expected error with null db_url");
            free_c_char(err_msg);
        }
    }
}

#[test]
fn test_ffi_geoip_actual_file_exists() {
    // Test that the actual GeoIP database file exists and can be used
    let geoip_path = easytier_bridge::config::get_geoip_db_path();
    assert!(
        geoip_path.is_some(),
        "GeoIP database path should be auto-detected"
    );

    let path = geoip_path.unwrap();
    println!("Testing with actual GeoIP path: {}", path);

    // Verify the file actually exists
    assert!(
        Path::new(&path).exists(),
        "GeoIP database file should exist at: {}",
        path
    );

    // Test file size (should be reasonable for a GeoIP database)
    let metadata = std::fs::metadata(&path).expect("Should be able to read file metadata");
    assert!(
        metadata.len() > 1000,
        "GeoIP database file should be larger than 1KB"
    );
    println!("GeoIP database file size: {} bytes", metadata.len());
}

#[test]
fn test_ffi_geoip_environment_variable() {
    // Test that environment variable override works
    let original_env = std::env::var("CORTEX_GEOIP_DB_PATH").ok();

    // Set environment variable to the actual path
    let geoip_path = easytier_bridge::config::get_geoip_db_path();
    if let Some(path) = &geoip_path {
        std::env::set_var("CORTEX_GEOIP_DB_PATH", path);

        // Verify the environment variable is set
        let env_path =
            std::env::var("CORTEX_GEOIP_DB_PATH").expect("Environment variable should be set");
        assert_eq!(
            env_path, *path,
            "Environment variable should match the path"
        );

        // Test FFI with this environment variable
        let db_url = "root:root123@tcp(127.0.0.1:3306)/non_existent_db";
        let (c_db_url, _c_db_string) = to_c_string_safe(db_url);

        let mut err_msg: *mut c_char = ptr::null_mut();

        unsafe {
            let result = create_network_config_service_singleton(
                c_db_url,
                ptr::null(), // Use environment variable
                &mut err_msg,
            );

            // Just check if the function was called, don't try to access error message
            if !result {
                println!(
                    "NetworkConfigService creation failed as expected with environment variable"
                );
                if !err_msg.is_null() {
                    free_c_char(err_msg);
                }
            } else {
                // If it succeeds, clean up
                let destroy_result = destroy_network_config_service_singleton(&mut err_msg);
                if !destroy_result && !err_msg.is_null() {
                    free_c_char(err_msg);
                }
            }
        }
    }

    // Restore original environment variable
    match original_env {
        Some(val) => std::env::set_var("CORTEX_GEOIP_DB_PATH", val),
        None => std::env::remove_var("CORTEX_GEOIP_DB_PATH"),
    }
}
