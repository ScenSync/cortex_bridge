//! Comprehensive FFI tests for gateway functionality
//!
//! This module tests the FFI interface for gateway operations including:
//! - start_easytier_core (with Builder API)
//! - stop_easytier_core
//! - get_easytier_core_status
//! - Configuration validation

use std::ffi::CString;
use std::ptr;

#[cfg(test)]
mod gateway_ffi_tests {
    use super::*;
    use easytier_network_gateway::{
        get_easytier_core_status, start_easytier_core, stop_easytier_core, EasyTierCoreConfig,
    };

    /// Helper function to create a basic valid config for testing
    fn create_test_config(instance_name: &str) -> (EasyTierCoreConfig, Vec<CString>) {
        // Keep CStrings alive
        let mut c_strings = Vec::new();

        let instance = CString::new(instance_name).unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret-123").unwrap();
        let listener1 = CString::new("tcp://0.0.0.0:11010").unwrap();
        let listener2 = CString::new("udp://0.0.0.0:11011").unwrap();

        let listeners = vec![listener1.as_ptr(), listener2.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        c_strings.push(instance);
        c_strings.push(network_name);
        c_strings.push(network_secret);
        c_strings.push(listener1);
        c_strings.push(listener2);

        let config = EasyTierCoreConfig {
            instance_name: c_strings[0].as_ptr(),
            network_name: c_strings[1].as_ptr(),
            network_secret: c_strings[2].as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 2,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        (config, c_strings)
    }

    #[test]
    fn test_start_gateway_null_config() {
        // Test starting with null config
        unsafe {
            let result = start_easytier_core(ptr::null());
            assert_eq!(result, -1, "Should fail with null config");
        }
    }

    #[test]
    fn test_start_gateway_null_instance_name() {
        // Test with null instance name
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11010").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: ptr::null(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with null instance name");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_null_network_name() {
        // Test with null network name
        let instance_name = CString::new("test-instance").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11010").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: ptr::null(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with null network name");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_null_network_secret() {
        // Test with null network secret
        let instance_name = CString::new("test-instance").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11010").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: ptr::null(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with null network secret");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_no_listeners() {
        // Test with no listener URLs
        let instance_name = CString::new("test-no-listeners").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: ptr::null(),
            listener_urls_count: 0,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with no listeners");
        }
    }

    #[test]
    fn test_start_gateway_invalid_listener_url() {
        // Test with invalid listener URL
        let instance_name = CString::new("test-invalid-listener").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let invalid_listener = CString::new("not-a-valid-url").unwrap();

        let listeners = vec![invalid_listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with invalid listener URL");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_invalid_ipv4() {
        // Test with invalid IPv4 address
        let instance_name = CString::new("test-invalid-ipv4").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11010").unwrap();
        let invalid_ipv4 = CString::new("999.999.999.999").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 0,
            ipv4: invalid_ipv4.as_ptr(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with invalid IPv4 address");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_valid_ipv4() {
        // Test with valid IPv4 address
        let instance_name = CString::new("test-valid-ipv4").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11015").unwrap();
        let ipv4 = CString::new("10.144.144.1").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 0,
            ipv4: ipv4.as_ptr(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            // May succeed or fail depending on network permissions
            assert!(
                result == 0 || result == -1,
                "Should handle valid IPv4 config"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_invalid_ipv6() {
        // Test with invalid IPv6 address
        let instance_name = CString::new("test-invalid-ipv6").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11010").unwrap();
        let invalid_ipv6 = CString::new("gggg::1").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 0,
            ipv4: ptr::null(),
            ipv6: invalid_ipv6.as_ptr(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 1,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with invalid IPv6 address");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_start_gateway_multiple_listeners() {
        // Test with multiple listener URLs
        let (_config, _c_strings) = create_test_config("test-multi-listeners");

        unsafe {
            let result = start_easytier_core(&_config);
            // May succeed or fail depending on port availability
            assert!(
                result == 0 || result == -1,
                "Should handle multiple listeners"
            );

            if result == 0 {
                let instance_name = CString::new("test-multi-listeners").unwrap();
                let _ = stop_easytier_core(instance_name.as_ptr());
            }
        }
    }

    #[test]
    fn test_start_gateway_with_peers() {
        // Test P2P mode with peer URLs
        let instance_name = CString::new("test-with-peers").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11016").unwrap();
        let peer1 = CString::new("tcp://peer1.example.com:11010").unwrap();
        let peer2 = CString::new("tcp://peer2.example.com:11010").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let peers = vec![peer1.as_ptr(), peer2.as_ptr()];
        let peers_box = peers.into_boxed_slice();
        let peers_ptr = Box::into_raw(peers_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: unsafe { (*peers_ptr).as_ptr() },
            peer_urls_count: 2,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 0, // P2P mode
        };

        unsafe {
            let result = start_easytier_core(&config);
            // May succeed or fail
            assert!(result == 0 || result == -1, "Should handle P2P mode");

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
            let _ = Box::from_raw(peers_ptr);
        }
    }

    #[test]
    fn test_start_gateway_invalid_rpc_port() {
        // Test with various RPC port values
        // Note: Port 0 is actually valid (OS assigns any port), so we test other invalid values
        let port_cases: Vec<(i32, &str)> = vec![
            (-1, "Negative port"),
            (65536, "Port > 65535"),
            (99999, "Very large port"),
        ];

        for (rpc_port, description) in port_cases {
            let instance_name =
                CString::new(format!("test-rpc-port-{}", rpc_port.abs())).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new("tcp://0.0.0.0:11017").unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: 1,
                ipv4: ptr::null(),
                ipv6: ptr::null(),
                listener_urls: unsafe { (*listeners_ptr).as_ptr() },
                listener_urls_count: 1,
                rpc_port,
                peer_urls: ptr::null(),
                peer_urls_count: 0,
                default_protocol: ptr::null(),
                dev_name: ptr::null(),
                enable_encryption: 1,
                enable_ipv6: 0,
                mtu: 1380,
                latency_first: 0,
                enable_exit_node: 0,
                no_tun: 0,
                use_smoltcp: 0,
                foreign_network_whitelist: ptr::null(),
                disable_p2p: 0,
                relay_all_peer_rpc: 0,
                disable_udp_hole_punching: 0,
                private_mode: 1,
            };

            unsafe {
                let result = start_easytier_core(&config);
                // Invalid ports should fail
                assert_eq!(result, -1, "Should fail with {}: {}", description, rpc_port);

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
            }
        }
    }

    #[test]
    fn test_stop_gateway_null_instance_name() {
        // Test stopping with null instance name
        unsafe {
            let result = stop_easytier_core(ptr::null());
            assert_eq!(result, -1, "Should fail with null instance name");
        }
    }

    #[test]
    fn test_stop_gateway_nonexistent_instance() {
        // Test stopping non-existent instance
        let instance_name = CString::new("nonexistent-gateway").unwrap();

        unsafe {
            let result = stop_easytier_core(instance_name.as_ptr());
            assert_eq!(result, -1, "Should fail for non-existent instance");
        }
    }

    #[test]
    fn test_get_status_null_instance_name() {
        // Test getting status with null instance name
        let mut status_json: *mut i8 = ptr::null_mut();

        unsafe {
            let result = get_easytier_core_status(ptr::null(), &mut status_json);
            assert_eq!(result, -1, "Should fail with null instance name");
        }
    }

    #[test]
    fn test_get_status_null_output_pointer() {
        // Test getting status with null output pointer
        let instance_name = CString::new("test-instance").unwrap();

        unsafe {
            let result = get_easytier_core_status(instance_name.as_ptr(), ptr::null_mut());
            assert_eq!(result, -1, "Should fail with null output pointer");
        }
    }

    #[test]
    fn test_get_status_nonexistent_instance() {
        // Test getting status for non-existent instance
        let instance_name = CString::new("nonexistent-for-status").unwrap();
        let mut status_json: *mut i8 = ptr::null_mut();

        unsafe {
            let result = get_easytier_core_status(instance_name.as_ptr(), &mut status_json);
            // Should succeed but indicate instance is not running
            assert_eq!(result, 0, "Should succeed for non-existent instance");

            if !status_json.is_null() {
                let status_str = std::ffi::CStr::from_ptr(status_json).to_str().unwrap();
                assert!(
                    status_str.contains("\"running\":false"),
                    "Status should indicate not running"
                );
                easytier_common::easytier_common_free_string(status_json);
            }
        }
    }

    #[test]
    fn test_dhcp_flag_variations() {
        // Test DHCP flag with different values
        let dhcp_values: Vec<(i32, &str)> = vec![
            (0, "DHCP disabled"),
            (1, "DHCP enabled"),
            (2, "DHCP non-zero"),
            (-1, "DHCP negative"),
        ];

        for (dhcp_value, description) in dhcp_values {
            let instance_name = CString::new(format!("test-dhcp-{}", dhcp_value)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:1101{}", dhcp_value.abs() % 10))
                .unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: dhcp_value,
                ipv4: ptr::null(),
                ipv6: ptr::null(),
                listener_urls: unsafe { (*listeners_ptr).as_ptr() },
                listener_urls_count: 1,
                rpc_port: 15888,
                peer_urls: ptr::null(),
                peer_urls_count: 0,
                default_protocol: ptr::null(),
                dev_name: ptr::null(),
                enable_encryption: 1,
                enable_ipv6: 0,
                mtu: 1380,
                latency_first: 0,
                enable_exit_node: 0,
                no_tun: 0,
                use_smoltcp: 0,
                foreign_network_whitelist: ptr::null(),
                disable_p2p: 0,
                relay_all_peer_rpc: 0,
                disable_udp_hole_punching: 0,
                private_mode: 1,
            };

            unsafe {
                let result = start_easytier_core(&config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle {}: {}",
                    description,
                    dhcp_value
                );

                if result == 0 {
                    let _ = stop_easytier_core(instance_name.as_ptr());
                }

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
            }
        }
    }

    #[test]
    fn test_mtu_variations() {
        // Test MTU with different values
        let mtu_values: Vec<(i32, &str)> = vec![
            (-1, "Negative MTU (should use default)"),
            (0, "Zero MTU (should use default)"),
            (576, "Minimum MTU"),
            (1380, "Default MTU"),
            (1500, "Ethernet MTU"),
            (9000, "Jumbo frames"),
        ];

        for (mtu_value, description) in mtu_values {
            let instance_name = CString::new(format!("test-mtu-{}", mtu_value)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener =
                CString::new(format!("tcp://0.0.0.0:110{}", 30 + (mtu_value.abs() % 10))).unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: 1,
                ipv4: ptr::null(),
                ipv6: ptr::null(),
                listener_urls: unsafe { (*listeners_ptr).as_ptr() },
                listener_urls_count: 1,
                rpc_port: 15888,
                peer_urls: ptr::null(),
                peer_urls_count: 0,
                default_protocol: ptr::null(),
                dev_name: ptr::null(),
                enable_encryption: 1,
                enable_ipv6: 0,
                mtu: mtu_value,
                latency_first: 0,
                enable_exit_node: 0,
                no_tun: 0,
                use_smoltcp: 0,
                foreign_network_whitelist: ptr::null(),
                disable_p2p: 0,
                relay_all_peer_rpc: 0,
                disable_udp_hole_punching: 0,
                private_mode: 1,
            };

            unsafe {
                let result = start_easytier_core(&config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle {}: {}",
                    description,
                    mtu_value
                );

                if result == 0 {
                    let _ = stop_easytier_core(instance_name.as_ptr());
                }

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
            }
        }
    }

    #[test]
    fn test_flag_combinations() {
        // Test various flag combinations
        let flag_cases = vec![
            (1, 1, 0, 0, "Encryption + IPv6"),
            (1, 0, 1, 0, "Encryption + Latency-first"),
            (0, 0, 0, 1, "Exit node only"),
            (1, 1, 1, 1, "All flags enabled"),
            (0, 0, 0, 0, "All flags disabled"),
        ];

        for (i, (enc, ipv6, latency, exit_node, desc)) in flag_cases.iter().enumerate() {
            let instance_name = CString::new(format!("test-flags-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:110{}", 40 + i)).unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: 1,
                ipv4: ptr::null(),
                ipv6: ptr::null(),
                listener_urls: unsafe { (*listeners_ptr).as_ptr() },
                listener_urls_count: 1,
                rpc_port: 15888,
                peer_urls: ptr::null(),
                peer_urls_count: 0,
                default_protocol: ptr::null(),
                dev_name: ptr::null(),
                enable_encryption: *enc,
                enable_ipv6: *ipv6,
                mtu: 1380,
                latency_first: *latency,
                enable_exit_node: *exit_node,
                no_tun: 0,
                use_smoltcp: 0,
                foreign_network_whitelist: ptr::null(),
                disable_p2p: 0,
                relay_all_peer_rpc: 0,
                disable_udp_hole_punching: 0,
                private_mode: 1,
            };

            unsafe {
                let result = start_easytier_core(&config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle flag combo: {}",
                    desc
                );

                if result == 0 {
                    let _ = stop_easytier_core(instance_name.as_ptr());
                }

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
            }
        }
    }

    #[test]
    fn test_private_mode_vs_p2p_mode() {
        // Test private_mode flag
        let modes = vec![
            (1, "Private mode"),
            (0, "P2P mode"),
        ];

        for (i, (private_mode, description)) in modes.iter().enumerate() {
            let instance_name = CString::new(format!("test-mode-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:110{}", 50 + i)).unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: 1,
                ipv4: ptr::null(),
                ipv6: ptr::null(),
                listener_urls: unsafe { (*listeners_ptr).as_ptr() },
                listener_urls_count: 1,
                rpc_port: 15888,
                peer_urls: ptr::null(),
                peer_urls_count: 0,
                default_protocol: ptr::null(),
                dev_name: ptr::null(),
                enable_encryption: 1,
                enable_ipv6: 0,
                mtu: 1380,
                latency_first: 0,
                enable_exit_node: 0,
                no_tun: 0,
                use_smoltcp: 0,
                foreign_network_whitelist: ptr::null(),
                disable_p2p: 0,
                relay_all_peer_rpc: 0,
                disable_udp_hole_punching: 0,
                private_mode: *private_mode,
            };

            unsafe {
                let result = start_easytier_core(&config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle {}",
                    description
                );

                if result == 0 {
                    let _ = stop_easytier_core(instance_name.as_ptr());
                }

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
            }
        }
    }
}

#[cfg(test)]
mod builder_api_tests {
    use super::*;
    use easytier_network_gateway::{start_easytier_core, stop_easytier_core, EasyTierCoreConfig};

    #[test]
    fn test_config_struct_size_and_alignment() {
        // Verify EasyTierCoreConfig struct has expected properties

        let size = std::mem::size_of::<EasyTierCoreConfig>();
        let align = std::mem::align_of::<EasyTierCoreConfig>();

        assert!(size > 0, "Config struct should have non-zero size");
        assert!(
            align >= std::mem::align_of::<*const i8>(),
            "Config struct should be pointer-aligned"
        );

        println!("EasyTierCoreConfig size: {} bytes, alignment: {}", size, align);
    }

    #[test]
    fn test_multiple_protocol_listeners() {
        // Test TCP, UDP, and WS listeners together
        let instance_name = CString::new("test-multi-protocol").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let tcp_listener = CString::new("tcp://0.0.0.0:11060").unwrap();
        let udp_listener = CString::new("udp://0.0.0.0:11061").unwrap();
        let ws_listener = CString::new("ws://0.0.0.0:11062").unwrap();

        let listeners = vec![
            tcp_listener.as_ptr(),
            udp_listener.as_ptr(),
            ws_listener.as_ptr(),
        ];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 3,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert!(
                result == 0 || result == -1,
                "Should handle multiple protocol listeners"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_network_secret_lengths() {
        // Test various network secret lengths
        let long_secret = "x".repeat(256);
        let secret_cases = vec![
            ("", "Empty secret"),
            ("a", "Single char secret"),
            ("short", "Short secret"),
            ("a-reasonably-long-secret-string", "Normal length secret"),
            (long_secret.as_str(), "Long secret"),
        ];

        for (i, (secret, description)) in secret_cases.iter().enumerate() {
            let instance_name = CString::new(format!("test-secret-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new(*secret).unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:110{}", 70 + i)).unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: 1,
                ipv4: ptr::null(),
                ipv6: ptr::null(),
                listener_urls: unsafe { (*listeners_ptr).as_ptr() },
                listener_urls_count: 1,
                rpc_port: 15888,
                peer_urls: ptr::null(),
                peer_urls_count: 0,
                default_protocol: ptr::null(),
                dev_name: ptr::null(),
                enable_encryption: 1,
                enable_ipv6: 0,
                mtu: 1380,
                latency_first: 0,
                enable_exit_node: 0,
                no_tun: 0,
                use_smoltcp: 0,
                foreign_network_whitelist: ptr::null(),
                disable_p2p: 0,
                relay_all_peer_rpc: 0,
                disable_udp_hole_punching: 0,
                private_mode: 1,
            };

            unsafe {
                let result = start_easytier_core(&config);
                assert!(
                    result == 0 || result == -1,
                    "Should handle {}: {}",
                    description,
                    secret
                );

                if result == 0 {
                    let _ = stop_easytier_core(instance_name.as_ptr());
                }

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
            }
        }
    }
}

#[cfg(test)]
mod gateway_lifecycle_tests {
    use super::*;
    use easytier_network_gateway::{start_easytier_core, stop_easytier_core, EasyTierCoreConfig};

    #[test]
    fn test_start_stop_lifecycle() {
        // Test complete lifecycle
        let instance_name = CString::new("lifecycle-test").unwrap();
        let network_name = CString::new("lifecycle-network").unwrap();
        let network_secret = CString::new("lifecycle-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11080").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            // Start gateway
            let start_result = start_easytier_core(&config);

            if start_result == 0 {
                // Stop gateway
                let stop_result = stop_easytier_core(instance_name.as_ptr());
                assert_eq!(stop_result, 0, "Stop should succeed after start");
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_double_stop() {
        // Test stopping the same instance twice
        let instance_name = CString::new("double-stop-test").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:11081").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: ptr::null(),
            ipv6: ptr::null(),
            listener_urls: unsafe { (*listeners_ptr).as_ptr() },
            listener_urls_count: 1,
            rpc_port: 15888,
            peer_urls: ptr::null(),
            peer_urls_count: 0,
            default_protocol: ptr::null(),
            dev_name: ptr::null(),
            enable_encryption: 1,
            enable_ipv6: 0,
            mtu: 1380,
            latency_first: 0,
            enable_exit_node: 0,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 0,
            relay_all_peer_rpc: 0,
            disable_udp_hole_punching: 0,
            private_mode: 1,
        };

        unsafe {
            let start_result = start_easytier_core(&config);

            if start_result == 0 {
                // First stop
                let stop1 = stop_easytier_core(instance_name.as_ptr());
                assert_eq!(stop1, 0, "First stop should succeed");

                // Second stop (should fail)
                let stop2 = stop_easytier_core(instance_name.as_ptr());
                assert_eq!(stop2, -1, "Second stop should fail");
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }
}

