//! Logging module for EasyTier integration crates
//!
//! This module provides logging initialization and panic recovery functionality
//! shared across all EasyTier integration crates.

use std::fs;
use std::path::Path;
use std::sync::Once;
use tracing::{debug, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

// Last panic message storage
static LAST_PANIC: once_cell::sync::Lazy<std::sync::Mutex<Option<String>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(None));

/// Initialize console logging with environment variable support
pub fn init_console_logging(config: &LoggingConfig) {
    CONSOLE_INIT.call_once(|| {
        let result = std::panic::catch_unwind(|| {
            let env_filter = create_env_filter(config);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_target(true).with_thread_ids(true))
                .init();

            debug!(
                "Console logging initialized for {} (level: {})",
                config.module_name, config.log_level
            );

            info!(
                "[RUST] Console logging initialized for module: {}",
                config.module_name
            );
        });

        if result.is_err() {
            eprintln!("[EASYTIER_COMMON] Failed to initialize console logging");
        }

        // Always initialize panic hook
        init_panic_recovery();
    });
}

/// Initialize file logging with both file and console output
pub fn init_file_logging(
    config: &LoggingConfig,
    log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut init_result = Ok(());

    FILE_INIT.call_once(|| {
        let result = std::panic::catch_unwind(|| -> Result<(), Box<dyn std::error::Error>> {
            // Extract directory and filename from log_path
            let path = Path::new(log_path);
            let log_dir = path
                .parent()
                .ok_or("Invalid log path: no parent directory")?;
            let log_filename = path.file_name().ok_or("Invalid log path: no filename")?;

            // Create log directory if it doesn't exist
            fs::create_dir_all(log_dir)?;

            use tracing_appender::non_blocking;

            let env_filter = create_env_filter(config);

            // Create file appender without rotation
            let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
            let (file_writer, file_guard) = non_blocking(file_appender);
            *FILE_GUARD.lock().unwrap() = Some(file_guard);

            // Create console writer
            let (console_writer, console_guard) = non_blocking(std::io::stdout());
            *CONSOLE_GUARD.lock().unwrap() = Some(console_guard);

            // Initialize subscriber with both outputs
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    fmt::layer()
                        .with_writer(file_writer)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_ansi(false),
                )
                .with(
                    fmt::layer()
                        .with_writer(console_writer)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_ansi(true),
                )
                .init();

            debug!(
                "File logging initialized for {} in directory: {}",
                config.module_name,
                log_dir.display()
            );

            info!(
                "[RUST] File logging initialized for module: {} (level: {})",
                config.module_name, config.log_level
            );

            Ok(())
        });

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                init_result = Err(e);
            }
            Err(e) => {
                let error_msg = format!("Panic during file logging initialization: {:?}", e);
                init_result = Err(error_msg.into());
            }
        }

        init_panic_recovery();
    });

    init_result
}

/// Create environment filter for logging
fn create_env_filter(config: &LoggingConfig) -> EnvFilter {
    let submodules = [
        "logging",
        "ffi_utils",
        "error",
        "web_client",
        "core_wrapper",
        "client_manager",
        "session",
        "storage",
        "config_srv",
        "db",
    ];

    EnvFilter::try_from_default_env()
        .map(|mut filter| {
            filter = filter.add_directive(
                format!("{}={}", config.module_name, config.log_level)
                    .parse()
                    .unwrap_or_else(|_| "info".parse().unwrap()),
            );
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
            let mut filter_str = format!("{}={}", config.module_name, config.log_level);
            for submodule in &submodules {
                filter_str.push_str(&format!(
                    ",{}::{}={}",
                    config.module_name, submodule, config.log_level
                ));
            }
            EnvFilter::new(filter_str)
        })
}

/// Set configuration and initialize console logging
pub fn set_and_init_console_logging(level: &str, module_name: &str) {
    let config = LoggingConfig::new(level, module_name);
    init_console_logging(&config);
}

/// Set configuration and initialize file logging
pub fn set_and_init_file_logging(
    level: &str,
    module_name: &str,
    log_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = LoggingConfig::new(level, module_name);
    init_file_logging(&config, log_path)
}

// FFI exports for Go integration
use std::ffi::{c_char, c_int, CStr};

/// FFI wrapper: Initialize console logging
///
/// # Safety
///
/// The caller must ensure that `level` and `module_name` are valid C strings.
#[no_mangle]
pub unsafe extern "C" fn easytier_common_init_console_logging(
    level: *const c_char,
    module_name: *const c_char,
) -> c_int {
    if level.is_null() || module_name.is_null() {
        return -1;
    }

    let level_str = match CStr::from_ptr(level).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let module_str = match CStr::from_ptr(module_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    set_and_init_console_logging(level_str, module_str);
    0
}

/// FFI wrapper: Initialize file logging
///
/// # Safety
///
/// The caller must ensure that `level`, `module_name`, and `log_path` are valid C strings.
#[no_mangle]
pub unsafe extern "C" fn easytier_common_init_file_logging(
    level: *const c_char,
    module_name: *const c_char,
    log_path: *const c_char,
) -> c_int {
    if level.is_null() || module_name.is_null() || log_path.is_null() {
        return -1;
    }

    let level_str = match CStr::from_ptr(level).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let module_str = match CStr::from_ptr(module_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let path_str = match CStr::from_ptr(log_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match set_and_init_file_logging(level_str, module_str, path_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Initialize panic recovery hook
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
            *LAST_PANIC.lock().unwrap() = Some(full_msg.clone());

            tracing::error!("PANIC RECOVERED: {}", panic_msg);
            eprintln!("[EASYTIER PANIC] {}", full_msg);
        }));

        info!("[RUST] EasyTier panic recovery hook initialized");
    });
}

/// Get the last panic message
pub fn get_last_panic() -> Option<String> {
    LAST_PANIC.lock().unwrap().clone()
}

/// Clear the stored panic message
pub fn clear_last_panic() {
    *LAST_PANIC.lock().unwrap() = None;
}

// Logging macros
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*)
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*)
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_logging_init() {
        set_and_init_console_logging("debug", "test_module");
        // Should not panic on second call
        set_and_init_console_logging("info", "test_module");
    }

    #[test]
    fn test_panic_recovery() {
        init_panic_recovery();
        clear_last_panic();

        let result = std::panic::catch_unwind(|| {
            panic!("test panic");
        });

        assert!(result.is_err());
        let panic_msg = get_last_panic();
        assert!(panic_msg.is_some());
        assert!(panic_msg.unwrap().contains("test panic"));
    }
}
