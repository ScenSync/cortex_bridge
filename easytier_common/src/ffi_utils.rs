//! FFI utility functions for C interoperability

use std::ffi::{c_char, CStr, CString};

/// Convert C string to Rust String
///
/// # Safety
///
/// The caller must ensure that `c_str` is a valid pointer to a null-terminated C string.
pub unsafe fn c_str_to_string(c_str: *const c_char) -> Result<String, &'static str> {
    if c_str.is_null() {
        return Err("Null pointer");
    }
    CStr::from_ptr(c_str)
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| "Invalid UTF-8")
}

/// Convert Rust string to C string (caller must free)
pub fn string_to_c_str(s: &str) -> Result<*mut c_char, &'static str> {
    CString::new(s)
        .map(|cs| cs.into_raw())
        .map_err(|_| "String contains null byte")
}

/// Parse an array of C strings
///
/// # Safety
///
/// The caller must ensure that:
/// - `arr` is a valid pointer to an array of C string pointers
/// - The array has at least `count` elements
/// - All C string pointers in the array are valid and null-terminated
pub unsafe fn parse_string_array(
    arr: *const *const c_char,
    count: i32,
) -> Result<Vec<String>, &'static str> {
    if count <= 0 {
        return Ok(Vec::new());
    }

    if arr.is_null() {
        return Err("Null pointer for string array");
    }

    let slice = std::slice::from_raw_parts(arr, count as usize);
    let mut result = Vec::with_capacity(count as usize);

    for &ptr in slice {
        let s = c_str_to_string(ptr)?;
        result.push(s);
    }

    Ok(result)
}

/// Free an array of C strings
///
/// # Safety
///
/// The caller must ensure that `arr` was allocated by Rust and contains `count` valid C string pointers.
#[no_mangle]
pub unsafe extern "C" fn easytier_common_free_string_array(arr: *const *const c_char, count: i32) {
    if !arr.is_null() && count > 0 {
        for i in 0..count {
            let ptr = *arr.offset(i as isize);
            if !ptr.is_null() {
                let _ = CString::from_raw(ptr as *mut c_char);
            }
        }
        let _ = Vec::from_raw_parts(arr as *mut *const c_char, count as usize, count as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_str_to_string() {
        let test_str = CString::new("test").unwrap();
        let result = unsafe { c_str_to_string(test_str.as_ptr()) };
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_c_str_to_string_null() {
        let result = unsafe { c_str_to_string(std::ptr::null()) };
        assert!(result.is_err());
    }

    #[test]
    fn test_string_to_c_str() {
        let result = string_to_c_str("test").unwrap();
        unsafe {
            let back = CStr::from_ptr(result).to_str().unwrap();
            assert_eq!(back, "test");
            let _ = CString::from_raw(result);
        }
    }
}
