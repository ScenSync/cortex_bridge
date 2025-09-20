# Cortex EasyTier Bridge

A unified Rust crate that provides comprehensive EasyTier integration for Cortex applications, combining both core EasyTier functionality and web client management capabilities.

## Overview

This crate merges the functionality of two previously separate crates:
- `cortex-easytier-core`: Direct EasyTier core integration
- `cortex-easytier-web`: EasyTier Web Client Manager with MySQL storage

## Features

### Core Features (always enabled)
- Direct EasyTier core integration
- Network instance management
- STUN wrapper functionality
- C FFI interfaces for core functionality
- Logging and panic recovery

### Web Features (optional, enabled by default)
- EasyTier Web Client Manager
- MySQL database storage integration
- Session and storage management
- GeoIP configuration support
- Additional C FFI interfaces for web functionality

## Feature Flags

- `default = ["core", "web"]` - Enables both core and web functionality
- `core` - Core EasyTier integration (always available)
- `web` - Web client management features
- `websocket` - WebSocket support (part of web features)
- `database` - Database integration (sea-orm, sea-orm-migration)

## Usage

### Basic Usage (Core Only)

```toml
[dependencies]
easytier-bridge = { version = "0.1.0", default-features = false, features = ["core"] }
```

### Full Usage (Core + Web)

```toml
[dependencies]
easytier-bridge = "0.1.0"
```

### Rust API

```rust
use cortex_easytier_bridge::{
    // Core functionality
    cortex_start_web_client,
    cortex_stop_web_client,
    cortex_get_web_client_network_info,
    
    // Web functionality (if enabled)
    ClientManager,
    Session,
    Storage,
};
```

### C FFI API

The crate provides comprehensive C FFI interfaces:

#### Core Functions
- `cortex_start_web_client`
- `cortex_stop_web_client`
- `cortex_get_network_info`
- `cortex_list_instances`
- `cortex_core_set_and_init_console_logging`
- `cortex_core_set_and_init_file_logging`

#### Web Functions (when web feature is enabled)
- `cortex_web_set_and_init_console_logging`
- `cortex_web_set_and_init_file_logging`
- Network configuration FFI functions

## Build Requirements

- Rust 2021 edition
- EasyTier dependency (GitHub: v2.4.2)
- MySQL database (for web features)
- cbindgen for C header generation

## Generated Headers

The build process automatically generates C headers:
- `include/easytier_bridge.h` - Main FFI interface

## Database Support

When web features are enabled, the crate supports:
- MySQL integration via sea-orm
- Database migrations
- Session storage
- GeoIP data storage

## Testing

The crate includes comprehensive test suites from both original crates:
- Unit tests for core functionality
- Integration tests for web features
- Database testing with mock support
- FFI testing

Run tests with:
```bash
cargo test
```

For web-specific tests:
```bash
cargo test --features web
```

## Architecture

The unified crate maintains a clean separation between core and web functionality:

```
src/
├── lib.rs                    # Main library with unified FFI
├── logging.rs               # Shared logging functionality
├── stun_wrapper.rs          # Core STUN functionality
├── easytier_web_client.rs   # Core web client functionality
├── client_manager/          # Web client management (conditional)
├── db/                      # Database integration (conditional)
├── config.rs               # Web configuration (conditional)
├── config_srv.rs           # Web config server (conditional)
└── network_config_srv_ffi.rs # Web FFI functions (conditional)
```

## Migration from Separate Crates

If you were previously using the separate crates:

### From `cortex-easytier-core`
```rust
// Old
use cortex_easytier_core::*;

// New
use cortex_easytier_bridge::*;
```

### From `cortex-easytier-web`
```rust
// Old
use cortex_easytier_web::*;

// New
use cortex_easytier_bridge::*;
// Ensure web features are enabled (default)
```

## License

MIT License

## Authors

Cortex Team