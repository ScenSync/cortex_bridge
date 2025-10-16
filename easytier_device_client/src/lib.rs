//! easytier_device_client
//!
//! Device-side EasyTier web client for connecting to config server.
//! This crate is used by cortex_agent (devices) to establish connection
//! with cortex_server's config server.

mod stun_wrapper;
mod web_client;

pub use stun_wrapper::MockStunInfoCollectorWrapper;
pub use web_client::*;

// Re-export common utilities
pub use easytier_common::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
