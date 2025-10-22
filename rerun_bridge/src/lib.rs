//! rerun_bridge
//!
//! FFI bridge for Rerun SDK to enable ROS data visualization from Go cortex_server.
//! This crate converts MCAP messages to Rerun RRD format.

use std::ffi::{c_char, CString};
use std::ptr;
use std::sync::Mutex;

mod error;
mod recording;

pub use error::*;
pub use recording::*;

// Re-export logging macros from easytier_common (avoid name conflict with error module)
pub use easytier_common::error as log_error;
pub use easytier_common::{debug, info, trace, warn};

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
pub extern "C" fn rerun_bridge_get_error() -> *const c_char {
    if let Ok(error_msg) = ERROR_MSG.lock() {
        if !error_msg.is_empty() {
            return error_msg.as_ptr() as *const c_char;
        }
    }
    ptr::null()
}

/// Free a C string allocated by Rust
#[no_mangle]
pub extern "C" fn rerun_bridge_free_string(s: *const c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s as *mut c_char);
        }
    }
}

/// Free RRD data buffer
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_bridge_free_rrd_data(data: *mut u8, len: usize) {
    if !data.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(data, len, len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_error_handling() {
        set_error_msg("test error");
        let err = rerun_bridge_get_error();
        assert!(!err.is_null());

        unsafe {
            let err_str = CStr::from_ptr(err).to_str().unwrap();
            assert_eq!(err_str, "test error");
        }
    }

    #[test]
    fn test_error_message_overwrite() {
        // Test that setting a new error overwrites the old one
        set_error_msg("first error");
        let err1 = rerun_bridge_get_error();
        assert!(!err1.is_null());
        
        unsafe {
            let err_str1 = CStr::from_ptr(err1).to_str().unwrap();
            assert_eq!(err_str1, "first error");
        }

        // Overwrite with new error
        set_error_msg("second error");
        let err2 = rerun_bridge_get_error();
        assert!(!err2.is_null());
        
        unsafe {
            let err_str2 = CStr::from_ptr(err2).to_str().unwrap();
            assert_eq!(err_str2, "second error", "New error should overwrite old one");
        }
    }

    #[test]
    fn test_free_string() {
        // Test freeing a CString
        let test_str = CString::new("test string").unwrap();
        let raw_ptr = test_str.into_raw();
        
        // Free the string
        rerun_bridge_free_string(raw_ptr);
        
        // Test freeing null pointer (should be safe)
        rerun_bridge_free_string(ptr::null());
    }

    #[test]
    fn test_free_rrd_data() {
        // Test freeing valid data
        let test_data = vec![1u8, 2, 3, 4, 5];
        let len = test_data.len();
        let ptr = test_data.as_ptr() as *mut u8;
        std::mem::forget(test_data); // Prevent double-free
        
        rerun_bridge_free_rrd_data(ptr, len);
        
        // Test freeing null pointer (should be safe)
        rerun_bridge_free_rrd_data(ptr::null_mut(), 0);
        rerun_bridge_free_rrd_data(ptr::null_mut(), 100);
    }

    #[test]
    fn test_error_with_special_characters() {
        // Test error messages with special characters
        set_error_msg("Error with newline\nand tab\tand quotes\"");
        let err = rerun_bridge_get_error();
        assert!(!err.is_null());
        
        unsafe {
            let err_str = CStr::from_ptr(err).to_str().unwrap();
            assert!(err_str.contains("newline"), "Should contain special characters");
            assert!(err_str.contains("\n"), "Should preserve newlines");
        }
    }

    #[test]
    fn test_empty_error_message() {
        // Test setting an empty error message
        set_error_msg("");
        let err = rerun_bridge_get_error();
        
        // Should still return a valid pointer (to null terminator)
        assert!(!err.is_null());
        
        unsafe {
            let err_str = CStr::from_ptr(err).to_str().unwrap();
            assert_eq!(err_str, "", "Empty error message should be preserved");
        }
    }
}
