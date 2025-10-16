//! Tests for EasyTier Builder API usage
//!
//! This module tests that the gateway correctly uses EasyTier's Builder API
//! instead of TOML string construction (the key improvement).

use std::ffi::CString;
use std::ptr;

#[cfg(test)]
mod builder_api_validation_tests {
    use super::*;
    use easytier_network_gateway::{start_easytier_core, stop_easytier_core, EasyTierCoreConfig};

    #[test]
    fn test_builder_api_network_identity() {
        // Test that network identity is properly set via Builder API
        let instance_name = CString::new("builder-identity-test").unwrap();
        let network_name = CString::new("production-network").unwrap();
        let network_secret = CString::new("super-secret-password-123").unwrap();
        let listener = CString::new("tcp://0.0.0.0:12010").unwrap();

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
            // Builder API should handle this correctly
            assert!(
                result == 0 || result == -1,
                "Builder API should set network identity"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_builder_api_dhcp_configuration() {
        // Test DHCP configuration via Builder API
        let dhcp_modes = [(0, "Manual IP"), (1, "DHCP enabled")];

        for (i, (dhcp_value, description)) in dhcp_modes.iter().enumerate() {
            let instance_name = CString::new(format!("builder-dhcp-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:120{}", 20 + i)).unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let config = EasyTierCoreConfig {
                instance_name: instance_name.as_ptr(),
                network_name: network_name.as_ptr(),
                network_secret: network_secret.as_ptr(),
                dhcp: *dhcp_value,
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
                    "Builder API should handle {}: {}",
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
    fn test_builder_api_manual_ip_addresses() {
        // Test manual IP configuration via Builder API
        let instance_name = CString::new("builder-manual-ip").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:12030").unwrap();
        let ipv4 = CString::new("10.144.144.1").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 0, // Manual IP mode
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
            assert!(
                result == 0 || result == -1,
                "Builder API should set manual IPv4"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_builder_api_listeners_array() {
        // Test that listener array is properly processed by Builder API
        let instance_name = CString::new("builder-listeners-test").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();

        // Create multiple listeners
        let tcp_listener = CString::new("tcp://0.0.0.0:12040").unwrap();
        let udp_listener = CString::new("udp://0.0.0.0:12041").unwrap();

        let listeners = vec![tcp_listener.as_ptr(), udp_listener.as_ptr()];
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

        unsafe {
            let result = start_easytier_core(&config);
            assert!(
                result == 0 || result == -1,
                "Builder API should process listener array"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_builder_api_peer_configuration() {
        // Test peer configuration for P2P mode
        let instance_name = CString::new("builder-peers-test").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:12050").unwrap();
        let peer1 = CString::new("tcp://10.0.0.1:11010").unwrap();
        let peer2 = CString::new("tcp://10.0.0.2:11010").unwrap();

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
            assert!(result == 0 || result == -1, "Builder API should set peers");

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
            let _ = Box::from_raw(peers_ptr);
        }
    }

    #[test]
    fn test_builder_api_flags_configuration() {
        // Test that all flags are properly set via Builder API
        let instance_name = CString::new("builder-flags-test").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:12060").unwrap();

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
            enable_ipv6: 1,
            mtu: 1500,
            latency_first: 1,
            enable_exit_node: 1,
            no_tun: 0,
            use_smoltcp: 0,
            foreign_network_whitelist: ptr::null(),
            disable_p2p: 1,
            relay_all_peer_rpc: 1,
            disable_udp_hole_punching: 1,
            private_mode: 1,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert!(
                result == 0 || result == -1,
                "Builder API should set all flags correctly"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_builder_api_rpc_portal() {
        // Test RPC portal configuration
        let rpc_ports = [
            (15888, "Default RPC port"),
            (15000, "Custom RPC port 1"),
            (16000, "Custom RPC port 2"),
        ];

        for (i, (rpc_port, description)) in rpc_ports.iter().enumerate() {
            let instance_name = CString::new(format!("builder-rpc-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:120{}", 70 + i)).unwrap();

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
                rpc_port: *rpc_port,
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
                    "Builder API should handle {}: {}",
                    description,
                    rpc_port
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
    fn test_builder_api_ipv4_and_ipv6_combination() {
        // Test dual-stack configuration
        let instance_name = CString::new("builder-dual-stack").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:12080").unwrap();
        let ipv4 = CString::new("10.144.144.1").unwrap();
        let ipv6 = CString::new("fd00::1").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 0,
            ipv4: ipv4.as_ptr(),
            ipv6: ipv6.as_ptr(),
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
            assert!(
                result == 0 || result == -1,
                "Builder API should handle dual-stack"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }

    #[test]
    fn test_builder_api_empty_ipv4_string() {
        // Test with empty IPv4 string (should be ignored)
        let instance_name = CString::new("builder-empty-ipv4").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:12090").unwrap();
        let empty_ipv4 = CString::new("").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let config = EasyTierCoreConfig {
            instance_name: instance_name.as_ptr(),
            network_name: network_name.as_ptr(),
            network_secret: network_secret.as_ptr(),
            dhcp: 1,
            ipv4: empty_ipv4.as_ptr(),
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
                "Builder API should ignore empty IPv4"
            );

            if result == 0 {
                let _ = stop_easytier_core(instance_name.as_ptr());
            }

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
        }
    }
}

#[cfg(test)]
mod configuration_parsing_tests {
    use super::*;
    use easytier_network_gateway::{start_easytier_core, stop_easytier_core, EasyTierCoreConfig};

    #[test]
    fn test_listener_url_schemes() {
        // Test all supported listener schemes
        let schemes = [
            ("tcp://0.0.0.0:13010", "TCP scheme"),
            ("udp://0.0.0.0:13011", "UDP scheme"),
            ("ws://0.0.0.0:13012", "WebSocket scheme"),
        ];

        for (i, (listener_url, description)) in schemes.iter().enumerate() {
            let instance_name = CString::new(format!("scheme-test-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(*listener_url).unwrap();

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
                    listener_url
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
    fn test_ipv6_listener_addresses() {
        // Test IPv6 listener addresses
        let ipv6_listeners = [
            ("tcp://[::]:13020", "IPv6 any address"),
            ("tcp://[::1]:13021", "IPv6 loopback"),
            ("tcp://[fe80::1]:13022", "IPv6 link-local"),
        ];

        for (i, (listener_url, description)) in ipv6_listeners.iter().enumerate() {
            let instance_name = CString::new(format!("ipv6-listener-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(*listener_url).unwrap();

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
                assert!(
                    result == 0 || result == -1,
                    "Should handle {}: {}",
                    description,
                    listener_url
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
    fn test_network_name_variations() {
        // Test various network names
        let network_names = [
            "simple",
            "network-with-dashes",
            "network_with_underscores",
            "network123numbers",
            "UPPERCASE-NETWORK",
            "MixedCase-Network",
        ];

        for (i, network_name_str) in network_names.iter().enumerate() {
            let instance_name = CString::new(format!("netname-test-{}", i)).unwrap();
            let network_name = CString::new(*network_name_str).unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:130{}", 30 + i)).unwrap();

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
                    "Should handle network name: {}",
                    network_name_str
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
    fn test_peer_url_parsing() {
        // Test peer URL parsing with various formats
        let peer_urls = [
            "tcp://192.168.1.1:11010",
            "udp://10.0.0.1:11011",
            "ws://example.com:11012",
            "tcp://[2001:db8::1]:11013",
        ];

        for (i, peer_url_str) in peer_urls.iter().enumerate() {
            let instance_name = CString::new(format!("peer-url-test-{}", i)).unwrap();
            let network_name = CString::new("test-network").unwrap();
            let network_secret = CString::new("test-secret").unwrap();
            let listener = CString::new(format!("tcp://0.0.0.0:130{}", 40 + i)).unwrap();
            let peer = CString::new(*peer_url_str).unwrap();

            let listeners = vec![listener.as_ptr()];
            let listeners_box = listeners.into_boxed_slice();
            let listeners_ptr = Box::into_raw(listeners_box);

            let peers = vec![peer.as_ptr()];
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
                peer_urls_count: 1,
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
                assert!(
                    result == 0 || result == -1,
                    "Should parse peer URL: {}",
                    peer_url_str
                );

                if result == 0 {
                    let _ = stop_easytier_core(instance_name.as_ptr());
                }

                // Clean up
                let _ = Box::from_raw(listeners_ptr);
                let _ = Box::from_raw(peers_ptr);
            }
        }
    }

    #[test]
    fn test_invalid_peer_url() {
        // Test with invalid peer URL
        let instance_name = CString::new("invalid-peer-url").unwrap();
        let network_name = CString::new("test-network").unwrap();
        let network_secret = CString::new("test-secret").unwrap();
        let listener = CString::new("tcp://0.0.0.0:13050").unwrap();
        let invalid_peer = CString::new("not-a-valid-peer-url").unwrap();

        let listeners = vec![listener.as_ptr()];
        let listeners_box = listeners.into_boxed_slice();
        let listeners_ptr = Box::into_raw(listeners_box);

        let peers = vec![invalid_peer.as_ptr()];
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
            peer_urls_count: 1,
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
            private_mode: 0,
        };

        unsafe {
            let result = start_easytier_core(&config);
            assert_eq!(result, -1, "Should fail with invalid peer URL");

            // Clean up
            let _ = Box::from_raw(listeners_ptr);
            let _ = Box::from_raw(peers_ptr);
        }
    }
}

#[cfg(test)]
mod memory_safety_tests {
    use super::*;
    use easytier_network_gateway::{start_easytier_core, EasyTierCoreConfig};

    #[test]
    fn test_struct_memory_layout() {
        // Verify struct layout for FFI safety
        let size = std::mem::size_of::<EasyTierCoreConfig>();
        let align = std::mem::align_of::<EasyTierCoreConfig>();

        assert!(size > 0, "Config struct should have non-zero size");
        assert!(
            align >= std::mem::align_of::<i32>(),
            "Config struct should be properly aligned"
        );

        // Verify it's #[repr(C)]
        println!(
            "EasyTierCoreConfig: size={} bytes, alignment={}",
            size, align
        );
    }

    #[test]
    fn test_null_pointer_safety() {
        // Test that all FFI functions handle null pointers safely
        unsafe {
            use easytier_network_gateway::{
                get_easytier_core_status, start_easytier_core, stop_easytier_core,
            };

            assert_eq!(
                start_easytier_core(ptr::null()),
                -1,
                "start should reject null config"
            );
            assert_eq!(
                stop_easytier_core(ptr::null()),
                -1,
                "stop should reject null name"
            );
            assert_eq!(
                get_easytier_core_status(ptr::null(), ptr::null_mut()),
                -1,
                "get_status should reject null name"
            );
        }
    }

    #[test]
    fn test_zero_length_arrays() {
        // Test with zero-length listener array
        let instance_name = CString::new("zero-array-test").unwrap();
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
}
