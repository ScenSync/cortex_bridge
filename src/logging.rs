//! Logging module for cortex-easytier-core
//!
//! This module provides logging initialization and panic recovery functionality
//! specifically for the cortex-easytier-core module.

use std::fs;
use std::path::Path;
use std::sync::Once;
use tracing::{debug, info};
use tracing_subscriber::{
    fmt, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

/// Configuration for logging setup
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_level: String,
    pub module_name: String,
}

impl LoggingConfig {
    pub fn new(level: &str, module_name: &str) -> Self {
        Self {
            log_level: level.to_string(),
            module_name: module_name.to_string(),
        }
    }
}

// Static variables for ensuring single initialization
static CONSOLE_INIT: Once = Once::new();
static FILE_INIT: Once = Once::new();
static PANIC_HOOK_INIT: Once = Once::new();

// Guards for non-blocking writers
static FILE_GUARD: once_cell::sync::Lazy<
    std::sync::Mutex<Option<tracing_appender::non_blocking::WorkerGuard>>,
> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));
static CONSOLE_GUARD: once_cell::sync::Lazy<
    std::sync::Mutex<Option<tracing_appender::non_blocking::WorkerGuard>>,
> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

// Error storage for initialization failures
static INIT_ERROR: once_cell::sync::Lazy<std::sync::Mutex<Option<String>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

// Last panic message storage
static LAST_PANIC: once_cell::sync::Lazy<std::sync::Mutex<Option<String>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

/// Initialize console logging with environment variable support
pub fn init_console_logging(config: &LoggingConfig) {
    CONSOLE_INIT.call_once(|| {
        let result = std::panic::catch_unwind(|| {
            // Create environment filter that prioritizes the explicitly provided log level
            // for the specific module, while still allowing other environment variables
            let env_filter = EnvFilter::try_from_default_env()
                .map(|mut filter| {
                    // Add our specific module level to override any existing setting
                    filter = filter.add_directive(
                        format!("{}={}", config.module_name, config.log_level)
                            .parse()
                            .unwrap_or_else(|_| "info".parse().unwrap()),
                    );
                    // Add all submodules to the filter
                    let submodules = [
                        "logging",
                        "stun_wrapper",
                        "easytier_web_client",
                        "easytier_core_ffi",
                        "client_manager",
                        "db",
                        "config",
                        "config_srv",
                        "network_config_srv_ffi",
                    ];
                    for submodule in &submodules {
                        filter = filter.add_directive(
                            format!("{}::{}={}", config.module_name, submodule, config.log_level)
                                .parse()
                                .unwrap_or_else(|_| "debug".parse().unwrap()),
                        );
                    }
                    filter
                })
                .unwrap_or_else(|_| {
                    // Create filter with module and all submodules
                    let submodules = [
                        "logging",
                        "stun_wrapper",
                        "easytier_web_client",
                        "easytier_core_ffi",
                        "client_manager",
                        "db",
                        "config",
                        "config_srv",
                        "network_config_srv_ffi",
                    ];
                    let mut filter_str = format!("{}={}", config.module_name, config.log_level);
                    for submodule in &submodules {
                        filter_str.push_str(&format!(
                            ",{}::{}={}",
                            config.module_name, submodule, config.log_level
                        ));
                    }
                    EnvFilter::new(filter_str)
                });

            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_timer(LocalTime::rfc_3339()),
                )
                .init();

            debug!(
                "Console logging initialized for {} with environment filter support (default: {})",
                config.module_name, config.log_level
            );

            info!(
                "[RUST] Console logging successfully initialized for module: {}",
                config.module_name
            );
        });

        if let Err(e) = result {
            let error_msg = format!("Failed to initialize console logging: {:?}", e);
            *INIT_ERROR.lock().unwrap() = Some(error_msg);
        }

        // Always initialize panic hook when logging is initialized
        init_panic_recovery();
    });
}

