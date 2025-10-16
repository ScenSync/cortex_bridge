# EasyTier Bridge - Multi-Crate Architecture

Comprehensive EasyTier integration for Cortex, separated into focused, independent crates.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    cortex_agent (Device)                    │
│  Uses: easytier_device_client                               │
│  Purpose: Connect to config server, run local VPN networks  │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Heartbeat + Config Requests
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  cortex_server (Server)                     │
│                                                             │
│  ┌────────────────────────────────────────────────────┐   │
│  │ easytier_network_gateway                           │   │
│  │ Purpose: Server VPN gateway (relay for devices)    │   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌────────────────────────────────────────────────────┐   │
│  │ easytier_config_server                             │   │
│  │ Purpose: Manage devices, distribute configs        │   │
│  └────────────────────────────────────────────────────┘   │
│                                                             │
│                MySQL Database (device_networks)             │
└─────────────────────────────────────────────────────────────┘
```

## Crates

### 1. `easytier_common`
**Purpose**: Shared utilities and logging

**Dependencies**: Minimal (no EasyTier, no database)

**Features**:
- Logging initialization (console + file)
- FFI utility functions
- Error handling
- Panic recovery

**Usage**: Included by all other crates

---

### 2. `easytier_device_client`
**Purpose**: Device-side web client for connecting to config server

**Target**: Used by `cortex_agent` (devices)

**Features**:
- Connects to config server
- Sends heartbeat with org_id + machine_id
- Receives RPC commands from server
- Runs network instances locally on device

**FFI Functions**:
```c
// Start web client
int cortex_start_web_client(const CortexWebClient* config);

// Stop web client
int cortex_stop_web_client(const char* instance_name);

// Get network info
int cortex_get_web_client_network_info(
    const char* instance_name,
    CortexNetworkInfo** info
);
```

**Dependencies**: `easytier`, `easytier_common`

**Build**:
```bash
cd easytier_device_client
cargo build --release
# Output: target/release/libeasytier_device_client.so
```

---

### 3. `easytier_network_gateway`
**Purpose**: Server-side EasyTier gateway (VPN relay/gateway)

**Target**: Used by `cortex_server`

**Features**:
- Runs EasyTier instance on server
- Acts as VPN gateway for devices
- Supports private mode or P2P mode
- **Improved**: Uses Builder API (not TOML strings)

**FFI Functions**:
```c
// Start gateway instance
int start_easytier_core(const EasyTierCoreConfig* config);

// Stop gateway instance
int stop_easytier_core(const char* instance_name);

// Get gateway status
int get_easytier_core_status(
    const char* instance_name,
    char** status_json_out
);
```

**Key Improvement**: Uses `ConfigLoader` trait methods instead of TOML string construction
```rust
// Builder pattern (type-safe)
let cfg = TomlConfigLoader::default();
cfg.set_network_identity(NetworkIdentity::new(name, secret));
cfg.set_dhcp(dhcp);
cfg.set_listeners(listeners);
let mut flags = cfg.get_flags();
flags.enable_encryption = true;
cfg.set_flags(flags);
```

**Dependencies**: `easytier`, `easytier_common`

**Build**:
```bash
cd easytier_network_gateway
cargo build --release
# Output: target/release/libeasytier_network_gateway.so
```

---

### 4. `easytier_config_server`
**Purpose**: Device connection manager and config distributor

**Target**: Used by `cortex_server`

**Features**:
- Manages device connections via sessions
- Stores device info in MySQL
- **NEW**: Supports multiple networks per device
- Distributes network configs via RPC
- GeoIP location tracking
- Heartbeat handling

**Database Schema**:
```sql
-- devices table (device metadata only)
CREATE TABLE devices (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(100),
    status ENUM('pending', 'rejected', 'online', 'offline', ...),
    organization_id CHAR(36),
    ...
);

-- device_networks table (NEW - supports multiple networks)
CREATE TABLE device_networks (
    id INT AUTO_INCREMENT PRIMARY KEY,
    device_id CHAR(36) NOT NULL,
    network_instance_id CHAR(36) NOT NULL UNIQUE,
    network_config JSON NOT NULL,
    disabled BOOLEAN DEFAULT FALSE,
    virtual_ip INT UNSIGNED,
    virtual_ip_network_length TINYINT,
    ...
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);
```

**FFI Functions**:
```c
// Initialize config server
bool create_network_config_service_singleton(
    const char* db_url,
    const char* geoip_path,
    char** err_msg
);

// Start listener
bool network_config_service_singleton_start(
    const char* protocol,
    uint16_t port,
    char** err_msg
);

