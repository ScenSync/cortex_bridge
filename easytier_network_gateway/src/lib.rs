//! easytier_network_gateway
//!
//! Server-side EasyTier network gateway wrapper.
//! This crate allows cortex_server to run its own EasyTier instance
//! acting as a VPN gateway/relay for devices to connect to.

mod core_wrapper;

pub use core_wrapper::*;

// Re-export common utilities
pub use easytier_common::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
