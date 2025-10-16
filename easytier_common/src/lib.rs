//! easytier_common
//!
//! Common utilities and shared functionality for EasyTier integration crates.
//! This crate provides logging, FFI utilities, and error handling.

#[cfg(test)]
use std::ffi::CStr;
use std::ffi::{c_char, CString};
use std::ptr;
use std::sync::Mutex;

mod error;
mod ffi_utils;
mod logging;

pub use error::*;
pub use ffi_utils::*;
pub use logging::*;

// Global error message storage for FFI
static ERROR_MSG: once_cell::sync::Lazy<Mutex<Vec<u8>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Set error message for FFI error reporting
pub fn set_error_msg(msg: &str) {
    if let Ok(mut error_msg) = ERROR_MSG.lock() {
        error_msg.clear();
        error_msg.extend_from_slice(msg.as_bytes());
        error_msg.push(0); // null terminator
    }
}

/// Get last error message
#[no_mangle]
pub extern "C" fn easytier_common_get_error_msg() -> *const c_char {
    if let Ok(error_msg) = ERROR_MSG.lock() {
        if !error_msg.is_empty() {
            return error_msg.as_ptr() as *const c_char;
        }
    }
    ptr::null()
}

/// Free a C string allocated by Rust
#[no_mangle]
pub extern "C" fn easytier_common_free_string(s: *const c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s as *mut c_char);
        }
    }
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // VERSION is a compile-time constant from CARGO_PKG_VERSION
        assert!(VERSION.contains('.'), "Version should be in semver format");
    }

    #[test]
    fn test_error_msg() {
        set_error_msg("test error");
        let msg = easytier_common_get_error_msg();
        assert!(!msg.is_null());

        unsafe {
            let c_str = CStr::from_ptr(msg);
            assert_eq!(c_str.to_str().unwrap(), "test error");
        }
    }
}
