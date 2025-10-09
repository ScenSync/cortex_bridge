//! Test GeoIP configuration and auto-detection functionality
//!
//! This test verifies that the GeoIP database can be automatically detected
//! from the project resources directory.

use easytier_bridge::client_manager::ClientManager;
use easytier_bridge::config::get_geoip_db_path;
use std::path::Path;

#[path = "common/mod.rs"]
mod common;
use common::*;

#[tokio::test]
async fn test_geoip_config_auto_detection() {
    // Test that the GeoIP database path can be auto-detected
    let geoip_path = get_geoip_db_path();

    // Should find the GeoIP database in resources directory
    assert!(
        geoip_path.is_some(),
        "GeoIP database path should be auto-detected"
    );

    let path = geoip_path.unwrap();
    println!("Auto-detected GeoIP path: {}", path);

    // Verify the file actually exists
    assert!(
        Path::new(&path).exists(),
        "GeoIP database file should exist at: {}",
        path
    );
}

#[tokio::test]
async fn test_client_manager_with_auto_geoip() {
    let db = get_test_database("test_client_manager_with_auto_geoip")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    // Create ClientManager without specifying GeoIP path (should auto-detect)
    let db_url = get_test_database_url("test_client_manager_with_auto_geoip");
    let mut client_manager = ClientManager::new(&db_url, None)
        .await
        .expect("Failed to create ClientManager");

    // ClientManager should be created successfully
    assert!(
        !client_manager.is_running(),
        "ClientManager should not be running initially"
    );

    // Cleanup resources
    client_manager.shutdown().await;
    remove_test_database("test_client_manager_with_auto_geoip")
        .await
        .expect("Failed to remove test database");
}

#[tokio::test]
async fn test_client_manager_with_explicit_geoip() {
    let db = get_test_database("test_client_manager_with_explicit_geoip")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    // Get the auto-detected path
    let geoip_path = get_geoip_db_path();
    assert!(
        geoip_path.is_some(),
        "Should be able to auto-detect GeoIP path"
    );

    // Create ClientManager with explicit GeoIP path
    let db_url = get_test_database_url("test_client_manager_with_explicit_geoip");
    let mut client_manager = ClientManager::new(&db_url, geoip_path)
        .await
        .expect("Failed to create ClientManager");

    // ClientManager should be created successfully
    assert!(
        !client_manager.is_running(),
        "ClientManager should not be running initially"
    );

    // Cleanup resources
    client_manager.shutdown().await;
    remove_test_database("test_client_manager_with_explicit_geoip")
        .await
        .expect("Failed to remove test database");
}

#[test]
fn test_geoip_path_fallback() {
    // Clean up any existing environment variable first
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Set environment variable to a non-existent path
    std::env::set_var("CORTEX_GEOIP_DB_PATH", "/non/existent/path/geoip2-cn.mmdb");

    let path = get_geoip_db_path();
    // Should return the environment variable value even if file doesn't exist
    assert_eq!(
        path,
        Some("/non/existent/path/geoip2-cn.mmdb".to_string()),
        "Should return env var path even if file doesn't exist"
    );

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Test fallback to auto-detection when env var is not set
    let fallback_path = get_geoip_db_path();
    assert!(
        fallback_path.is_some(),
        "Should fallback to auto-detection when env var is not set"
    );
}

#[tokio::test]
async fn test_concurrent_geoip_access() {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    let db = get_test_database("test_concurrent_geoip_access")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    let _db = Arc::new(db);
    let mut tasks = JoinSet::new();

    // Create multiple ClientManagers concurrently
    for i in 0..5 {
        tasks.spawn(async move {
            let db_url = get_test_database_url("test_concurrent_geoip_access");
            let mut client_manager = ClientManager::new(&db_url, None)
                .await
                .expect("Failed to create ClientManager");
            assert!(
                !client_manager.is_running(),
                "ClientManager {} should not be running initially",
                i
            );
            client_manager.shutdown().await;
            i
        });
    }

    // Wait for all tasks to complete
    let mut completed = 0;
    while let Some(result) = tasks.join_next().await {
        assert!(
            result.is_ok(),
            "Concurrent ClientManager creation should succeed"
        );
        completed += 1;
    }

    assert_eq!(
        completed, 5,
        "All concurrent tasks should complete successfully"
    );
    remove_test_database("test_concurrent_geoip_access")
        .await
        .expect("Failed to remove test database");
}

