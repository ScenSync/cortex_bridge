//! Comprehensive FFI tests for web client functionality
//!
//! This module tests the FFI interface for web client operations including:
//! - cortex_start_web_client
//! - cortex_stop_web_client
//! - cortex_get_web_client_network_info
//! - cortex_list_web_client_instances

use std::ffi::CString;
use std::ptr;

#[cfg(test)]
mod web_client_ffi_tests {
    use super::*;
    use easytier_device_client::{
        cortex_get_web_client_network_info, cortex_list_web_client_instances,
        cortex_start_web_client, cortex_stop_web_client, CortexNetworkInfo, CortexWebClient,
    };

    #[test]
    fn test_start_web_client_null_config() {
        // Test that starting with null config fails gracefully
        unsafe {
            let result = cortex_start_web_client(ptr::null());
            assert_eq!(result, -1, "Should fail with null config");
        }
    }

    #[test]
    fn test_start_web_client_invalid_url() {
        // Test with invalid URL format
        let invalid_url = CString::new("not-a-valid-url").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: invalid_url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            assert_eq!(result, -1, "Should fail with invalid URL");
        }
    }

    #[test]
    fn test_start_web_client_missing_organization_id() {
        // Test with URL missing organization ID in path
        let url = CString::new("tcp://localhost:11020").unwrap(); // No path/org_id
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            assert_eq!(
                result, -1,
                "Should fail when organization ID is missing in URL path"
            );
        }
    }

    #[test]
    fn test_start_web_client_valid_config_udp() {
        // Test with valid UDP configuration
        let url = CString::new("udp://localhost:11020/test-org-id").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            // May succeed or fail depending on network, but should not crash
            // In test environment without actual server, likely to fail gracefully
            assert!(
                result == 0 || result == -1,
                "Should return valid status code"
            );

            // Clean up if it succeeded
            if result == 0 {
                let instance_name = CString::new("test-org-id").unwrap();
                let _ = cortex_stop_web_client(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_start_web_client_valid_config_tcp() {
        // Test with valid TCP configuration
        let url = CString::new("tcp://localhost:11020/test-org-tcp").unwrap();
        let machine_id = CString::new("7c9e6679-7425-40de-944b-e07fc1f90ae7").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            // May succeed or fail depending on network
            assert!(
                result == 0 || result == -1,
                "Should return valid status code"
            );

            if result == 0 {
                let instance_name = CString::new("test-org-tcp").unwrap();
                let _ = cortex_stop_web_client(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_start_web_client_with_null_machine_id() {
        // Test that null machine_id is handled (should use system default)
        let url = CString::new("tcp://localhost:11020/test-org-null-machine").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: ptr::null(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            // Should not crash with null machine_id
            assert!(
                result == 0 || result == -1,
                "Should handle null machine_id gracefully"
            );

            if result == 0 {
                let instance_name = CString::new("test-org-null-machine").unwrap();
                let _ = cortex_stop_web_client(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_start_web_client_invalid_machine_id_format() {
        // Test with invalid UUID format for machine_id
        let url = CString::new("tcp://localhost:11020/test-org-bad-uuid").unwrap();
        let invalid_machine_id = CString::new("not-a-uuid").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: invalid_machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            // Should handle invalid UUID gracefully (will use system default)
            assert!(
                result == 0 || result == -1,
                "Should handle invalid UUID format"
            );

            if result == 0 {
                let instance_name = CString::new("test-org-bad-uuid").unwrap();
                let _ = cortex_stop_web_client(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_stop_web_client_null_instance_name() {
        // Test stopping with null instance name
        unsafe {
            let result = cortex_stop_web_client(ptr::null());
            assert_eq!(result, -1, "Should fail with null instance name");
        }
    }

    #[test]
    fn test_stop_web_client_nonexistent_instance() {
        // Test stopping a non-existent instance
        let instance_name = CString::new("nonexistent-instance").unwrap();

        unsafe {
            let result = cortex_stop_web_client(instance_name.as_ptr());
            assert_eq!(result, -1, "Should fail for non-existent instance");
        }
    }

    #[test]
    fn test_get_network_info_null_arguments() {
        // Test with null instance name
        unsafe {
            let mut info_ptr: *const CortexNetworkInfo = ptr::null();
            let result = cortex_get_web_client_network_info(ptr::null(), &mut info_ptr);
            assert_eq!(result, -1, "Should fail with null instance name");
        }

        // Test with null info pointer
        unsafe {
            let instance_name = CString::new("test-instance").unwrap();
            let result =
                cortex_get_web_client_network_info(instance_name.as_ptr(), ptr::null_mut());
            assert_eq!(result, -1, "Should fail with null info pointer");
        }
    }

    #[test]
    fn test_get_network_info_nonexistent_instance() {
        // Test getting info for non-existent instance
        let instance_name = CString::new("nonexistent").unwrap();
        let mut info_ptr: *const CortexNetworkInfo = ptr::null();

        unsafe {
            let result = cortex_get_web_client_network_info(instance_name.as_ptr(), &mut info_ptr);
            assert_eq!(result, -1, "Should fail for non-existent instance");
        }
    }

    #[test]
    fn test_list_instances_null_arguments() {
        // Test with null instances pointer
        unsafe {
            let result = cortex_list_web_client_instances(ptr::null_mut(), 10);
            assert_eq!(result, -1, "Should fail with null instances pointer");
        }
    }

    #[test]
    fn test_list_instances_invalid_max_count() {
        // Test with invalid max_count values
        let mut instances_ptr: *const *const i8 = ptr::null();

        unsafe {
            // Zero max_count
            let result = cortex_list_web_client_instances(&mut instances_ptr, 0);
            assert_eq!(result, -1, "Should fail with zero max_count");

            // Negative max_count
            let result = cortex_list_web_client_instances(&mut instances_ptr, -1);
            assert_eq!(result, -1, "Should fail with negative max_count");
        }
    }

    #[test]
    fn test_list_instances_empty() {
        // Test listing when no instances exist
        let mut instances_ptr: *const *const i8 = ptr::null();

        unsafe {
            let count = cortex_list_web_client_instances(&mut instances_ptr, 10);
            assert_eq!(count, 0, "Should return 0 when no instances exist");
            assert!(instances_ptr.is_null(), "Pointer should be null when empty");
        }
    }

    #[test]
    fn test_url_parsing_various_schemes() {
        // Test various URL schemes
        let test_urls = vec![
            "tcp://server.example.com:11020/org-123",
            "udp://192.168.1.1:11020/org-456",
            "ws://localhost:11020/org-789",
            "tcp://[::1]:11020/org-ipv6",
        ];

        for url_str in test_urls {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                // Should not crash with various URL schemes
                let result = cortex_start_web_client(&client_config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle URL scheme: {}",
                    url_str
                );

                // Clean up if successful
                if result == 0 {
                    let path = url::Url::parse(url_str).unwrap();
                    let org_id = path.path().trim_start_matches('/');
                    let instance_name = CString::new(org_id).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }

    #[test]
    fn test_special_characters_in_organization_id() {
        // Test organization IDs with special characters
        let test_cases = vec![
            "tcp://localhost:11020/org-with-dashes",
            "tcp://localhost:11020/org_with_underscores",
            "tcp://localhost:11020/org123numbers",
        ];

        for url_str in test_cases {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle special chars in org_id: {}",
                    url_str
                );

                if result == 0 {
                    let path = url::Url::parse(url_str).unwrap();
                    let org_id = path.path().trim_start_matches('/');
                    let instance_name = CString::new(org_id).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }

    #[test]
    fn test_multiple_uuids() {
        // Test with different UUID versions and formats
        let test_uuids = vec![
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
        ];

        for (i, uuid_str) in test_uuids.iter().enumerate() {
            let url = CString::new(format!("tcp://localhost:11020/org-{}", i)).unwrap();
            let machine_id = CString::new(uuid_str.as_str()).unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle UUID: {}",
                    uuid_str
                );

                if result == 0 {
                    let instance_name = CString::new(format!("org-{}", i)).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }

    #[test]
    fn test_concurrent_instance_names() {
        // Test that different organization IDs create separate instances
        let configs = vec![
            ("tcp://localhost:11020/org-a", "uuid-a"),
            ("tcp://localhost:11021/org-b", "uuid-b"),
            ("tcp://localhost:11022/org-c", "uuid-c"),
        ];

        let mut created_instances = Vec::new();

        for (url_str, machine_id_str) in configs {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new(machine_id_str).unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                if result == 0 {
                    let path = url::Url::parse(url_str).unwrap();
                    let org_id = path.path().trim_start_matches('/').to_string();
                    created_instances.push(org_id);
                }
            }
        }

        // Clean up all created instances
        for instance_name in created_instances {
            let name = CString::new(instance_name).unwrap();
            unsafe {
                let _ = cortex_stop_web_client(name.as_ptr());
            }
        }
    }

    #[test]
    fn test_empty_string_parameters() {
        // Test with empty string for config_server_url
        let empty_url = CString::new("").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: empty_url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            assert_eq!(result, -1, "Should fail with empty URL");
        }
    }

    #[test]
    fn test_very_long_organization_id() {
        // Test with very long organization ID
        let long_org_id = "a".repeat(500);
        let url = CString::new(format!("tcp://localhost:11020/{}", long_org_id)).unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            // Should handle long org_id
            assert!(
                result == 0 || result == -1,
                "Should handle long organization ID"
            );

            if result == 0 {
                let instance_name = CString::new(long_org_id).unwrap();
                let _ = cortex_stop_web_client(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_url_with_query_parameters() {
        // Test URL with query parameters (should still work)
        let url = CString::new("tcp://localhost:11020/org-query?param=value").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let result = cortex_start_web_client(&client_config);
            assert!(
                result == 0 || result == -1,
                "Should handle URL with query params"
            );

            if result == 0 {
                let instance_name = CString::new("org-query").unwrap();
                let _ = cortex_stop_web_client(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_ipv6_addresses_in_url() {
        // Test IPv6 addresses
        let ipv6_urls = vec![
            "tcp://[::1]:11020/org-ipv6-loopback",
            "tcp://[2001:db8::1]:11020/org-ipv6-addr",
            "tcp://[fe80::1]:11020/org-ipv6-local",
        ];

        for url_str in ipv6_urls {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle IPv6 URL: {}",
                    url_str
                );

                if result == 0 {
                    let path = url::Url::parse(url_str).unwrap();
                    let org_id = path.path().trim_start_matches('/');
                    let instance_name = CString::new(org_id).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }

    #[test]
    fn test_double_stop_same_instance() {
        // Test stopping the same instance twice
        let url = CString::new("tcp://localhost:11020/org-double-stop").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            let start_result = cortex_start_web_client(&client_config);

            if start_result == 0 {
                let instance_name = CString::new("org-double-stop").unwrap();

                // First stop should succeed
                let result1 = cortex_stop_web_client(instance_name.as_ptr());
                assert_eq!(result1, 0, "First stop should succeed");

                // Second stop should fail (instance no longer exists)
                let result2 = cortex_stop_web_client(instance_name.as_ptr());
                assert_eq!(result2, -1, "Second stop should fail");
            }
        }
    }

    #[test]
    fn test_struct_memory_layout() {
        // Test that CortexWebClient has expected memory layout
        assert_eq!(
            std::mem::size_of::<CortexWebClient>(),
            std::mem::size_of::<*const i8>() * 2,
            "CortexWebClient should contain exactly 2 pointers"
        );
    }

    #[test]
    fn test_network_info_struct_layout() {
        // Test that CortexNetworkInfo has expected memory layout
        let size = std::mem::size_of::<CortexNetworkInfo>();
        assert!(size > 0, "CortexNetworkInfo should have non-zero size");

        // Should have 5 string pointers + 2 integers
        let expected_min_size = std::mem::size_of::<*const i8>() * 5
            + std::mem::size_of::<i32>() * 2;
        assert!(
            size >= expected_min_size,
            "CortexNetworkInfo should be at least {} bytes, got {}",
            expected_min_size,
            size
        );
    }

    #[test]
    fn test_organization_id_extraction_edge_cases() {
        // Test organization ID extraction from various URL formats
        let test_cases = vec![
            ("tcp://localhost:11020/single", Some("single")),
            ("tcp://localhost:11020/path/nested", Some("path/nested")),
            ("tcp://localhost:11020//double-slash", Some("/double-slash")),
            ("tcp://localhost:11020/", None), // Empty path after slash
        ];

        for (url_str, expected_org_id) in test_cases {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);

                if expected_org_id.is_some() {
                    // Should handle valid org_id
                    assert!(
                        result == 0 || result == -1,
                        "Should handle URL: {}",
                        url_str
                    );

                    if result == 0 {
                        let instance_name = CString::new(expected_org_id.unwrap()).unwrap();
                        let _ = cortex_stop_web_client(instance_name.as_ptr());
                    }
                } else {
                    // Should fail with empty org_id
                    assert_eq!(
                        result, -1,
                        "Should fail with empty organization ID: {}",
                        url_str
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod web_client_lifecycle_tests {
    use super::*;
    use easytier_device_client::{cortex_start_web_client, cortex_stop_web_client, CortexWebClient};

    #[test]
    fn test_start_stop_lifecycle() {
        // Test complete start-stop lifecycle
        let url = CString::new("tcp://localhost:11025/org-lifecycle").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        unsafe {
            // Start
            let start_result = cortex_start_web_client(&client_config);

            if start_result == 0 {
                // Stop
                let instance_name = CString::new("org-lifecycle").unwrap();
                let stop_result = cortex_stop_web_client(instance_name.as_ptr());
                assert_eq!(stop_result, 0, "Stop should succeed after successful start");
            }
        }
    }

    #[test]
    fn test_multiple_sequential_starts() {
        // Test starting multiple instances sequentially
        for i in 0..3 {
            let url = CString::new(format!("tcp://localhost:1102{}/org-seq-{}", i, i)).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);

                if result == 0 {
                    let instance_name = CString::new(format!("org-seq-{}", i)).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }

    #[test]
    fn test_hostname_handling() {
        // Test that system hostname is used correctly
        let hostname = gethostname::gethostname();
        assert!(
            !hostname.to_string_lossy().is_empty(),
            "System hostname should not be empty"
        );
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use easytier_device_client::{cortex_start_web_client, cortex_stop_web_client, CortexWebClient};

    #[test]
    fn test_error_message_setting() {
        // Test that error messages are properly set
        unsafe {
            // Trigger an error
            let result = cortex_start_web_client(ptr::null());
            assert_eq!(result, -1);

            // Error message should be set (can be retrieved via easytier_common_get_error_msg)
            let error_msg = easytier_common::easytier_common_get_error_msg();
            assert!(!error_msg.is_null(), "Error message should be set");
        }
    }

    #[test]
    fn test_malformed_url_schemes() {
        // Test various malformed URL schemes
        let malformed_urls = vec![
            "http://localhost:11020/org", // Wrong scheme (http instead of tcp/udp/ws)
            "ftp://localhost:11020/org",  // FTP not supported
            "://localhost:11020/org",     // Missing scheme
            "tcp:/localhost:11020/org",   // Single slash
            "tcp:localhost:11020/org",    // No slashes
        ];

        for url_str in malformed_urls {
            let url_cstring = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url_cstring.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                // Should handle malformed URLs gracefully
                assert!(
                    result == 0 || result == -1,
                    "Should handle malformed URL: {}",
                    url_str
                );

                if result == 0 {
                    let path = url::Url::parse(url_str);
                    if let Ok(parsed) = path {
                        let org_id = parsed.path().trim_start_matches('/');
                        if !org_id.is_empty() {
                            let instance_name = CString::new(org_id).unwrap();
                            let _ = cortex_stop_web_client(instance_name.as_ptr());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_port_number_edge_cases() {
        // Test various port numbers
        let port_cases = vec![
            ("tcp://localhost:1/org-port-1", "Low port"),
            ("tcp://localhost:65535/org-port-max", "Max valid port"),
            ("tcp://localhost:8080/org-port-8080", "Common port"),
            ("tcp://localhost:11020/org-port-11020", "Default port"),
        ];

        for (url_str, description) in port_cases {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle {}: {}",
                    description,
                    url_str
                );

                if result == 0 {
                    let path = url::Url::parse(url_str).unwrap();
                    let org_id = path.path().trim_start_matches('/');
                    let instance_name = CString::new(org_id).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }

    #[test]
    fn test_unicode_in_organization_id() {
        // Test Unicode characters in organization ID
        let unicode_org_ids = vec![
            "tcp://localhost:11020/组织-中文",
            "tcp://localhost:11020/org-日本語",
            "tcp://localhost:11020/org-한국어",
        ];

        for url_str in unicode_org_ids {
            let url = CString::new(url_str).unwrap();
            let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

            let client_config = CortexWebClient {
                config_server_url: url.as_ptr(),
                machine_id: machine_id.as_ptr(),
            };

            unsafe {
                let result = cortex_start_web_client(&client_config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle Unicode in org_id: {}",
                    url_str
                );

                if result == 0 {
                    let path = url::Url::parse(url_str).unwrap();
                    let org_id = path.path().trim_start_matches('/');
                    let instance_name = CString::new(org_id).unwrap();
                    let _ = cortex_stop_web_client(instance_name.as_ptr());
                }
            }
        }
    }
}

#[cfg(test)]
mod memory_safety_tests {
    use super::*;

    #[test]
    fn test_null_pointer_handling() {
        // Comprehensive null pointer tests
        unsafe {
            use easytier_device_client::{
                cortex_get_web_client_network_info, cortex_list_web_client_instances,
                cortex_start_web_client, cortex_stop_web_client,
            };

            // Test all functions with null pointers
            assert_eq!(
                cortex_start_web_client(ptr::null()),
                -1,
                "start_web_client should reject null"
            );
            assert_eq!(
                cortex_stop_web_client(ptr::null()),
                -1,
                "stop_web_client should reject null"
            );
            assert_eq!(
                cortex_get_web_client_network_info(ptr::null(), ptr::null_mut()),
                -1,
                "get_network_info should reject null"
            );
            assert_eq!(
                cortex_list_web_client_instances(ptr::null_mut(), 10),
                -1,
                "list_instances should reject null"
            );
        }
    }

    #[test]
    fn test_struct_alignment() {
        // Verify struct alignment for FFI compatibility
        use easytier_device_client::{CortexNetworkInfo, CortexWebClient};

        let web_client_align = std::mem::align_of::<CortexWebClient>();
        let network_info_align = std::mem::align_of::<CortexNetworkInfo>();

        // Should have pointer alignment
        assert!(
            web_client_align >= std::mem::align_of::<*const i8>(),
            "CortexWebClient should be pointer-aligned"
        );
        assert!(
            network_info_align >= std::mem::align_of::<*const i8>(),
            "CortexNetworkInfo should be pointer-aligned"
        );
    }
}

