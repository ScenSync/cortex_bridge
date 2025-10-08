// Copyright 2025 coScene
// SPDX-License-Identifier: MIT

//! Tests for machine_id FFI integration

use std::ffi::CString;
use uuid::Uuid;

#[cfg(test)]
mod machine_id_ffi_tests {
    use super::*;
    use easytier_bridge::CortexWebClient;

    #[test]
    fn test_ffi_struct_with_machine_id() {
        // Test that CortexWebClient struct accepts machine_id field
        let config_url = CString::new("udp://localhost:22020/test-org").unwrap();
        let machine_id = CString::new("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let client_config = CortexWebClient {
            config_server_url: config_url.as_ptr(),
            machine_id: machine_id.as_ptr(),
        };

        // Struct should be created successfully
        assert!(!client_config.config_server_url.is_null());
        assert!(!client_config.machine_id.is_null());
    }

    #[test]
    fn test_ffi_struct_without_machine_id() {
        // Test that CortexWebClient works with null machine_id
        let config_url = CString::new("udp://localhost:22020/test-org").unwrap();

        let client_config = CortexWebClient {
            config_server_url: config_url.as_ptr(),
            machine_id: std::ptr::null(),  // No machine_id provided
        };

        assert!(!client_config.config_server_url.is_null());
        assert!(client_config.machine_id.is_null());
    }

    #[test]
    fn test_uuid_parsing() {
        // Test UUID parsing with various formats
        let valid_uuids = vec![
            "550e8400-e29b-41d4-a716-446655440000",  // Lowercase
            "550E8400-E29B-41D4-A716-446655440000",  // Uppercase
            "7c9e6679-7425-40de-944b-e07fc1f90ae7",  // Different UUID
        ];

        for uuid_str in valid_uuids {
            let parsed = Uuid::parse_str(uuid_str);
            assert!(parsed.is_ok(), "Failed to parse UUID: {}", uuid_str);
        }
    }

    #[test]
    fn test_invalid_uuid_parsing() {
        // Test that invalid UUIDs are rejected
        let invalid_uuids = vec![
            "not-a-uuid",
            "12345",
            "550e8400-e29b-41d4-a716",  // Too short
            // Note: "550e8400e29b41d4a716446655440000" (no dashes) is actually valid in Uuid::parse_str
            // because the library accepts both formats
        ];

        for uuid_str in invalid_uuids {
            let parsed = Uuid::parse_str(uuid_str);
            assert!(parsed.is_err(), "Should reject invalid UUID: {}", uuid_str);
        }
    }

    // Note: Full integration test with cortex_start_web_client would require
    // network setup and is better suited for integration test suite
}

#[cfg(test)]
mod machine_id_persistence_tests {
    use super::*;

    #[test]
    fn test_uuid_serialization() {
        // Test that UUID can be converted to string for FFI
        let test_uuid = Uuid::new_v4();
        let uuid_string = test_uuid.to_string();

        // Should be 36 characters (32 hex + 4 dashes)
        assert_eq!(uuid_string.len(), 36);

        // Should be parseable back to UUID
        let reparsed = Uuid::parse_str(&uuid_string).unwrap();
        assert_eq!(test_uuid, reparsed);
    }

    #[test]
    fn test_uuid_roundtrip() {
        // Test UUID → String → CString → String → UUID roundtrip
        let original_uuid = Uuid::new_v4();
        let uuid_string = original_uuid.to_string();
        let c_string = CString::new(uuid_string.clone()).unwrap();
        let back_to_string = c_string.to_str().unwrap();
        let final_uuid = Uuid::parse_str(back_to_string).unwrap();

        assert_eq!(original_uuid, final_uuid);
    }
}

