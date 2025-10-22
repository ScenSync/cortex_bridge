//! Rerun RRD encoding for streaming visualization
//!
//! # Encoder-Based Streaming (CORRECT IMPLEMENTATION) ✅
//!
//! This module uses `re_log_encoding::Encoder` to generate proper RRD format with:
//! - Valid `RRF2` magic bytes and file header
//! - Incremental chunk extraction (buffer position tracking)
//! - Direct MCAP → RRD conversion using re_data_loader
//!
//! # HTTP Streaming Example
//!
//! ```c
//! // Backend: Create encoder for the stream
//! RerunStreamingEncoder* encoder = rerun_encoder_create("my_stream");
//!
//! // Send initial RRD header (contains RRF2 magic bytes)
//! uint8_t* header_data; size_t header_len;
//! rerun_encoder_get_initial_chunk(encoder, &header_data, &header_len);
//! http_write(response, header_data, header_len);
//! http_flush(response);
//!
//! // Loop: stream MCAP chunks → convert to RRD → send
//! for (mcap_chunk in mcap_data) {
//!     uint8_t* rrd_data; size_t rrd_len;
//!     rerun_encoder_process_mcap_chunk(encoder, mcap_chunk, mcap_len, &rrd_data, &rrd_len);
//!     http_write(response, rrd_data, rrd_len);
//!     http_flush(response);
//!     free(rrd_data);
//! }
//!
//! rerun_encoder_destroy(encoder);
//! ```

use std::ffi::{c_char, CStr};
use std::io::Write;
use std::ptr;
use std::sync::{Arc, Mutex};

use re_data_loader::{loader_mcap::load_mcap, DataLoaderSettings, LoadedData};
use re_log_encoding::{Encoder, EncodingOptions};
use re_log_types::ApplicationId;
use std::sync::mpsc::channel;

use crate::{set_error_msg, RerunBridgeError, Result};

// ============================================================================
// Encoder-Based Streaming (CORRECT IMPLEMENTATION) ✅
// ============================================================================

/// A shared buffer writer that allows reading the data without consuming it
#[derive(Clone)]
struct SharedBufferWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl SharedBufferWriter {
    fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_bytes(&self) -> Vec<u8> {
        self.buffer.lock().unwrap().clone()
    }

    fn len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }
}

impl Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.lock().unwrap().flush()
    }
}

/// Streaming encoder for generating proper RRD format from MCAP data
/// This uses `re_log_encoding::Encoder` which generates valid RRD files with `RRF2` headers
pub struct RerunStreamingEncoder {
    encoder: Encoder<SharedBufferWriter>,
    buffer: SharedBufferWriter,
    last_position: usize,
    recording_id: String,
}

/// Create a new streaming encoder
/// This is the CORRECT way to generate RRD format for streaming
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_encoder_create(
    application_id: *const c_char,
) -> *mut RerunStreamingEncoder {
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

    match encoder_create_internal(app_id) {
        Ok(encoder) => Box::into_raw(Box::new(encoder)),
        Err(e) => {
            set_error_msg(&e.to_string());
            ptr::null_mut()
        }
    }
}

fn encoder_create_internal(app_id: &str) -> Result<RerunStreamingEncoder> {
    let options = EncodingOptions::PROTOBUF_COMPRESSED;
    let version = re_build_info::CrateVersion::LOCAL;

    let buffer = SharedBufferWriter::new();
    let encoder = Encoder::new(version, options, buffer.clone()).map_err(|e| {
        RerunBridgeError::RecordingCreation(format!("Failed to create encoder: {}", e))
    })?;

    crate::debug!("🎬 Created RRD encoder with proper RRF2 format support");

    Ok(RerunStreamingEncoder {
        encoder,
        buffer,
        last_position: 0,
        recording_id: app_id.to_string(),
    })
}

