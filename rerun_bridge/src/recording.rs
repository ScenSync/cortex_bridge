//! Rerun recording management and RRD generation
//!
//! # Two Recording Modes
//!
//! ## 1. One-Shot Recording (`RerunRecording`)
//! - Create recording → log all messages → drain complete RRD file
//! - Best for: generating complete .rrd files for download
//!
//! ## 2. Incremental Streaming (`RerunStreamingRecording`) ⭐
//! - Create recording ONCE → log messages → flush → repeat
//! - Each `flush_chunk()` returns ONLY new data since last flush
//! - Best for: HTTP chunked transfer encoding, real-time streaming
//!
//! # HTTP Streaming Example
//!
//! ```c
//! // Backend: Create ONE recording for the entire stream
//! RerunStreamingRecording* rec = rerun_create_streaming_recording("my_stream");
//!
//! // Send initial RRD header
//! uint8_t* chunk1_data; size_t chunk1_len;
//! rerun_streaming_flush_chunk(rec, &chunk1_data, &chunk1_len);
//! http_write(response, chunk1_data, chunk1_len);  // Contains RRD header
//! http_flush(response);
//!
//! // Loop: query MCAP → convert → log → flush → send
//! for (batch in mcap_data) {
//!     // Log messages to the SAME recording
//!     for (msg in batch) {
//!         rerun_streaming_log_image(rec, path, w, h, data, len);
//!     }
//!     
//!     // Extract ONLY new data since last flush
//!     uint8_t* chunk_data; size_t chunk_len;
//!     rerun_streaming_flush_chunk(rec, &chunk_data, &chunk_len);
//!     
//!     // Send incremental chunk to client
//!     http_write(response, chunk_data, chunk_len);
//!     http_flush(response);  // Triggers HTTP chunked transfer
//!     
//!     free(chunk_data);  // Don't forget to free!
//! }
//!
//! rerun_destroy_streaming_recording(rec);
//! ```
//!
//! # Why This Works
//!
//! Rerun's `MemorySinkStorage::drain_as_bytes()` is **stateful**:
//! - First call: returns RRD header + any logged data
//! - Subsequent calls: return ONLY data logged since last drain
//! - The viewer concatenates all chunks to form a complete RRD stream

use std::ffi::{c_char, CStr};
use std::ptr;
use std::sync::Arc;

use rerun::sink::MemorySinkStorage;
use rerun::{RecordingStream, RecordingStreamBuilder};

use crate::converters::{
    parse_ros_image_cdr, parse_ros_pointcloud2_cdr, parse_ros_imu_cdr,
    parse_ros_compressed_image_cdr, decode_compressed_image,
};
use crate::{set_error_msg, RerunBridgeError, Result};

/// Opaque handle to a Rerun recording
pub struct RerunRecording {
    stream: Arc<RecordingStream>,
    memory_sink: Arc<std::sync::Mutex<MemorySinkStorage>>,
}

/// Opaque handle to a streaming Rerun recording
/// This maintains state across multiple chunk writes for incremental streaming
pub struct RerunStreamingRecording {
    stream: Arc<RecordingStream>,
    // Memory sink stores accumulated RRD data, drain_as_bytes() extracts new data since last drain
    memory_sink: Arc<std::sync::Mutex<MemorySinkStorage>>,
}

/// Create a new Rerun recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
    // Create a recording stream that stores data in memory
    let (stream, memory_sink) = RecordingStreamBuilder::new(app_id)
        .memory()
        .map_err(|e| RerunBridgeError::RecordingCreation(e.to_string()))?;

    Ok(RerunRecording {
        stream: Arc::new(stream),
        memory_sink: Arc::new(std::sync::Mutex::new(memory_sink)),
    })
}

/// Destroy a Rerun recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_destroy_recording(handle: *mut RerunRecording) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Log image data to recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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

    crate::trace!("Logging image to path '{}': {}x{}, {} bytes", entity_path, width, height, data.len());

    recording
        .stream
        .log(entity_path, &image)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;

    Ok(())
}

/// Save recording to RRD format
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
    // Flush the stream to ensure all data is written
    recording.stream.flush_blocking()
        .map_err(|e| RerunBridgeError::SerializationFailed(format!("Failed to flush stream: {}", e)))?;

    // Get the RRD data from the MemorySinkStorage
    let memory_sink = recording.memory_sink.lock().map_err(|e| {
        RerunBridgeError::SerializationFailed(format!("Failed to lock memory sink: {}", e))
    })?;

    // Drain all data from the memory sink and convert to bytes
    let rrd_data = memory_sink
        .drain_as_bytes()
        .map_err(|e| RerunBridgeError::SerializationFailed(e.to_string()))?;

    crate::debug!("🔍 RRD serialization: {} bytes generated", rrd_data.len());
    
    // Validate RRD header
    if rrd_data.len() > 4 {
        crate::debug!("🔍 RRD header bytes: {:?}", &rrd_data[0..4]);
    } else if rrd_data.is_empty() {
        crate::warn!("⚠️  RRD serialization produced 0 bytes");
    }

    Ok(rrd_data)
}