// List devices for organization
bool network_config_service_list_devices(
    const char* org_id,
    char** result_json_out,
    char** err_msg
);

// Run network instance on device (can be called multiple times per device)
bool network_config_service_run_network_instance(
    const char* org_id,
    const char* device_id,
    const char* config_json,
    char** inst_id_out,
    char** err_msg
);

// Remove network instance
bool network_config_service_remove_network_instance(
    const char* org_id,
    const char* device_id,
    const char* inst_id,
    char** err_msg
);
```

**Dependencies**: `easytier`, `easytier_common`, `sea-orm`, `maxminddb`

**Build**:
```bash
cd easytier_config_server
cargo build --release
# Output: target/release/libeasytier_config_server.so
```

---

## Building All Crates

### Quick Build

```bash
./build_all.sh
```

This will:
1. Build all 4 crates in dependency order
2. Generate C headers for each
3. Display generated libraries
4. Report any errors

### Manual Build

```bash
# Build workspace
cargo build --all --release

# Generate headers
cd easytier_common && cbindgen --config cbindgen.toml --output include/easytier_common.h
cd easytier_device_client && cbindgen --config cbindgen.toml --output include/easytier_device_client.h
cd easytier_network_gateway && cbindgen --config cbindgen.toml --output include/easytier_network_gateway.h
cd easytier_config_server && cbindgen --config cbindgen.toml --output include/easytier_config_server.h
```

### Cross-Compilation

```bash
# For ARM64 devices
cargo build --target aarch64-unknown-linux-gnu --release

# For x86_64 servers
cargo build --target x86_64-unknown-linux-gnu --release
```

---

## Development Workflow

### Adding New Features

**For device functionality** (e.g., new device capabilities):
→ Modify `easytier_device_client`

**For server gateway** (e.g., new VPN features):
→ Modify `easytier_network_gateway`

**For device management** (e.g., new approval workflows):
→ Modify `easytier_config_server`

**For shared utilities** (e.g., new logging formats):
→ Modify `easytier_common`

### Testing Changes

```bash
# Test specific crate
cargo test -p easytier_config_server

# Test with feature flags
cargo test -p easytier_config_server --features geoip

# Test all crates
cargo test --all
```

---

## Database Migrations

Migrations are managed by `sea-orm-migration` in `easytier_config_server`.

### Current Migrations

| ID | Name | Purpose |
|----|------|---------|
| 000002 | create_devices_table | Creates devices (no network fields) |
| 000005 | create_organizations_table | Creates organizations |
| 000008 | update_device_status_enum | Updates device status enum |
| 000010 | **create_device_networks_table** | **NEW: Creates device_networks** |
| 000011 | **migrate_network_data** | **NEW: Migrates data + drops old columns** |

### Running Migrations

Migrations run automatically when config_server initializes.

Manual migration:
```bash
cd easytier_config_server
sea-orm-cli migrate up
```

---

## Examples

See `examples/` directory:
- `cortex_agent_integration.py` - Python device client example
- `cortex_server_gateway.go` - Go gateway service example
- `cortex_server_config_server.go` - Go config server example

---

## Documentation

- [Separation Plan](SEPARATION_PLAN.md) - Detailed architecture plan
- [Migration Guide](MIGRATION_GUIDE.md) - Step-by-step migration instructions
- [Progress](SEPARATION_PROGRESS.md) - Implementation progress tracker

---

## Requirements

### Build Requirements

- Rust 2021 edition
- Protocol Buffers compiler (`protoc`)
- cbindgen (`cargo install cbindgen`)
- MySQL 8.0+ (for config_server)

### Runtime Requirements

**cortex_agent**:
- `libeasytier_device_client.so`
- Python 3.8+
- Network connectivity to cortex_server

**cortex_server**:
- `libeasytier_network_gateway.so`
- `libeasytier_config_server.so`
- MySQL database
- GeoIP2 database (optional)

---

## License

MIT License - See individual crate LICENSE files

## Authors

Cortex Team

---

## Changelog

### v0.2.0 (2025-10-15) - Multi-Crate Architecture

**Breaking Changes**:
- Split monolithic crate into 4 focused crates
- Database schema changed (multiple networks per device)
- FFI function signatures updated

**Improvements**:
- Builder API for gateway configuration (cleaner, type-safe)
- Support for multiple network instances per device
- Clear separation of concerns
- Reduced binary sizes for cortex_agent

**Migration**: See [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md)

### v0.1.0 - Initial monolithic release
- Combined all functionality in single crate
