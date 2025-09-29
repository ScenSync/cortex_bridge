//! Configuration module for cortex-easytier-web
//!
//! This module provides global configuration settings, including timezone and GeoIP database configuration.

use chrono::FixedOffset;
use once_cell::sync::Lazy;
use std::{env, path::PathBuf};

/// Default database URL for MySQL connection (UTC-First approach)
const DEFAULT_DATABASE_URL: &str = "mysql://root:root@localhost:3306/cortex";

/// Default timezone offset for Asia/Shanghai (+8 hours)
const DEFAULT_TIMEZONE_OFFSET_HOURS: i32 = 8;

/// Default GeoIP database path in project resources
const DEFAULT_GEOIP_DB_PATH: &str = "./resources/geoip2-cn.mmdb";

/// Global timezone configuration
///
/// This can be configured via environment variable CORTEX_TIMEZONE_OFFSET_HOURS
/// Default is +8 (Asia/Shanghai)
pub static TIMEZONE: Lazy<FixedOffset> = Lazy::new(|| {
    let offset_hours = env::var("CORTEX_TIMEZONE_OFFSET_HOURS")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_TIMEZONE_OFFSET_HOURS);

    FixedOffset::east_opt(offset_hours * 3600)
        .unwrap_or_else(|| FixedOffset::east_opt(DEFAULT_TIMEZONE_OFFSET_HOURS * 3600).unwrap())
});

/// Get the configured timezone
pub fn get_timezone() -> FixedOffset {
    *TIMEZONE
}

/// Get the GeoIP database path
///
/// This can be configured via environment variable CORTEX_GEOIP_DB_PATH
/// Default path points to the project's resources directory
pub fn get_geoip_db_path() -> Option<String> {
    // First check environment variable
    if let Ok(path) = env::var("CORTEX_GEOIP_DB_PATH") {
        if !path.is_empty() {
            return Some(path);
        }
    }

    // Try to resolve the default path relative to the current working directory
    let default_path = PathBuf::from(DEFAULT_GEOIP_DB_PATH);

    // Check if the default path exists
    if default_path.exists() {
        return default_path.to_str().map(|s| s.to_string());
    }

    // Try alternative paths for different execution contexts
    let alternative_paths = [
        "../resources/geoip2-cn.mmdb",                 // From target directory
        "../../resources/geoip2-cn.mmdb",              // From nested directories
        "./easytier-bridge/resources/geoip2-cn.mmdb",  // From cortex-core root
        "../easytier-bridge/resources/geoip2-cn.mmdb", // From cortex-core subdirectory
        "../../easytier-bridge/resources/geoip2-cn.mmdb", // From deeper nested directories
    ];

    for alt_path in &alternative_paths {
        let path = PathBuf::from(alt_path);
        if path.exists() {
            return path.to_str().map(|s| s.to_string());
        }
    }

    // If no path found, return None to disable GeoIP
    None
}

/// Convert UTC timestamp to configured timezone
pub fn utc_to_local_timezone(
    utc_timestamp: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<FixedOffset> {
    utc_timestamp.with_timezone(&get_timezone())
}

/// Get current time in configured timezone
pub fn now_in_timezone() -> chrono::DateTime<FixedOffset> {
    chrono::Utc::now().with_timezone(&get_timezone())
}

/// Get the database URL for MySQL connection
///
/// This can be configured via environment variable CORTEX_DATABASE_URL
/// Default is mysql://root:root@localhost:3306/cortex
pub fn get_database_url() -> Option<String> {
    // First check environment variable
    if let Ok(url) = env::var("CORTEX_DATABASE_URL") {
        if !url.is_empty() {
            return Some(url);
        }
    }

    // Return default URL
    Some(DEFAULT_DATABASE_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::env;

    #[test]
    fn test_default_timezone() {
        let tz = get_timezone();
        // Get expected offset from environment or default
        let expected_offset_hours = env::var("CORTEX_TIMEZONE_OFFSET_HOURS")
            .ok()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_TIMEZONE_OFFSET_HOURS);

        assert_eq!(tz.local_minus_utc(), expected_offset_hours * 3600);
    }

    #[test]
    fn test_utc_to_local_conversion() {
        let utc_time = Utc::now();
        let local_time = utc_to_local_timezone(utc_time);

        // Get expected offset from environment or default
        let expected_offset_hours = env::var("CORTEX_TIMEZONE_OFFSET_HOURS")
            .ok()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_TIMEZONE_OFFSET_HOURS);
        let expected_offset = expected_offset_hours * 3600;

        assert_eq!(local_time.offset().local_minus_utc(), expected_offset);
    }

    #[test]
    fn test_now_in_timezone() {
        let local_now = now_in_timezone();

        // Get expected offset from environment or default
        let expected_offset_hours = env::var("CORTEX_TIMEZONE_OFFSET_HOURS")
            .ok()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_TIMEZONE_OFFSET_HOURS);

        assert_eq!(
            local_now.offset().local_minus_utc(),
            expected_offset_hours * 3600
        );
    }

    #[test]
    fn test_timezone_configuration() {
        // Test that timezone can be configured via environment variable
        // This test verifies the configuration mechanism works
        let tz = get_timezone();

        // The timezone should be valid (between -12 and +14 hours)
        let offset_seconds = tz.local_minus_utc();
        let offset_hours = offset_seconds / 3600;

        assert!(
            (-12..=14).contains(&offset_hours),
            "Timezone offset {} hours is out of valid range [-12, +14]",
            offset_hours
        );
    }
}