/// Initialize logging with both file and console output
pub fn init_file_logging(
    config: &LoggingConfig,
    log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut init_result = Ok(());

    FILE_INIT.call_once(|| {
        let result = std::panic::catch_unwind(|| -> Result<(), Box<dyn std::error::Error>> {
            // Extract directory and filename from log_path
            let path = Path::new(log_path);
            let log_dir = path.parent()
                .ok_or("Invalid log path: no parent directory")?;
            let log_filename = path.file_name()
                .ok_or("Invalid log path: no filename")?;

            // Create log directory if it doesn't exist
            fs::create_dir_all(log_dir)?;

            use tracing_appender::non_blocking;

            // Create environment filter that prioritizes the explicitly provided log level
            // for the specific module, while still allowing other environment variables
            let env_filter = EnvFilter::try_from_default_env()
                .map(|mut filter| {
                    // Add our specific module level to override any existing setting
                    filter = filter.add_directive(
                        format!("{}={}", config.module_name, config.log_level)
                            .parse()
                            .unwrap_or_else(|_| "info".parse().unwrap())
                    );
                    // Add all submodules to the filter
                    let submodules = [
                        "logging", "stun_wrapper", "easytier_web_client", "easytier_core_ffi",
                        "client_manager", "db", "config", "config_srv", "network_config_srv_ffi"
                    ];
                    for submodule in &submodules {
                        filter = filter.add_directive(
                            format!("{}::{}={}", config.module_name, submodule, config.log_level)
                                .parse()
                                .unwrap_or_else(|_| "debug".parse().unwrap())
                        );
                    }
                    filter
                })
                .unwrap_or_else(|_| {
                    // Create filter with module and all submodules
                    let submodules = [
                        "logging", "stun_wrapper", "easytier_web_client", "easytier_core_ffi",
                        "client_manager", "db", "config", "config_srv", "network_config_srv_ffi"
                    ];
                    let mut filter_str = format!("{}={}", config.module_name, config.log_level);
                    for submodule in &submodules {
                        filter_str.push_str(&format!(",{}::{}={}", config.module_name, submodule, config.log_level));
                    }
                    EnvFilter::new(filter_str)
                });

            // Create simple file appender without rotation (to match Go side)
            let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
            let (file_writer, file_guard) = non_blocking(file_appender);

            // Store the guard to prevent it from being dropped
            *FILE_GUARD.lock().unwrap() = Some(file_guard);

            // Create console writer
            let (console_writer, console_guard) = non_blocking(std::io::stdout());
            *CONSOLE_GUARD.lock().unwrap() = Some(console_guard);

            // Initialize tracing subscriber with both file and console output
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_writer(file_writer)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_ansi(false) // No ANSI colors in file
                        .with_timer(LocalTime::rfc_3339()),
                )
                .with(
                    fmt::layer()
                        .with_writer(console_writer)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_ansi(true) // ANSI colors for console
                        .with_timer(LocalTime::rfc_3339()),
                )
                .init();

            debug!(
                "File logging initialized for {} in directory: {} (default level: {}, no rotation)",
                config.module_name, log_dir.display(), config.log_level
            );

            info!("[RUST] File logging successfully initialized for module: {} in directory: {} with level: {} (no rotation)t ", config.module_name, log_dir.display(), config.log_level);

            Ok(())
        });

        match result {
            Ok(Ok(())) => {},
            Ok(Err(e)) => {
                init_result = Err(e);
            },
            Err(e) => {
                let error_msg = format!("Panic during file logging initialization: {:?}", e);
                init_result = Err(error_msg.into());
            }
        }

        // Always initialize panic hook when logging is initialized
        init_panic_recovery();
    });

    init_result
}

/// Set configuration and initialize console logging in one call
pub fn set_and_init_console_logging(level: &str, module_name: &str) {
    let config = LoggingConfig::new(level, module_name);
    init_console_logging(&config);
}

/// Set configuration and initialize file logging in one call
pub fn set_and_init_file_logging(
    level: &str,
    module_name: &str,
    log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LoggingConfig::new(level, module_name);
    init_file_logging(&config, log_path)
}

/// Initialize panic recovery hook
/// This function sets up a custom panic hook that logs panic information
/// and stores the last panic message for retrieval via FFI
pub fn init_panic_recovery() {
    PANIC_HOOK_INIT.call_once(|| {
        std::panic::set_hook(Box::new(|panic_info| {
            let panic_msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let location = if let Some(location) = panic_info.location() {
                format!(
                    " at {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            } else {
                " at unknown location".to_string()
            };

            let full_msg = format!("{}{}", panic_msg, location);

            // Store the panic message
            *LAST_PANIC.lock().unwrap() = Some(full_msg.clone());

            // Log the panic
            tracing::error!("PANIC RECOVERED: {}", panic_msg);

            // Also print to stderr as a fallback
            eprintln!("[CORTEX-EASYTIER-CORE PANIC] {}", full_msg);
        }));

        info!("[RUST] Cortex EasyTier Core panic recovery hook initialized");
    });
}

/// Get the last panic message
/// Returns None if no panic has occurred since the last call to clear_last_panic
pub fn get_last_panic() -> Option<String> {
    LAST_PANIC.lock().unwrap().clone()
}

/// Clear the stored panic message
pub fn clear_last_panic() {
    *LAST_PANIC.lock().unwrap() = None;
}

/// This is automatically called when logging is initialized,
/// but can also be called manually if needed
pub fn ensure_panic_recovery() {
    init_panic_recovery();
}

// Local logging macros that use the properly initialized tracing system
// These should be used instead of global tracing macros to avoid conflicts

/// Debug logging macro
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*)
    };
}

/// Info logging macro
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
}

/// Warning logging macro
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*)
    };
}

/// Error logging macro
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*)
    };
}

/// Trace logging macro
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*)
    };
}
