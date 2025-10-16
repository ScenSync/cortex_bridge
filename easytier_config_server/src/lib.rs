//! easytier_config_server
//!
//! Server-side config server for managing device connections.
//! This crate handles device registration, heartbeat processing,
//! and network configuration distribution.

pub mod client_manager;
pub mod config;
pub mod config_srv;
pub mod db;
mod ffi;

pub use client_manager::{session::Session, storage::Storage, ClientManager};
pub use config_srv::NetworkConfigService;
pub use db::Database;
pub use ffi::*;

// Re-export common utilities
pub use easytier_common::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