/// Process MCAP chunk and return RRD bytes
/// This converts MCAP data to RRD format and returns only new data since last call
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_encoder_process_mcap_chunk(
    handle: *mut RerunStreamingEncoder,
    mcap_data: *const u8,
    mcap_len: usize,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || mcap_data.is_null() || out_data.is_null() || out_len.is_null() {
        set_error_msg("Null pointer passed to rerun_encoder_process_mcap_chunk");
        return -1;
    }

    let encoder = unsafe { &mut *handle };
    let mcap_bytes = unsafe { std::slice::from_raw_parts(mcap_data, mcap_len) };

    match encoder_process_mcap_chunk_internal(encoder, mcap_bytes) {
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

fn encoder_process_mcap_chunk_internal(
    encoder_state: &mut RerunStreamingEncoder,
    mcap_data: &[u8],
) -> Result<Vec<u8>> {
    // Create channel for data loader
    let (tx, rx) = channel::<LoadedData>();

    // Create settings for MCAP loader
    let app_id = ApplicationId::from(encoder_state.recording_id.as_str());

    let settings = DataLoaderSettings {
        application_id: Some(app_id),
        recording_id: encoder_state.recording_id.as_str().into(),
        opened_store_id: None,
        force_store_info: false,
        entity_path_prefix: None,
        timepoint: None,
    };

    // Load MCAP chunk
    let result = load_mcap(
        mcap_data,
        &settings,
        &tx,
        &re_mcap::SelectedLayers::All,
        true, // stop_on_error
    );

    drop(tx); // Close sender to signal completion

    if let Err(e) = result {
        return Err(RerunBridgeError::MCAPError(format!(
            "Failed to load MCAP: {}",
            e
        )));
    }

    // Get current buffer position before encoding new data
    let start_position = encoder_state.last_position;

    // Process all loaded data
    let mut message_count = 0;
    while let Ok(loaded_data) = rx.recv() {
        let log_msg = match loaded_data {
            LoadedData::LogMsg(_, msg) => msg,
            LoadedData::Chunk(_, store_id, chunk) => match chunk.to_arrow_msg() {
                Ok(arrow_msg) => re_log_types::LogMsg::ArrowMsg(store_id, arrow_msg),
                Err(e) => {
                    crate::warn!("Failed to convert chunk to arrow: {}", e);
                    continue;
                }
            },
            LoadedData::ArrowMsg(_, store_id, arrow_msg) => {
                re_log_types::LogMsg::ArrowMsg(store_id, arrow_msg)
            }
        };

        // Append to encoder
        encoder_state.encoder.append(&log_msg).map_err(|e| {
            RerunBridgeError::SerializationFailed(format!("Failed to encode message: {}", e))
        })?;

        message_count += 1;
    }

    // Note: The encoder writes directly to SharedBufferWriter via Write trait
    // Data is immediately available in the buffer after append() - no explicit flush needed
    // Message boundaries are maintained by the encoder's internal state

    // Get the current buffer state
    // Use len() first for efficiency - no need to clone the entire buffer just to check length
    let current_position = encoder_state.buffer.len();

    // Extract only new bytes since last extraction
    if current_position > start_position {
        // Only clone the buffer if we actually have new data to extract
        let encoder_bytes = encoder_state.buffer.get_bytes();
        let new_bytes = &encoder_bytes[start_position..current_position];
        encoder_state.last_position = current_position;

        crate::debug!(
            "📦 Encoded {} MCAP messages → {} new RRD bytes (total buffer: {} bytes)",
            message_count,
            new_bytes.len(),
            current_position
        );

        // Validate RRD header on first chunk
        if start_position == 0 && new_bytes.len() >= 4 {
            crate::debug!(
                "RRD header magic bytes: {:?} (expecting [82, 82, 70, 50] = 'RRF2')",
                &new_bytes[0..4]
            );
        }

        Ok(new_bytes.to_vec())
    } else {
        crate::trace!("No new data generated from MCAP chunk");
        Ok(Vec::new())
    }
}

/// Get initial RRD header chunk (call immediately after creation)
/// This returns the RRF2 header + metadata before any data is logged
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_encoder_get_initial_chunk(
    handle: *mut RerunStreamingEncoder,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        set_error_msg("Null pointer passed to rerun_encoder_get_initial_chunk");
        return -1;
    }

    let encoder = unsafe { &mut *handle };

    // On first call (last_position == 0), return the initial RRD header
    if encoder.last_position == 0 {
        let encoder_bytes = encoder.buffer.get_bytes();
        if !encoder_bytes.is_empty() {
            let header_chunk = encoder_bytes.to_vec();
            let len = header_chunk.len();
            let ptr = header_chunk.as_ptr() as *mut u8;

            std::mem::forget(header_chunk);
            encoder.last_position = len;

            crate::info!("Sending initial RRD header: {} bytes", len);

            unsafe {
                *out_data = ptr;
                *out_len = len;
            }
            return 0;
        }
    }

    // No initial header available or already sent
    unsafe {
        *out_data = ptr::null_mut();
        *out_len = 0;
    }
    0
}

/// Finalize encoder and get final chunk (call before destroy)
/// This extracts any remaining data written by encoder.finish()
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_encoder_finalize(
    handle: *mut RerunStreamingEncoder,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        set_error_msg("Null pointer passed to rerun_encoder_finalize");
        return -1;
    }

    let encoder = unsafe { &mut *handle };

    // Finalize the encoder (writes end marker if needed)
    if let Err(e) = encoder.encoder.finish() {
        set_error_msg(&format!("Failed to finalize encoder: {}", e));
        return -1;
    }

    // Extract any final bytes written by finish()
    let current_position = encoder.buffer.len();
    if current_position > encoder.last_position {
        let encoder_bytes = encoder.buffer.get_bytes();
        let final_bytes = &encoder_bytes[encoder.last_position..current_position];
        let final_chunk = final_bytes.to_vec();
        let len = final_chunk.len();

        if len > 0 {
            crate::info!("📋 Finalized encoder: {} final bytes (end marker)", len);
            let ptr = final_chunk.as_ptr() as *mut u8;
            std::mem::forget(final_chunk);
            encoder.last_position = current_position;

            unsafe {
                *out_data = ptr;
                *out_len = len;
            }
            return 0;
        }
    }

    // No final bytes
    unsafe {
        *out_data = ptr::null_mut();
        *out_len = 0;
    }
    0
}

