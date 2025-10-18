//! Rerun recording management and RRD generation

use std::ffi::{c_char, CStr};
use std::ptr;
use std::sync::Arc;

use rerun::{RecordingStream, RecordingStreamBuilder};

use crate::{set_error_msg, RerunBridgeError, Result};

/// Opaque handle to a Rerun recording
pub struct RerunRecording {
    stream: Arc<RecordingStream>,
    buffer: Vec<u8>,
}

/// Create a new Rerun recording
#[no_mangle]
pub extern "C" fn rerun_create_recording(application_id: *const c_char) -> *mut RerunRecording {
    if application_id.is_null() {
        set_error_msg("application_id is null");
        return ptr::null_mut();
    }

    let app_id = unsafe {
        match CStr::from_ptr(application_id).to_str() {
            Ok(s) => s,
            Err(e) => {
                set_error_msg(&format!("Invalid UTF-8 in application_id: {}", e));
                return ptr::null_mut();
            }
        }
    };

    match create_recording_internal(app_id) {
        Ok(recording) => Box::into_raw(Box::new(recording)),
        Err(e) => {
            set_error_msg(&e.to_string());
            ptr::null_mut()
        }
    }
}

fn create_recording_internal(app_id: &str) -> Result<RerunRecording> {
    let stream = RecordingStreamBuilder::new(app_id)
        .buffered()
        .map_err(|e| RerunBridgeError::RecordingCreation(e.to_string()))?;

    Ok(RerunRecording {
        stream: Arc::new(stream),
        buffer: Vec::new(),
    })
}

/// Destroy a Rerun recording
#[no_mangle]
pub extern "C" fn rerun_destroy_recording(handle: *mut RerunRecording) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Log image data to recording
#[no_mangle]
pub extern "C" fn rerun_log_image(
    handle: *mut RerunRecording,
    entity_path: *const c_char,
    width: u32,
    height: u32,
    data: *const u8,
    data_len: usize,
) -> i32 {
    if handle.is_null() || entity_path.is_null() || data.is_null() {
        set_error_msg("Null pointer passed to rerun_log_image");
        return -1;
    }

    let recording = unsafe { &mut *handle };
    let path = unsafe {
        match CStr::from_ptr(entity_path).to_str() {
            Ok(s) => s,
            Err(e) => {
                set_error_msg(&format!("Invalid UTF-8 in entity_path: {}", e));
                return -1;
            }
        }
    };

    let image_data = unsafe { std::slice::from_raw_parts(data, data_len) };

    match log_image_internal(recording, path, width, height, image_data) {
        Ok(_) => 0,
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn log_image_internal(
    recording: &RerunRecording,
    entity_path: &str,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<()> {
    // Create image from raw bytes (RGB8 format)
    let resolution = [width, height];
    let image = rerun::Image::from_rgb24(data.to_vec(), resolution);

    recording
        .stream
        .log(entity_path, &image)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;

    Ok(())
}

/// Save recording to RRD format
#[no_mangle]
pub extern "C" fn rerun_save_to_rrd(
    handle: *mut RerunRecording,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        set_error_msg("Null pointer passed to rerun_save_to_rrd");
        return -1;
    }

    let recording = unsafe { &mut *handle };

    match save_to_rrd_internal(recording) {
        Ok(rrd_data) => {
            let len = rrd_data.len();
            let ptr = rrd_data.as_ptr() as *mut u8;
            
            // Transfer ownership to caller
            std::mem::forget(rrd_data);
            
            unsafe {
                *out_data = ptr;
                *out_len = len;
            }
            0
        }
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn save_to_rrd_internal(recording: &mut RerunRecording) -> Result<Vec<u8>> {
    // Flush the stream and get RRD bytes
    recording.stream.flush_blocking();
    
    // TODO: Implement actual RRD serialization
    // For now, return empty buffer as placeholder
    // In production, this would call recording.stream.to_bytes() or similar
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_destroy_recording() {
        let app_id = CString::new("test_app").unwrap();
        let handle = rerun_create_recording(app_id.as_ptr());
        assert!(!handle.is_null());
        rerun_destroy_recording(handle);
    }
}