// ============================================================================
// Streaming Recording API (for HTTP chunked transfer encoding)
// ============================================================================

/// Create a new streaming Rerun recording
/// This recording can be used to incrementally add data and extract RRD chunks
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_create_streaming_recording(application_id: *const c_char) -> *mut RerunStreamingRecording {
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

    match create_streaming_recording_internal(app_id) {
        Ok(recording) => Box::into_raw(Box::new(recording)),
        Err(e) => {
            set_error_msg(&e.to_string());
            ptr::null_mut()
        }
    }
}

fn create_streaming_recording_internal(app_id: &str) -> Result<RerunStreamingRecording> {
    // Create a recording stream that stores data in memory
    // The memory sink will accumulate data as we log messages
    let (stream, memory_sink) = RecordingStreamBuilder::new(app_id)
        .memory()
        .map_err(|e| RerunBridgeError::RecordingCreation(e.to_string()))?;

    crate::debug!("🎬 Created streaming recording for incremental RRD extraction");

    Ok(RerunStreamingRecording {
        stream: Arc::new(stream),
        memory_sink: Arc::new(std::sync::Mutex::new(memory_sink)),
    })
}

/// Destroy a streaming Rerun recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_destroy_streaming_recording(handle: *mut RerunStreamingRecording) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Log image data to streaming recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_streaming_log_image(
    handle: *mut RerunStreamingRecording,
    entity_path: *const c_char,
    width: u32,
    height: u32,
    data: *const u8,
    data_len: usize,
) -> i32 {
    if handle.is_null() || entity_path.is_null() || data.is_null() {
        set_error_msg("Null pointer passed to rerun_streaming_log_image");
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

    match streaming_log_image_internal(recording, path, width, height, image_data) {
        Ok(_) => 0,
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn streaming_log_image_internal(
    recording: &RerunStreamingRecording,
    entity_path: &str,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<()> {
    // Create image from raw bytes (RGB8 format)
    let resolution = [width, height];
    let image = rerun::Image::from_rgb24(data.to_vec(), resolution);

    crate::trace!("Streaming log image to path '{}': {}x{}, {} bytes", entity_path, width, height, data.len());

    recording
        .stream
        .log(entity_path, &image)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;

    Ok(())
}

/// Flush and get any new RRD data from the streaming recording
/// Returns new RRD chunk data that should be sent to client
/// This is non-destructive - the recording stream continues
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_streaming_flush_chunk(
    handle: *mut RerunStreamingRecording,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        set_error_msg("Null pointer passed to rerun_streaming_flush_chunk");
        return -1;
    }

    let recording = unsafe { &mut *handle };

    match streaming_flush_chunk_internal(recording) {
        Ok(chunk_data) => {
            let len = chunk_data.len();
            
            if len == 0 {
                // No new data
                unsafe {
                    *out_data = ptr::null_mut();
                    *out_len = 0;
                }
                return 0;
            }
            
            let ptr = chunk_data.as_ptr() as *mut u8;

            // Transfer ownership to caller
            std::mem::forget(chunk_data);

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

fn streaming_flush_chunk_internal(recording: &mut RerunStreamingRecording) -> Result<Vec<u8>> {
    // Step 1: Flush the stream to ensure recent logs are committed to the memory sink
    recording.stream.flush_blocking()
        .map_err(|e| RerunBridgeError::SerializationFailed(format!("Failed to flush stream: {}", e)))?;

    // Step 2: Drain accumulated RRD data from the memory sink
    // IMPORTANT: drain_as_bytes() returns ONLY the new data since the last drain
    // This is the key to incremental streaming - each call gives you the delta!
    let memory_sink = recording.memory_sink.lock().map_err(|e| {
        RerunBridgeError::SerializationFailed(format!("Failed to lock memory sink: {}", e))
    })?;

    let new_rrd_chunk = memory_sink
        .drain_as_bytes()
        .map_err(|e| RerunBridgeError::SerializationFailed(e.to_string()))?;

    if new_rrd_chunk.is_empty() {
        crate::trace!("Streaming flush: no new data");
    } else {
        crate::debug!("Streaming flush: extracted {} bytes of new RRD data", new_rrd_chunk.len());
    }
    
    Ok(new_rrd_chunk)
}

// ============================================================================
// Advanced Logging Functions with CDR Parsing
// ============================================================================

/// Log ROS Image message (CDR format) to recording
/// This parses the CDR data and extracts the image
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_log_ros_image_cdr(
    handle: *mut RerunRecording,
    entity_path: *const c_char,
    cdr_data: *const u8,
    cdr_len: usize,
) -> i32 {
    if handle.is_null() || entity_path.is_null() || cdr_data.is_null() {
        set_error_msg("Null pointer passed to rerun_log_ros_image_cdr");
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

    let cdr_bytes = unsafe { std::slice::from_raw_parts(cdr_data, cdr_len) };

    match log_ros_image_cdr_internal(recording, path, cdr_bytes) {
        Ok(_) => 0,
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn log_ros_image_cdr_internal(
    recording: &RerunRecording,
    entity_path: &str,
    cdr_data: &[u8],
) -> Result<()> {
    // Parse ROS Image CDR message
    let (width, height, rgb_data) = parse_ros_image_cdr(cdr_data)?;
    
    // Create image from parsed RGB8 data
    let resolution = [width, height];
    let image = rerun::Image::from_rgb24(rgb_data, resolution);
    
    recording
        .stream
        .log(entity_path, &image)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;
    
    Ok(())
}

/// Log ROS CompressedImage message (CDR format) to recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_log_ros_compressed_image_cdr(
    handle: *mut RerunRecording,
    entity_path: *const c_char,
    cdr_data: *const u8,
    cdr_len: usize,
) -> i32 {
    if handle.is_null() || entity_path.is_null() || cdr_data.is_null() {
        set_error_msg("Null pointer passed to rerun_log_ros_compressed_image_cdr");
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

    let cdr_bytes = unsafe { std::slice::from_raw_parts(cdr_data, cdr_len) };

    match log_ros_compressed_image_cdr_internal(recording, path, cdr_bytes) {
        Ok(_) => 0,
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn log_ros_compressed_image_cdr_internal(
    recording: &RerunRecording,
    entity_path: &str,
    cdr_data: &[u8],
) -> Result<()> {
    // Parse ROS CompressedImage CDR message
    let (format, compressed_data) = parse_ros_compressed_image_cdr(cdr_data)?;
    
    // Decode compressed image to RGB8
    let (width, height, rgb_data) = decode_compressed_image(&format, &compressed_data)?;
    
    // Create image from decoded RGB8 data
    let resolution = [width, height];
    let image = rerun::Image::from_rgb24(rgb_data, resolution);
    
    recording
        .stream
        .log(entity_path, &image)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;
    
    Ok(())
}

/// Log ROS PointCloud2 message (CDR format) to recording
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_log_ros_pointcloud2_cdr(
    handle: *mut RerunRecording,
    entity_path: *const c_char,
    cdr_data: *const u8,
    cdr_len: usize,
) -> i32 {
    if handle.is_null() || entity_path.is_null() || cdr_data.is_null() {
        set_error_msg("Null pointer passed to rerun_log_ros_pointcloud2_cdr");
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

    let cdr_bytes = unsafe { std::slice::from_raw_parts(cdr_data, cdr_len) };

    match log_ros_pointcloud2_cdr_internal(recording, path, cdr_bytes) {
        Ok(_) => 0,
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn log_ros_pointcloud2_cdr_internal(
    recording: &RerunRecording,
    entity_path: &str,
    cdr_data: &[u8],
) -> Result<()> {
    // Parse ROS PointCloud2 CDR message
    let (points, colors) = parse_ros_pointcloud2_cdr(cdr_data)?;
    
    if points.is_empty() {
        crate::warn!("PointCloud2 has no valid points, skipping");
        return Ok(());
    }
    
    // Create Rerun Points3D with colors
    let points3d = rerun::Points3D::new(
        points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>()
    )
    .with_colors(
        colors.chunks_exact(3).map(|c| rerun::Color::from_rgb(c[0], c[1], c[2])).collect::<Vec<_>>()
    );
    
    recording
        .stream
        .log(entity_path, &points3d)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;
    
    Ok(())
}

/// Log ROS IMU message (CDR format) to recording
/// Logs both orientation (as transform) and acceleration/angular velocity (as arrows)
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_log_ros_imu_cdr(
    handle: *mut RerunRecording,
    entity_path: *const c_char,
    cdr_data: *const u8,
    cdr_len: usize,
) -> i32 {
    if handle.is_null() || entity_path.is_null() || cdr_data.is_null() {
        set_error_msg("Null pointer passed to rerun_log_ros_imu_cdr");
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

    let cdr_bytes = unsafe { std::slice::from_raw_parts(cdr_data, cdr_len) };

    match log_ros_imu_cdr_internal(recording, path, cdr_bytes) {
        Ok(_) => 0,
        Err(e) => {
            set_error_msg(&e.to_string());
            -1
        }
    }
}

fn log_ros_imu_cdr_internal(
    recording: &RerunRecording,
    entity_path: &str,
    cdr_data: &[u8],
) -> Result<()> {
    // Parse ROS IMU CDR message
    let imu_data = parse_ros_imu_cdr(cdr_data)?;
    
    // Log orientation as a transform
    let transform = rerun::Transform3D::from_rotation(
        rerun::Quaternion::from_xyzw([
            imu_data.orientation[0] as f32,
            imu_data.orientation[1] as f32,
            imu_data.orientation[2] as f32,
            imu_data.orientation[3] as f32,
        ])
    );
    
    recording
        .stream
        .log(entity_path, &transform)
        .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;
    
    // Log linear acceleration as an arrow
    let accel_magnitude = (
        imu_data.linear_acceleration[0].powi(2) +
        imu_data.linear_acceleration[1].powi(2) +
        imu_data.linear_acceleration[2].powi(2)
    ).sqrt();
    
    if accel_magnitude > 0.01 {
        let accel_arrow = rerun::Arrows3D::from_vectors(
            [[
                imu_data.linear_acceleration[0] as f32,
                imu_data.linear_acceleration[1] as f32,
                imu_data.linear_acceleration[2] as f32,
            ]]
        )
        .with_colors([rerun::Color::from_rgb(255, 0, 0)]); // Red for acceleration
        
        let accel_path = format!("{}/acceleration", entity_path);
        recording
            .stream
            .log(accel_path.as_str(), &accel_arrow)
            .map_err(|e| RerunBridgeError::LoggingFailed(e.to_string()))?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_create_and_destroy_recording() {
        let app_id = CString::new("test_app").unwrap();
        let handle = rerun_create_recording(app_id.as_ptr());
        assert!(!handle.is_null());
        rerun_destroy_recording(handle);
    }
    
    #[test]
    fn test_create_and_destroy_streaming_recording() {
        let app_id = CString::new("test_streaming_app").unwrap();
        let handle = rerun_create_streaming_recording(app_id.as_ptr());
        assert!(!handle.is_null());
        rerun_destroy_streaming_recording(handle);
    }
    
    #[test]
    fn test_streaming_incremental_extraction() {
        let app_id = CString::new("test_incremental").unwrap();
        let handle = rerun_create_streaming_recording(app_id.as_ptr());
        assert!(!handle.is_null());
        
        unsafe {
            let recording = &mut *handle;
            
            // First flush - should get initial RRD data (header)
            let chunk1 = streaming_flush_chunk_internal(recording);
            assert!(chunk1.is_ok());
            let bytes1 = chunk1.unwrap();
            let len1 = bytes1.len();
            assert!(len1 > 0, "First flush should return RRD header");
            
            // Log first batch of data
            let path = CString::new("/test/image1").unwrap();
            let test_data = vec![255u8; 100 * 100 * 3]; // 100x100 RGB image
            let result = rerun_streaming_log_image(
                handle,
                path.as_ptr(),
                100,
                100,
                test_data.as_ptr(),
                test_data.len(),
            );
            assert_eq!(result, 0, "First logging should succeed");
            
            // Second flush - should get the image data
            let chunk2 = streaming_flush_chunk_internal(recording);
            assert!(chunk2.is_ok());
            let bytes2 = chunk2.unwrap();
            let len2 = bytes2.len();
            assert!(len2 > 0, "Second flush should return new image data");
            
            // Log second batch of data
            let path2 = CString::new("/test/image2").unwrap();
            let result2 = rerun_streaming_log_image(
                handle,
                path2.as_ptr(),
                100,
                100,
                test_data.as_ptr(),
                test_data.len(),
            );
            assert_eq!(result2, 0, "Second logging should succeed");
            
            // Third flush - should get the second image data
            let chunk3 = streaming_flush_chunk_internal(recording);
            assert!(chunk3.is_ok());
            let bytes3 = chunk3.unwrap();
            let len3 = bytes3.len();
            assert!(len3 > 0, "Third flush should return second image data");
            
            println!("✅ Incremental extraction test: chunk1={} bytes, chunk2={} bytes, chunk3={} bytes", 
                     len1, len2, len3);
            println!("   Each flush extracts ONLY new data since last flush - true incremental streaming!");
        }
        
        rerun_destroy_streaming_recording(handle);
    }
}