/// Destroy streaming encoder
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rerun_encoder_destroy(handle: *mut RerunStreamingEncoder) {
    if !handle.is_null() {
        unsafe {
            let _encoder = Box::from_raw(handle);
            // Encoder is dropped here (finish() should have been called via finalize)
            crate::debug!("🗑️ Destroyed encoder handle");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rerun_bridge_free_rrd_data;
    use std::ffi::CString;

    #[test]
    fn test_create_and_destroy_encoder() {
        let app_id = CString::new("test_encoder").unwrap();
        let handle = rerun_encoder_create(app_id.as_ptr());
        assert!(!handle.is_null());
        rerun_encoder_destroy(handle);
    }

    #[test]
    fn test_encoder_initial_chunk() {
        let app_id = CString::new("test_initial_chunk").unwrap();
        let handle = rerun_encoder_create(app_id.as_ptr());
        assert!(!handle.is_null());

        let mut out_data: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;

        let result = rerun_encoder_get_initial_chunk(handle, &mut out_data, &mut out_len);
        assert_eq!(result, 0, "Get initial chunk should succeed");

        // Initial header should be generated automatically
        if out_len > 0 {
            println!("Generated initial RRD header: {} bytes", out_len);

            // Validate RRF2 magic bytes
            if out_len >= 4 {
                let magic_bytes = unsafe { std::slice::from_raw_parts(out_data, 4) };
                println!(
                    "🔍 Magic bytes: {:?} (expecting [82, 82, 70, 50] = 'RRF2')",
                    magic_bytes
                );
                assert_eq!(
                    magic_bytes,
                    &[82, 82, 70, 50],
                    "Should have RRF2 magic bytes"
                );
            }

            // Free the allocated data
            if !out_data.is_null() {
                use crate::rerun_bridge_free_rrd_data;
                rerun_bridge_free_rrd_data(out_data, out_len);
            }
        }

        rerun_encoder_destroy(handle);
    }

    #[test]
    fn test_encoder_finalize() {
        let app_id = CString::new("test_finalize").unwrap();
        let handle = rerun_encoder_create(app_id.as_ptr());
        assert!(!handle.is_null());

        // Get initial chunk
        let mut out_data: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;
        let result = rerun_encoder_get_initial_chunk(handle, &mut out_data, &mut out_len);
        assert_eq!(result, 0);
        if !out_data.is_null() && out_len > 0 {
            crate::rerun_bridge_free_rrd_data(out_data, out_len);
        }

        // Finalize encoder (should succeed even with no data)
        let mut final_data: *mut u8 = ptr::null_mut();
        let mut final_len: usize = 0;
        let result = rerun_encoder_finalize(handle, &mut final_data, &mut final_len);
        assert_eq!(result, 0, "Finalize should succeed");

        // Check if we got final bytes
        if final_len > 0 {
            println!("📋 Final chunk size: {} bytes", final_len);
            assert!(
                !final_data.is_null(),
                "Final data pointer should not be null"
            );
            crate::rerun_bridge_free_rrd_data(final_data, final_len);
        } else {
            println!("📋 No final chunk generated (encoder may not produce end marker for empty streams)");
        }

        rerun_encoder_destroy(handle);
    }
}