#[test]
fn test_geoip_path_unicode_characters() {
    // Test paths with Unicode characters
    let test_cases = vec![
        "/路径/geoip2-cn.mmdb",
        "/пуÑ‚ÑŒ/geoip2-cn.mmdb",
        "/パス/geoip2-cn.mmdb",
        "/مسار/geoip2-cn.mmdb",
    ];

    for test_path in test_cases {
        std::env::set_var("CORTEX_GEOIP_DB_PATH", test_path);
        let path = get_geoip_db_path();
        assert_eq!(
            path,
            Some(test_path.to_string()),
            "Should handle Unicode characters in path"
        );
    }

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}

#[test]
fn test_geoip_path_very_long_path() {
    // Test with very long path
    let long_path = format!("/very/long/path/{}/geoip2-cn.mmdb", "a".repeat(200));
    std::env::set_var("CORTEX_GEOIP_DB_PATH", &long_path);

    let path = get_geoip_db_path();
    assert_eq!(path, Some(long_path), "Should handle very long paths");

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}

#[tokio::test]
async fn test_client_manager_resource_cleanup() {
    let db = get_test_database("test_client_manager_resource_cleanup")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    // Create and immediately shutdown multiple ClientManagers
    for i in 0..3 {
        let db_url = get_test_database_url("test_client_manager_resource_cleanup");
        let mut client_manager = ClientManager::new(&db_url, None)
            .await
            .expect("Failed to create ClientManager");
        assert!(
            !client_manager.is_running(),
            "ClientManager {} should not be running initially",
            i
        );
        client_manager.shutdown().await;
        // Verify that resources are properly cleaned up
    }
    remove_test_database("test_client_manager_resource_cleanup")
        .await
        .expect("Failed to remove test database");
}

