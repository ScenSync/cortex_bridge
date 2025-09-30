# Cortex EasyTier Bridge

A unified Rust crate that provides comprehensive EasyTier integration for Cortex applications, combining both core EasyTier functionality and web client management capabilities.

## Overview

This crate merges the functionality of two previously separate crates:
- `cortex-easytier-core`: Direct EasyTier core integration
- `cortex-easytier-web`: EasyTier Web Client Manager with MySQL storage

## Features

### Server Features (optional, enabled by default)
- EasyTier Core Server functionality
- Client Manager for handling incoming connections
- MySQL database storage integration
- Session and storage management
- GeoIP configuration support
- Network configuration service FFI
- C FFI interfaces for server functionality

### Client Features (optional, enabled by default)
- EasyTier Web Client functionality
- Connects to EasyTier servers
- C FFI interfaces for client functionality

### Shared Features (always enabled)
- Logging and panic recovery
- STUN wrapper functionality

## Feature Flags

- `default = ["server", "client"]` - Enables both server and client functionality
- `server` - Server functionality (manages clients, database, sessions)
- `client` - Client functionality (connects to servers)
- `database` - Database integration (sea-orm, sea-orm-migration) - part of server
- `websocket` - WebSocket support (part of server features)

## Usage

### Server Only (for hosting/managing clients)

```toml
[dependencies]
easytier-bridge = { version = "0.1.0", default-features = false, features = ["server"] }
```

### Client Only (for connecting to servers)

```toml
[dependencies]
easytier-bridge = { version = "0.1.0", default-features = false, features = ["client"] }
```

### Full Usage (Server + Client)

```toml
[dependencies]
easytier-bridge = "0.1.0"
```

### Rust API

```rust
use cortex_easytier_bridge::{
    // Server functionality (if server feature enabled)
    start_easytier_core,
    stop_easytier_core,
    EasyTierCoreConfig,
    ClientManager,
    Session,
    Storage,
    
    // Client functionality (if client feature enabled)
    cortex_start_web_client,
    cortex_stop_web_client,
    cortex_get_web_client_network_info,
};
```

### C FFI API

The crate provides comprehensive C FFI interfaces:

#### Server Functions (when server feature is enabled)
- `start_easytier_core`
- `stop_easytier_core`
- `cortex_web_set_and_init_console_logging`
- `cortex_web_set_and_init_file_logging`
- Network configuration FFI functions

#### Client Functions (when client feature is enabled)
- `cortex_start_web_client`
- `cortex_stop_web_client`
- `cortex_get_web_client_network_info`
- `cortex_list_web_client_instances`

## Build Requirements

- Rust 2021 edition
- Protocol Buffers compiler (`protoc`) - **Required for building**
- EasyTier dependency (GitHub: v2.4.2)
- MySQL database (for web features)
- cbindgen for C header generation

### Installing Protocol Buffers Compiler

The project requires `protoc` (Protocol Buffers compiler) to build successfully. Install it using your system's package manager:

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install protobuf-compiler
```

#### macOS (Homebrew)
```bash
brew install protobuf
```

#### Windows (Chocolatey)
```bash
choco install protoc
```

#### Manual Installation
Download the latest release from the [Protocol Buffers releases page](https://github.com/protocolbuffers/protobuf/releases) <mcreference link="https://github.com/protocolbuffers/protobuf/releases" index="0">0</mcreference> and add the `protoc` binary to your PATH.

For more information, see the [prost-build documentation](https://docs.rs/prost-build/#sourcing-protoc) <mcreference link="https://docs.rs/prost-build/#sourcing-protoc" index="1">1</mcreference>.

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

For server-specific tests:
```bash
cargo test --features server
```

For client-specific tests:
```bash
cargo test --features client
```

## Architecture

The unified crate maintains a clean separation between server and client functionality:

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