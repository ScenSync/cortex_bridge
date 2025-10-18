//! rerun_bridge
//!
//! FFI bridge for Rerun SDK to enable ROS data visualization from Go cortex_server.
//! This crate converts MCAP messages to Rerun RRD format.

use std::ffi::{c_char, CString};
use std::ptr;
use std::sync::Mutex;

mod recording;
mod converters;
mod error;

pub use recording::*;
pub use converters::*;
pub use error::*;

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
}