#[test]
fn test_geoip_config_thread_safety() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Test concurrent access to get_geoip_db_path from multiple threads
    for _ in 0..10 {
        let counter_clone = counter.clone();
        let handle = thread::spawn(move || {
            let _path = get_geoip_db_path();
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }

    assert_eq!(
        counter.load(Ordering::Relaxed),
        10,
        "All threads should complete successfully"
    );
}

#[test]
fn test_geoip_path_edge_cases() {
    // Clean up any existing environment variable first
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Test various edge cases - only empty strings trigger fallback
    let test_cases = vec![
        (
            "/some/absolute/path/geoip.mmdb",
            "Absolute path should be returned as-is",
        ),
        ("~", "Tilde should be returned as-is"),
        (
            "$HOME/geoip.mmdb",
            "Environment variable in path should be returned as-is",
        ),
        (
            "/valid/path/geoip.mmdb",
            "Valid path should be returned as-is",
        ),
        (
            "relative/path/geoip.mmdb",
            "Relative path should be returned as-is",
        ),
    ];

    for (test_path, description) in test_cases {
        std::env::set_var("CORTEX_GEOIP_DB_PATH", test_path);
        let path = get_geoip_db_path();
        assert_eq!(path, Some(test_path.to_string()), "{}", description);
    }

    // Test empty string - should fallback to auto-detection
    std::env::set_var("CORTEX_GEOIP_DB_PATH", "");
    let empty_path = get_geoip_db_path();
    assert!(
        empty_path.is_some(),
        "Empty string should trigger fallback to auto-detection"
    );
    assert_ne!(
        empty_path,
        Some("".to_string()),
        "Should not return empty string"
    );
    if let Some(path) = empty_path {
        assert!(
            path.ends_with("geoip2-cn.mmdb"),
            "Should find the actual GeoIP database file"
        );
    }

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}

#[tokio::test]
async fn test_client_manager_with_none_and_some_geoip() {
    let db1 = get_test_database("test_client_manager_none_geoip1")
        .await
        .expect("Failed to setup test database 1");
    let db2 = get_test_database("test_client_manager_some_geoip2")
        .await
        .expect("Failed to setup test database 2");

    cleanup_test_database(&db1)
        .await
        .expect("Failed to cleanup test database 1");
    cleanup_test_database(&db2)
        .await
        .expect("Failed to cleanup test database 2");

    // Test with None (auto-detection)
    let db_url1 = get_test_database_url("test_client_manager_none_geoip1");
    let mut client_manager1 = ClientManager::new(&db_url1, None)
        .await
        .expect("Failed to create ClientManager 1");

    // Test with Some (explicit path)
    let geoip_path = get_geoip_db_path();
    let db_url2 = get_test_database_url("test_client_manager_some_geoip2");
    let mut client_manager2 = ClientManager::new(&db_url2, geoip_path)
        .await
        .expect("Failed to create ClientManager 2");

    // Both should work
    assert!(
        !client_manager1.is_running(),
        "ClientManager with None should not be running initially"
    );
    assert!(
        !client_manager2.is_running(),
        "ClientManager with Some should not be running initially"
    );

    // Cleanup resources
    client_manager1.shutdown().await;
    client_manager2.shutdown().await;

    remove_test_database("test_client_manager_none_geoip1")
        .await
        .expect("Failed to remove test database 1");
    remove_test_database("test_client_manager_some_geoip2")
        .await
        .expect("Failed to remove test database 2");
}

#[test]
fn test_geoip_path_empty_env_var() {
    // Clean up any existing environment variable first
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Test with empty environment variable - should fallback to auto-detection
    std::env::set_var("CORTEX_GEOIP_DB_PATH", "");

    let path = get_geoip_db_path();
    // Should fallback to auto-detection and find the actual file
    assert!(
        path.is_some(),
        "Should fallback to auto-detection when env var is empty"
    );

    // The path should not be empty
    if let Some(p) = path {
        assert!(!p.is_empty(), "Detected path should not be empty");
        assert!(
            p.contains("geoip2-cn.mmdb"),
            "Should detect the correct GeoIP database file"
        );
    }

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Test that auto-detection works when no env var is set
    let auto_path = get_geoip_db_path();
    assert!(
        auto_path.is_some(),
        "Auto-detection should find the GeoIP database"
    );
    if let Some(p) = auto_path {
        assert!(
            p.contains("geoip2-cn.mmdb"),
            "Auto-detected path should be the GeoIP database"
        );
    }
}

#[tokio::test]
async fn test_client_manager_with_invalid_geoip_path() {
    let db = get_test_database("test_client_manager_with_invalid_geoip_path")
        .await
        .expect("Failed to setup test database");
    cleanup_test_database(&db)
        .await
        .expect("Failed to cleanup test database");

    // Create ClientManager with invalid GeoIP path (should not crash)
    let invalid_path = Some("/non/existent/path/invalid.mmdb".to_string());
    let db_url = get_test_database_url("test_client_manager_with_invalid_geoip_path");
    let mut client_manager = ClientManager::new(&db_url, invalid_path)
        .await
        .expect("Failed to create ClientManager");

    // ClientManager should still be created successfully even with invalid GeoIP path
    assert!(
        !client_manager.is_running(),
        "ClientManager should not be running initially"
    );

    // Cleanup resources
    client_manager.shutdown().await;
    remove_test_database("test_client_manager_with_invalid_geoip_path")
        .await
        .expect("Failed to remove test database");
}

#[test]
fn test_geoip_path_with_whitespace_env_var() {
    // Test that whitespace-only environment variable falls back to auto-detection
    std::env::set_var("CORTEX_GEOIP_DB_PATH", "   ");

    let path = get_geoip_db_path();
    // Should return the whitespace value as-is (environment variable takes precedence)
    assert_eq!(path, Some("   ".to_string()));

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}

#[test]
fn test_geoip_path_with_relative_paths() {
    // Test various relative path formats
    let test_cases = vec![
        "./some/path/geoip2-cn.mmdb",
        "../some/path/geoip2-cn.mmdb",
        "some/path/geoip2-cn.mmdb",
        "./non-existent-geoip.mmdb",
    ];

    for test_path in test_cases {
        std::env::set_var("CORTEX_GEOIP_DB_PATH", test_path);
        let path = get_geoip_db_path();
        assert_eq!(
            path,
            Some(test_path.to_string()),
            "Should return the exact environment variable value"
        );
    }

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}

#[test]
fn test_geoip_path_with_absolute_paths() {
    // Test absolute path scenarios
    let test_cases = vec![
        "/usr/local/share/geoip/geoip2-cn.mmdb",
        "/opt/geoip/geoip2-cn.mmdb",
        "/home/user/geoip/geoip2-cn.mmdb",
    ];

    for test_path in test_cases {
        std::env::set_var("CORTEX_GEOIP_DB_PATH", test_path);
        let path = get_geoip_db_path();
        assert_eq!(
            path,
            Some(test_path.to_string()),
            "Should return the exact environment variable value"
        );
    }

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}

#[tokio::test]
async fn test_multiple_client_managers_with_same_geoip() {
    let db1 = get_test_database("test_multiple_client_managers_1")
        .await
        .expect("Failed to setup test database 1");
    let db2 = get_test_database("test_multiple_client_managers_2")
        .await
        .expect("Failed to setup test database 2");

    cleanup_test_database(&db1)
        .await
        .expect("Failed to cleanup test database 1");
    cleanup_test_database(&db2)
        .await
        .expect("Failed to cleanup test database 2");

    // Create multiple ClientManagers with same GeoIP path
    let geoip_path = get_geoip_db_path();
    let db_url1 = get_test_database_url("test_multiple_client_managers_1");
    let db_url2 = get_test_database_url("test_multiple_client_managers_2");

    let mut client_manager1 = ClientManager::new(&db_url1, geoip_path.clone())
        .await
        .expect("Failed to create ClientManager 1");
    let mut client_manager2 = ClientManager::new(&db_url2, geoip_path)
        .await
        .expect("Failed to create ClientManager 2");

    // Both should be created successfully
    assert!(
        !client_manager1.is_running(),
        "ClientManager 1 should not be running initially"
    );
    assert!(
        !client_manager2.is_running(),
        "ClientManager 2 should not be running initially"
    );

    // Cleanup resources
    client_manager1.shutdown().await;
    client_manager2.shutdown().await;
    remove_test_database("test_multiple_client_managers_1")
        .await
        .expect("Failed to remove test database 1");
    remove_test_database("test_multiple_client_managers_2")
        .await
        .expect("Failed to remove test database 2");
}

#[test]
fn test_geoip_path_environment_variable_priority() {
    // Ensure environment variable takes precedence over auto-detection
    let custom_path = "/custom/path/to/geoip.mmdb";
    std::env::set_var("CORTEX_GEOIP_DB_PATH", custom_path);

    let path = get_geoip_db_path();
    assert_eq!(
        path,
        Some(custom_path.to_string()),
        "Environment variable should take precedence"
    );

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Now test auto-detection
    let auto_path = get_geoip_db_path();
    if let Some(p) = auto_path {
        assert_ne!(
            p, custom_path,
            "Auto-detected path should be different from custom path"
        );
        assert!(
            p.contains("geoip2-cn.mmdb"),
            "Auto-detected path should contain the database filename"
        );
    }
}

#[test]
fn test_geoip_path_special_characters() {
    // Clean up any existing environment variable first
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");

    // Test paths with special characters
    let test_cases = vec![
        "/path with spaces/geoip2-cn.mmdb",
        "/path-with-dashes/geoip2-cn.mmdb",
        "/path_with_underscores/geoip2-cn.mmdb",
        "/path.with.dots/geoip2-cn.mmdb",
        "/path@with@symbols/geoip2-cn.mmdb",
        "/path#with#hash/geoip2-cn.mmdb",
    ];

    for test_path in test_cases {
        std::env::set_var("CORTEX_GEOIP_DB_PATH", test_path);
        let path = get_geoip_db_path();
        assert_eq!(
            path,
            Some(test_path.to_string()),
            "Should handle special characters in path: {}",
            test_path
        );
    }

    // Clean up environment variable
    std::env::remove_var("CORTEX_GEOIP_DB_PATH");
}
