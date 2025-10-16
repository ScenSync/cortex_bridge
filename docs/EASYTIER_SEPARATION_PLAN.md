# EasyTier Bridge Separation Plan

## Executive Summary

This document outlines the plan to separate the monolithic `easytier_bridge` crate into 4 focused crates, along with necessary database schema changes and integration updates for both `cortex_agent` and `cortex_server`.

**Current Problem**: 
- Monolithic crate mixing device client, config server, and network gateway functionality
- TOML string construction instead of using EasyTier's builder API

**Target Goal**:
- Clean separation of concerns
- Each crate has a single responsibility
- Keep ONE network per device (simplified model)
- Use EasyTier's builder API directly

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         cortex_server                                │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ easytier_network_gateway                                     │  │
│  │  FFI: start_easytier_core() → Runs server VPN gateway       │  │
│  │  Listener: tcp://0.0.0.0:11010, udp://0.0.0.0:11011         │  │
│  │  Purpose: Server acts as VPN relay/gateway for devices      │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ easytier_config_server                                       │  │
│  │  FFI: create_network_config_service_singleton()              │  │
│  │  Listener: tcp://0.0.0.0:XXXXX (separate from gateway)      │  │
│  │  Purpose: Manage device connections, send configs to devices│  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                      │
│                     MySQL Database (devices table)                  │
└─────────────────────────────────────────────────────────────────────┘
                            │
                            │ Heartbeat + RPC Commands
                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         cortex_agent                                 │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │ easytier_device_client                                       │  │
│  │  FFI: cortex_start_web_client() → WebClient config mode     │  │
│  │  Purpose: Connect to config server, run local network       │  │
│  │  Supports: ONE network instance per device                  │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Part 1: Crate Separation

### **1.1 Crate: `easytier_common`**

**Location**: `cortex_server/easytier_bridge/easytier_common/`

**Purpose**: Shared utilities and types

**Dependencies**:
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"
thiserror = "1.0"
anyhow = "1.0"
libc = "0.2"
```

**Files**:
```
easytier_common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── logging.rs          # Logging initialization (from current logging.rs)
    ├── ffi_utils.rs        # FFI helper functions
    └── error.rs            # Common error types
```

**Exports**:
```rust
// Logging
pub fn set_and_init_console_logging(level: &str, module_name: &str);
pub fn set_and_init_file_logging(level: &str, module_name: &str, log_path: &str);
pub fn init_panic_recovery();

// FFI utilities
pub fn c_str_to_string(c_str: *const c_char) -> Result<String, &'static str>;
pub fn set_error_msg(msg: &str);
pub extern "C" fn get_error_msg() -> *const c_char;
pub extern "C" fn free_c_char(s: *mut c_char);

// Macros: debug!, info!, warn!, error!, trace!
```

---

### **1.2 Crate: `easytier_device_client`**

**Location**: `cortex_server/easytier_bridge/easytier_device_client/`

**Purpose**: Device-side web client for connecting to config server

**Dependencies**:
```toml
[dependencies]
easytier = { git = "https://github.com/EasyTier/EasyTier", tag = "v2.4.2" }
easytier_common = { path = "../easytier_common" }

tokio = { version = "1.0", features = ["full"] }
uuid = { version = "1.0", features = ["v4"] }
url = "2.5"
gethostname = "0.4.3"
once_cell = "1.19"
```

**Files**:
```
easytier_device_client/
├── Cargo.toml
├── cbindgen.toml
├── build.rs
└── src/
    ├── lib.rs              # FFI exports + client logic
    ├── web_client.rs       # WebClient wrapper (from easytier_web_client.rs)
    └── stun_wrapper.rs     # MockStunInfoCollectorWrapper
```

**Key FFI Functions**:
```rust
// Start web client in config mode
#[no_mangle]
pub unsafe extern "C" fn cortex_start_web_client(
    config_server_url: *const c_char,
    organization_id: *const c_char,    // NEW: explicit org_id
    machine_id: *const c_char,
) -> c_int;

// Stop web client
#[no_mangle]
pub extern "C" fn cortex_stop_web_client(
    instance_name: *const c_char
) -> c_int;

// Get network info
#[no_mangle]
pub unsafe extern "C" fn cortex_get_web_client_network_info(
    instance_name: *const c_char,
    info: *mut *const CortexNetworkInfo,
) -> c_int;

// List running instances
#[no_mangle]
pub unsafe extern "C" fn cortex_list_web_client_instances(
    instances: *mut *const *const c_char,
    max_count: c_int,
) -> c_int;
```

**C Header** (`include/easytier_device_client.h`):
```c
// Start web client
int cortex_start_web_client(
    const char* config_server_url,
    const char* organization_id,
    const char* machine_id
);

// Stop web client
int cortex_stop_web_client(const char* instance_name);

// Get network info
int cortex_get_web_client_network_info(
    const char* instance_name,
    CortexNetworkInfo** info
);
```

---

### **1.3 Crate: `easytier_config_server`**

**Location**: `cortex_server/easytier_bridge/easytier_config_server/`

**Purpose**: Server-side device connection manager and config distributor

**Dependencies**:
```toml
[dependencies]
easytier = { git = "https://github.com/EasyTier/EasyTier", tag = "v2.4.2" }
easytier_common = { path = "../easytier_common" }

tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
sea-orm = { version = "1.1", features = ["sqlx-mysql", "runtime-tokio-rustls", "macros"] }
sea-orm-migration = { version = "1.1" }
maxminddb = { version = "0.24", optional = true }
urlencoding = "2.1"
uuid = { version = "1.0", features = ["v4"] }
dashmap = "5.5"
url = "2.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
once_cell = "1.19"

[features]
default = ["geoip"]
geoip = ["maxminddb"]
```

**Files**:
```
easytier_config_server/
├── Cargo.toml
├── cbindgen.toml
├── build.rs
└── src/
    ├── lib.rs                          # FFI exports
    ├── ffi.rs                          # network_config_srv_ffi.rs
    ├── config.rs                       # Timezone, GeoIP config
    ├── config_srv.rs                   # NetworkConfigService
    ├── client_manager/
    │   ├── mod.rs                      # ClientManager
    │   ├── session.rs                  # Session + RPC handling
    │   └── storage.rs                  # In-memory storage
    └── db/
        ├── mod.rs
        ├── connection.rs
        ├── entities/
        │   ├── mod.rs
        │   ├── devices.rs              # Device entity (network fields included)
        │   └── organizations.rs
        └── migrations/
            ├── mod.rs
            ├── m20240101_000002_create_devices_table.rs
            ├── m20240101_000005_create_organizations_table.rs
            └── m20240101_000008_update_device_status_enum.rs
```

**Key Changes from Current Code**:
1. Keep network fields in `devices` table (ONE network per device)
2. Update `config_srv.rs` to use `devices.network_config` directly
3. Update `session.rs::run_network_on_start()` to handle single network per device

**FFI Functions** (`include/easytier_config_server.h`):
```c
// Create config server singleton
bool create_network_config_service_singleton(
    const char* db_url,
    const char* geoip_path,
    char** err_msg
);

// Start config server listener
bool network_config_service_singleton_start(
    const char* protocol,
    uint16_t port,
    char** err_msg
);

// Destroy config server
bool destroy_network_config_service_singleton(char** err_msg);

// List devices for an organization
bool network_config_service_list_devices(
    const char* org_id,
    char** result_json_out,
    char** err_msg
);

// Run network instance on device
bool network_config_service_run_network_instance(
    const char* org_id,
    const char* device_id,
    const char* config_json,
    char** inst_id_out,
    char** err_msg
);

// Collect network info from device
bool network_config_service_collect_one_network_info(
    const char* org_id,
    const char* device_id,
    const char* inst_id,
    char** result_json_out,
    char** err_msg
);

// Remove network instance
bool network_config_service_remove_network_instance(
    const char* org_id,
    const char* device_id,
    const char* inst_id,
    char** err_msg
);

// Update network state (enable/disable)
bool network_config_service_update_network_state(
    const char* org_id,
    const char* device_id,
    const char* inst_id,
    bool disabled,
    char** err_msg
);
```

---

### **1.4 Crate: `easytier_network_gateway`**

**Location**: `cortex_server/easytier_bridge/easytier_network_gateway/`

**Purpose**: Server-side EasyTier core wrapper (server acts as VPN gateway)

**Dependencies**:
```toml
[dependencies]
easytier = { git = "https://github.com/EasyTier/EasyTier", tag = "v2.4.2" }
easytier_common = { path = "../easytier_common" }

tokio = { version = "1.0", features = ["full"] }
uuid = { version = "1.0", features = ["v4"] }
url = "2.5"
once_cell = "1.19"
```

**Files**:
```
easytier_network_gateway/
├── Cargo.toml
├── cbindgen.toml
├── build.rs
└── src/
    ├── lib.rs              # FFI exports
    └── core_wrapper.rs     # EasyTier core management (IMPROVED)
```

**Key Improvement**: Use Builder API instead of TOML strings

**FFI Functions** (`include/easytier_network_gateway.h`):
```c
// Simplified config structure for server gateway
typedef struct {
    const char* instance_name;
    const char* network_name;
    const char* network_secret;
    
    // IP configuration
    int dhcp;                    // 0 = false, 1 = true
    const char* ipv4;            // Optional
    const char* ipv6;            // Optional
    
    // Listeners
    const char** listener_urls;
    int listener_urls_count;
    
    // Peers (for P2P mode)
    const char** peer_urls;
    int peer_urls_count;
    
    // RPC configuration
    int rpc_port;
    
    // Flags
    int enable_encryption;
    int enable_ipv6;
    int mtu;
    int latency_first;
    int private_mode;            // 0 = P2P, 1 = Private
    // ... other flags
} EasyTierCoreConfig;

// Start server gateway
int start_easytier_core(const EasyTierCoreConfig* config);

// Stop server gateway
int stop_easytier_core(const char* instance_name);

// Get gateway status
int get_easytier_core_status(
    const char* instance_name,
    char** status_json_out
);
```

**Implementation Changes**:
```rust
// OLD: Build TOML string (lines 268-420 in current code)
let toml_config = format!(r#"instance_name = "{}" ..."#, ...);
let cfg = TomlConfigLoader::new_from_str(&toml_config)?;

// NEW: Use Builder API
pub unsafe extern "C" fn start_easytier_core(
    core_config: *const EasyTierCoreConfig
) -> c_int {
    let config = &*core_config;
    
    // Parse all parameters first
    let instance_name = c_str_to_string(config.instance_name)?;
    let network_name = c_str_to_string(config.network_name)?;
    let network_secret = c_str_to_string(config.network_secret)?;
    
    // Create config using builder API
    let mut cfg = TomlConfigLoader::default();
    
    // Set instance name
    cfg.set_inst_name(instance_name.clone());
    
    // Set network identity
    cfg.set_network_identity(NetworkIdentity::new(
        network_name,
        network_secret,
    ));
    
    // Set DHCP
    cfg.set_dhcp(config.dhcp != 0);
    
    // Set IP addresses
    if !config.ipv4.is_null() {
        if let Ok(ipv4_str) = c_str_to_string(config.ipv4) {
            if !ipv4_str.is_empty() {
                cfg.set_ipv4(Some(ipv4_str.parse()?));
            }
        }
    }
    
    if !config.ipv6.is_null() {
        if let Ok(ipv6_str) = c_str_to_string(config.ipv6) {
            if !ipv6_str.is_empty() {
                cfg.set_ipv6(Some(ipv6_str.parse()?));
            }
        }
    }
    
    // Set listeners
    let listener_urls = parse_string_array(
        config.listener_urls,
        config.listener_urls_count,
    )?;
    cfg.set_listeners(
        listener_urls.iter()
            .map(|s| s.parse().unwrap())
            .collect()
    );
    
    // Set peers (for P2P mode)
    let peer_urls = parse_string_array(
        config.peer_urls,
        config.peer_urls_count,
    )?;
    let peers: Vec<PeerConfig> = peer_urls
        .iter()
        .map(|url| PeerConfig { uri: url.parse().unwrap() })
        .collect();
    cfg.set_peers(peers);
    
    // Set RPC portal
    let rpc_addr = format!("0.0.0.0:{}", config.rpc_port).parse()?;
    cfg.set_rpc_portal(rpc_addr);
    
    // Set flags
    let mut flags = cfg.get_flags();
    flags.enable_encryption = config.enable_encryption != 0;
    flags.enable_ipv6 = config.enable_ipv6 != 0;
    flags.mtu = if config.mtu <= 0 { 1380 } else { config.mtu as u32 };
    flags.latency_first = config.latency_first != 0;
    flags.private_mode = config.private_mode != 0;
    // ... set other flags
    cfg.set_flags(flags);
    
    // Create and start instance
    let mut instance = NetworkInstance::new(cfg, ConfigSource::FFI);
    let _event_subscriber = instance.start()
        .map_err(|e| {
            error!("Failed to start network instance: {}", e);
            set_error_msg(&format!("failed to start: {}", e));
        })?;
    
    // Store instance
    CLIENT_INSTANCES.lock().unwrap()
        .insert(instance_name.clone(), instance);
    
    0 // Success
}
```

---

### **1.5 Crate: `easytier_config_server`**

**Critical Database Schema Change Required**

**Current Schema (Keeping as-is)**:
```sql
-- devices table (ONE network per device - simplified model)
CREATE TABLE devices (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    serial_number VARCHAR(100) NOT NULL UNIQUE,
    device_type ENUM('robot', 'edge') NOT NULL,
    model VARCHAR(100),
    status ENUM('pending', 'rejected', 'online', 'offline', 'busy', 'maintenance', 'disabled') NOT NULL DEFAULT 'pending',
    capabilities JSON,
    organization_id CHAR(36),
    scenario_id INT UNSIGNED,
    last_heartbeat TIMESTAMP,
    robot_type_id CHAR(36),
    
    -- Network fields (ONE network per device)
    network_instance_id CHAR(36) UNIQUE,
    network_config JSON,
    network_disabled BOOLEAN,
    network_create_time TIMESTAMP,
    network_update_time TIMESTAMP,
    virtual_ip INT UNSIGNED,
    virtual_ip_network_length TINYINT UNSIGNED,
    
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_devices_organization_id (organization_id),
    INDEX idx_devices_scenario_id (scenario_id),
    INDEX idx_devices_robot_type_id (robot_type_id),
    INDEX idx_devices_network_instance_id (network_instance_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL ON UPDATE CASCADE
);
```

**Note**: We are NOT creating a separate `device_networks` table. The simplified model keeps one network configuration per device stored directly in the `devices` table. No database migration is needed.

---

## Part 2: Code Changes Required

### **2.1 `config_srv.rs` Implementation (ONE network per device)**

**Key implementation**: All network operations use the `devices` table directly. Each device can have ONE network configuration stored in the network fields.

**Example - Store network configuration**:
```rust
// Update device with network config
use crate::db::entities::devices;

let existing_device = devices::Entity::find_by_id(device_id.to_string())
    .one(db.orm())
    .await?
    .ok_or_else(|| anyhow::anyhow!("Device not found"))?;

let mut active_model: devices::ActiveModel = existing_device.into();
active_model.network_instance_id = Set(Some(inst_id.to_string()));
active_model.network_config = Set(Some(serde_json::to_value(&config)?));
active_model.network_disabled = Set(Some(false));
active_model.update(db.orm()).await?;
```

**Example - Remove network configuration**:
```rust
// Clear network fields
let mut active_model: devices::ActiveModel = device.into();
active_model.network_instance_id = Set(None);
active_model.network_config = Set(None);
active_model.network_disabled = Set(None);
active_model.update(db.orm()).await?;
```

---

### **2.2 `session.rs::run_network_on_start()` (ONE network per device)**

**Key implementation**: Check device status, then check if device has a network config (single network).

**Simplified Code**:
```rust
// Query device with network config
let device = devices::Entity::find()
    .filter(devices::Column::OrganizationId.eq(organization_id))
    .filter(devices::Column::Id.eq(device_id.to_string()))
    .one(storage.db().orm())
    .await?;

let Some(device) = device else {
    crate::warn!("Device not found: {}", device_id);
    return;
};

// Only approved devices can run networks
if !device.status.is_approved() {
    return;
}

// Check if device has a network config and it's not disabled
let network_config_opt = if device.network_disabled == Some(true) {
    None
} else if let (Some(inst_id), Some(config_json)) = 
    (&device.network_instance_id, &device.network_config) 
{
    Some((inst_id.clone(), config_json.clone()))
} else {
    None
};

// Process single network configuration if exists
if let Some((instance_id, config_json)) = network_config_opt {
    if !running_inst_ids.contains(&instance_id) {
        let network_config: NetworkConfig = 
            serde_json::from_value(config_json)?;
        
        // Send RPC to device
        let ret = rpc_client
            .run_network_instance(
                BaseController::default(),
                RunNetworkInstanceRequest {
                    inst_id: Some(instance_id.into()),
                    config: Some(network_config),
                },
            )
            .await;
    }
}
```

---

### **2.3 `devices.rs` Entity (No Changes Needed)**

**Keep Network Fields** (ONE network per device):
```rust
// src/db/entities/devices.rs

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "devices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Char(Some(36))")]
    pub id: String,
    
    #[sea_orm(column_type = "Text")]
    pub name: String,
    
    #[sea_orm(unique, column_type = "Text")]
    pub serial_number: String,
    
    pub device_type: DeviceType,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub model: Option<String>,
    
    #[sea_orm(default_value = "pending")]
    pub status: DeviceStatus,
    
    #[sea_orm(column_type = "Json", nullable)]
    pub capabilities: Option<serde_json::Value>,
    
    #[sea_orm(column_type = "Char(Some(36))", nullable)]
    pub organization_id: Option<String>,
    
    #[sea_orm(nullable)]
    pub scenario_id: Option<u32>,
    
    pub last_heartbeat: Option<DateTimeWithTimeZone>,
    
    #[sea_orm(column_type = "Char(Some(36))", nullable)]
    pub robot_type_id: Option<String>,
    
    // ✅ KEEP all network fields (ONE network per device):
    #[sea_orm(unique, column_type = "Char(Some(36))", nullable)]
    pub network_instance_id: Option<String>,
    
    #[sea_orm(column_type = "Json", nullable)]
    pub network_config: Option<serde_json::Value>,
    
    #[sea_orm(nullable)]
    pub network_disabled: Option<bool>,
    
    #[sea_orm(nullable)]
    pub network_create_time: Option<DateTimeWithTimeZone>,
    
    #[sea_orm(nullable)]
    pub network_update_time: Option<DateTimeWithTimeZone>,
    
    #[sea_orm(nullable)]
    pub virtual_ip: Option<u32>,
    
    #[sea_orm(nullable)]
    pub virtual_ip_network_length: Option<u8>,
    
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

// Simple relation - no device_networks table
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organizations::Entity",
        from = "Column::OrganizationId",
        to = "super::organizations::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Organizations,
}

impl Related<super::organizations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organizations.def()
    }
}
```

---

## Part 3: Integration Changes

### **3.1 cortex_agent Changes**

**Current Python Code** (hypothetical):
```python
# OLD: Single web client call
from ctypes import CDLL, c_char_p

lib = CDLL("./network/lib/libeasytier_bridge.so")

# Start web client
config_server_url = "tcp://server.com:XXXXX/org-token"
machine_id = str(uuid.uuid4())
result = lib.cortex_start_web_client(
    c_char_p(config_server_url.encode()),
    c_char_p(machine_id.encode())
)
```

**New Python Code**:
```python
# NEW: Use easytier_device_client library
from ctypes import CDLL, c_char_p, c_int

lib = CDLL("./network/lib/libeasytier_device_client.so")

# Define FFI signatures
lib.cortex_start_web_client.argtypes = [c_char_p, c_char_p, c_char_p]
lib.cortex_start_web_client.restype = c_int

# Start web client with explicit org_id
config_server_url = "tcp://server.com:XXXXX"  # No token in URL
organization_id = "org-uuid-123"              # Explicit org_id
machine_id = str(uuid.uuid4())                # Persistent device ID

result = lib.cortex_start_web_client(
    c_char_p(config_server_url.encode()),
    c_char_p(organization_id.encode()),
    c_char_p(machine_id.encode())
)

if result != 0:
    # Get error message
    err_msg = lib.cortex_get_error_msg()
    print(f"Failed to start web client: {err_msg}")
```

**Changes to `cortex_agent/network/easytier_client.py`**:
```python
class EasyTierClient:
    def __init__(self, config_server_url: str, organization_id: str, machine_id: str):
        self.lib = CDLL("./network/lib/libeasytier_device_client.so")
        self.config_server_url = config_server_url
        self.organization_id = organization_id
        self.machine_id = machine_id
        self.instance_name = None
    
    def start(self) -> bool:
        """Start web client connection to config server"""
        result = self.lib.cortex_start_web_client(
            c_char_p(self.config_server_url.encode()),
            c_char_p(self.organization_id.encode()),
            c_char_p(self.machine_id.encode())
        )
        
        if result == 0:
            # Instance name is the org_id for tracking
            self.instance_name = self.organization_id
            return True
        
        return False
    
    def stop(self) -> bool:
        """Stop web client"""
        if not self.instance_name:
            return False
        
        result = self.lib.cortex_stop_web_client(
            c_char_p(self.instance_name.encode())
        )
        return result == 0
    
    def get_network_info(self) -> dict:
        """Get network information for running instances"""
        # Implementation...
        pass
```

---

### **3.2 cortex_server Changes**

**Go Integration Updates**:

**File**: `cortex_server/internal/easytier/gateway_service.go` (NEW)
```go
package easytier

// #cgo LDFLAGS: -L${SRCDIR}/../../easytier_bridge/easytier_network_gateway/target/debug -leasytier_network_gateway
// #include "../../easytier_bridge/easytier_network_gateway/include/easytier_network_gateway.h"
import "C"

type GatewayService struct {
    instanceName string
    config       *GatewayConfig
}

type GatewayConfig struct {
    InstanceName    string
    NetworkName     string
    NetworkSecret   string
    DHCP            bool
    IPv4            string
    ListenerURLs    []string
    PeerURLs        []string
    RPCPort         int
    EnableEncryption bool
    PrivateMode     bool
}

func NewGatewayService(config *GatewayConfig) *GatewayService {
    return &GatewayService{
        instanceName: config.InstanceName,
        config:       config,
    }
}

func (g *GatewayService) Start() error {
    // Convert Go config to C struct
    cConfig := C.EasyTierCoreConfig{
        instance_name: C.CString(g.config.InstanceName),
        network_name:  C.CString(g.config.NetworkName),
        network_secret: C.CString(g.config.NetworkSecret),
        dhcp: boolToInt(g.config.DHCP),
        rpc_port: C.int(g.config.RPCPort),
        private_mode: boolToInt(g.config.PrivateMode),
        // ... set all fields
    }
    defer freeCStrings(/* all C strings */)
    
    // Convert listener URLs
    cListeners := make([]*C.char, len(g.config.ListenerURLs))
    for i, url := range g.config.ListenerURLs {
        cListeners[i] = C.CString(url)
    }
    cConfig.listener_urls = &cListeners[0]
    cConfig.listener_urls_count = C.int(len(cListeners))
    
    // Start gateway
    result := C.start_easytier_core(&cConfig)
    if result != 0 {
        errMsg := C.GoString(C.cortex_get_error_msg())
        return fmt.Errorf("failed to start gateway: %s", errMsg)
    }
    
    return nil
}

func (g *GatewayService) Stop() error {
    cName := C.CString(g.instanceName)
    defer C.free(unsafe.Pointer(cName))
    
    result := C.stop_easytier_core(cName)
    if result != 0 {
        return fmt.Errorf("failed to stop gateway")
    }
    return nil
}
```

**File**: `cortex_server/internal/easytier/config_server_service.go` (NEW)
```go
package easytier

// #cgo LDFLAGS: -L${SRCDIR}/../../easytier_bridge/easytier_config_server/target/debug -leasytier_config_server
// #include "../../easytier_bridge/easytier_config_server/include/easytier_config_server.h"
import "C"

type ConfigServerService struct {
    dbURL     string
    geoipPath string
    protocol  string
    port      uint16
}

func NewConfigServerService(dbURL, geoipPath, protocol string, port uint16) *ConfigServerService {
    return &ConfigServerService{
        dbURL:     dbURL,
        geoipPath: geoipPath,
        protocol:  protocol,
        port:      port,
    }
}

func (c *ConfigServerService) Initialize() error {
    var errMsg *C.char
    defer func() {
        if errMsg != nil {
            C.free_c_char(errMsg)
        }
    }()
    
    cDBURL := C.CString(c.dbURL)
    defer C.free(unsafe.Pointer(cDBURL))
    
    cGeoipPath := C.CString(c.geoipPath)
    defer C.free(unsafe.Pointer(cGeoipPath))
    
    success := C.create_network_config_service_singleton(
        cDBURL,
        cGeoipPath,
        &errMsg,
    )
    
    if !success {
        return fmt.Errorf("failed to create config server: %s", C.GoString(errMsg))
    }
    
    return nil
}

func (c *ConfigServerService) Start() error {
    var errMsg *C.char
    defer func() {
        if errMsg != nil {
            C.free_c_char(errMsg)
        }
    }()
    
    cProtocol := C.CString(c.protocol)
    defer C.free(unsafe.Pointer(cProtocol))
    
    success := C.network_config_service_singleton_start(
        cProtocol,
        C.ushort(c.port),
        &errMsg,
    )
    
    if !success {
        return fmt.Errorf("failed to start config server: %s", C.GoString(errMsg))
    }
    
    return nil
}

// List devices for organization
func (c *ConfigServerService) ListDevices(orgID string) ([]DeviceInfo, error) {
    var resultJSON *C.char
    var errMsg *C.char
    defer func() {
        if resultJSON != nil {
            C.free_c_char(resultJSON)
        }
        if errMsg != nil {
            C.free_c_char(errMsg)
        }
    }()
    
    cOrgID := C.CString(orgID)
    defer C.free(unsafe.Pointer(cOrgID))
    
    success := C.network_config_service_list_devices(
        cOrgID,
        &resultJSON,
        &errMsg,
    )
    
    if !success {
        return nil, fmt.Errorf("failed to list devices: %s", C.GoString(errMsg))
    }
    
    // Parse JSON result
    var devices []DeviceInfo
    if err := json.Unmarshal([]byte(C.GoString(resultJSON)), &devices); err != nil {
        return nil, err
    }
    
    return devices, nil
}

// Run network instance on device
func (c *ConfigServerService) RunNetworkInstance(
    orgID, deviceID string,
    config NetworkConfig,
) (string, error) {
    var instIDOut *C.char
    var errMsg *C.char
    defer func() {
        if instIDOut != nil {
            C.free_c_char(instIDOut)
        }
        if errMsg != nil {
            C.free_c_char(errMsg)
        }
    }()
    
    cOrgID := C.CString(orgID)
    defer C.free(unsafe.Pointer(cOrgID))
    
    cDeviceID := C.CString(deviceID)
    defer C.free(unsafe.Pointer(cDeviceID))
    
    configJSON, _ := json.Marshal(config)
    cConfigJSON := C.CString(string(configJSON))
    defer C.free(unsafe.Pointer(cConfigJSON))
    
    success := C.network_config_service_run_network_instance(
        cOrgID,
        cDeviceID,
        cConfigJSON,
        &instIDOut,
        &errMsg,
    )
    
    if !success {
        return "", fmt.Errorf("failed to run network: %s", C.GoString(errMsg))
    }
    
    return C.GoString(instIDOut), nil
}
```

**Update Main Service** (`cortex_server/internal/service/easytier_service.go`):
```go
type EasyTierService struct {
    gateway      *easytier.GatewayService
    configServer *easytier.ConfigServerService
}

func NewEasyTierService(cfg *config.Config) *EasyTierService {
    // Initialize gateway (server's own network)
    gatewayConfig := &easytier.GatewayConfig{
        InstanceName:    "cortex-server-gateway",
        NetworkName:     cfg.EasyTier.NetworkName,
        NetworkSecret:   cfg.EasyTier.NetworkSecret,
        DHCP:            false,
        IPv4:            "10.144.144.1",  // Server's VPN IP
        ListenerURLs: []string{
            "tcp://0.0.0.0:11010",
            "udp://0.0.0.0:11011",
            "ws://0.0.0.0:11012",
        },
        PeerURLs:         []string{},  // Server is the gateway
        RPCPort:          15888,
        EnableEncryption: true,
        PrivateMode:      true,        // Server creates the network
    }
    
    // Initialize config server (for device management)
    configServer := easytier.NewConfigServerService(
        cfg.Database.DSN,
        cfg.EasyTier.GeoIPPath,
        "tcp",
        11020,  // Different port from gateway
    )
    
    return &EasyTierService{
        gateway:      easytier.NewGatewayService(gatewayConfig),
        configServer: configServer,
    }
}

func (s *EasyTierService) Start() error {
    // Start gateway first
    if err := s.gateway.Start(); err != nil {
        return fmt.Errorf("failed to start gateway: %w", err)
    }
    
    // Start config server
    if err := s.configServer.Initialize(); err != nil {
        return fmt.Errorf("failed to initialize config server: %w", err)
    }
    
    if err := s.configServer.Start(); err != nil {
        return fmt.Errorf("failed to start config server: %w", err)
    }
    
    return nil
}
```

---

## Part 4: Directory Structure

### **4.1 Final Directory Layout**

```
cortex_server/easytier_bridge/
├── easytier_common/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── cbindgen.toml
│   ├── include/
│   │   └── easytier_common.h
│   └── src/
│       ├── lib.rs
│       ├── logging.rs
│       ├── ffi_utils.rs
│       └── error.rs
│
├── easytier_device_client/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── cbindgen.toml
│   ├── include/
│   │   └── easytier_device_client.h
│   ├── tests/
│   │   └── test_web_client.rs
│   └── src/
│       ├── lib.rs
│       ├── web_client.rs
│       └── stun_wrapper.rs
│
├── easytier_config_server/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── cbindgen.toml
│   ├── include/
│   │   └── easytier_config_server.h
│   ├── resources/
│   │   └── geoip2-cn.mmdb
│   ├── tests/
│   │   ├── common/
│   │   │   └── mod.rs
│   │   ├── test_client_manager.rs
│   │   ├── test_device_status_updates.rs
│   │   ├── test_multiple_networks.rs    # NEW
│   │   └── test_cleanup.rs
│   └── src/
│       ├── lib.rs
│       ├── ffi.rs
│       ├── config.rs
│       ├── config_srv.rs
│       ├── client_manager/
│       │   ├── mod.rs
│       │   ├── session.rs
│       │   └── storage.rs
│       └── db/
│           ├── mod.rs
│           ├── connection.rs
│           ├── entities/
│           │   ├── mod.rs
│           │   ├── devices.rs          # Device entity (network fields included)
│           │   └── organizations.rs
│           └── migrations/
│               ├── mod.rs
│               ├── m20240101_000002_create_devices_table.rs
│               ├── m20240101_000005_create_organizations_table.rs
│               └── m20240101_000008_update_device_status_enum.rs
│
├── easytier_network_gateway/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── cbindgen.toml
│   ├── include/
│   │   └── easytier_network_gateway.h
│   ├── tests/
│   │   ├── launcher_test.rs
│   │   └── test_gateway.rs            # NEW
│   └── src/
│       ├── lib.rs
│       └── core_wrapper.rs            # From easytier_core_ffi.rs (IMPROVED)
│
├── Cargo.toml                         # Workspace manifest
├── README.md                          # Updated documentation
└── build_all.sh                       # NEW: Build all crates
```

### **4.2 Workspace Cargo.toml**

```toml
[workspace]
members = [
    "easytier_common",
    "easytier_device_client",
    "easytier_config_server",
    "easytier_network_gateway",
]
resolver = "2"

[workspace.dependencies]
easytier = { git = "https://github.com/EasyTier/EasyTier", tag = "v2.4.2" }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
uuid = { version = "1.0", features = ["v4"] }
url = "2.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
once_cell = "1.19"

[profile.dev]
opt-level = 1
incremental = true
codegen-units = 256

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

---

## Part 5: Database Migration Strategy

### **5.1 No Migration Needed**

**Database Schema**: We are keeping the existing `devices` table structure with network fields included. No schema changes are required.

**Migration Order**:
```
1. m20240101_000002_create_devices_table.rs        (existing)
2. m20240101_000005_create_organizations_table.rs  (existing)
3. m20240101_000008_update_device_status_enum.rs   (existing)
```

**Note**: The simplified one-network-per-device model keeps all network configuration in the `devices` table. No data migration is required.

---

## Part 6: Build and Deployment

### **6.1 Build Script** (`build_all.sh`)

```bash
#!/bin/bash
set -e

echo "Building all EasyTier crates..."

# Build order (respect dependencies)
cd easytier_common
echo "Building easytier_common..."
cargo build
cbindgen --config cbindgen.toml --output include/easytier_common.h
cd ..

cd easytier_device_client
echo "Building easytier_device_client..."
cargo build
cbindgen --config cbindgen.toml --output include/easytier_device_client.h
cd ..

cd easytier_network_gateway
echo "Building easytier_network_gateway..."
cargo build
cbindgen --config cbindgen.toml --output include/easytier_network_gateway.h
cd ..

cd easytier_config_server
echo "Building easytier_config_server..."
cargo build
cbindgen --config cbindgen.toml --output include/easytier_config_server.h
cd ..

echo "✓ All crates built successfully"
echo ""
echo "Generated headers:"
ls -la */include/*.h
```

### **6.2 cortex_agent Deployment**

**Files Needed**:
- `libeasytier_device_client.so` (Linux) or `.dylib` (macOS) or `.dll` (Windows)
- `easytier_device_client.h` (for ctypes integration)

**Installation**:
```bash
# In cortex_agent/
mkdir -p network/lib
cp ../cortex_server/easytier_bridge/easytier_device_client/target/release/libeasytier_device_client.so \
   network/lib/
```

### **6.3 cortex_server Deployment**

**Files Needed**:
- `libeasytier_network_gateway.so`
- `libeasytier_config_server.so`
- Both header files

**CGo Configuration** (`cortex_server/internal/easytier/`):
```go
// gateway_service.go
// #cgo LDFLAGS: -L${SRCDIR}/../../easytier_bridge/easytier_network_gateway/target/release -leasytier_network_gateway
// #include "../../easytier_bridge/easytier_network_gateway/include/easytier_network_gateway.h"

// config_server_service.go
// #cgo LDFLAGS: -L${SRCDIR}/../../easytier_bridge/easytier_config_server/target/release -leasytier_config_server
// #include "../../easytier_bridge/easytier_config_server/include/easytier_config_server.h"
```

---

## Part 7: Testing Strategy

### **7.1 Unit Tests per Crate**

**easytier_common**:
- Logging initialization tests
- FFI utility tests
- Error handling tests

**easytier_device_client**:
- Web client creation tests
- Mock config server connection tests
- Instance management tests

**easytier_config_server**:
- ClientManager with single network per device
- Session handling with device network config
- Database operations on `devices` table
- No migration needed

**easytier_network_gateway**:
- Gateway instance creation
- Builder API usage tests
- Multi-protocol listener tests

### **7.2 Integration Tests**

**Test Scenario 1**: Device connects and receives multiple configs
```
1. Start server gateway
2. Start config server
3. Device connects via web client
4. Admin approves device
5. Admin creates Network A config
6. Admin creates Network B config
7. Verify device receives both configs
8. Verify device runs both networks locally
```

**Test Scenario 2**: Multiple devices in same organization
```
1. Device A connects (org-1)
2. Device B connects (org-1)
3. Admin creates shared network config
4. Both devices receive and run same network
5. Devices can communicate via VPN
```

---

## Part 8: Migration Checklist

### **Phase 1: Preparation** (No Breaking Changes)
- [ ] Create workspace structure
- [ ] Create `easytier_common` crate
- [ ] Move logging code to `easytier_common`
- [ ] Test `easytier_common` builds independently

### **Phase 2: Create New Crates**
- [ ] Create `easytier_device_client`
- [ ] Move web client code
- [ ] Test device client builds
- [ ] Create `easytier_network_gateway`
- [ ] Move and improve core wrapper (use builder API)
- [ ] Test gateway builds

### **Phase 3: Database Schema (No Changes)**
- [x] Keep network fields in `devices` table
- [x] No new migrations needed
- [x] Database schema remains unchanged

### **Phase 4: Config Server Refactor**
- [ ] Create `easytier_config_server` crate
- [ ] Update `config_srv.rs` to use `devices` table (no changes needed)
- [ ] Update `session.rs::run_network_on_start()` for single network per device
- [ ] Update all FFI functions
- [ ] Test config server

### **Phase 5: Integration Updates**
- [ ] Update `cortex_agent` Python code
- [ ] Update `cortex_server` Go code
- [ ] Create new Go service wrappers
- [ ] Update CGo build configuration
- [ ] Test end-to-end integration

### **Phase 6: Cleanup**
- [ ] Remove old monolithic `easytier_bridge` crate
- [ ] Update documentation
- [ ] Update CI/CD workflows
- [ ] Update deployment scripts

---

## Part 9: Risk Assessment and Mitigation

### **High Risk Items**

**Risk 1**: Database migration breaks existing data
- **Mitigation**: Test on staging, backup production, rollback plan

**Risk 2**: FFI ABI compatibility issues
- **Mitigation**: Keep same C types, test FFI boundaries thoroughly

**Risk 3**: ~~Multiple networks causing device resource exhaustion~~
- **Not applicable**: Using simplified model with ONE network per device

### **Medium Risk Items**

**Risk 4**: Breaking changes in cortex_server Go code
- **Mitigation**: Phased rollout, keep old functions during transition

**Risk 5**: cortex_agent compatibility
- **Mitigation**: Version detection, graceful fallback

---

## Part 10: Timeline Estimate

**Assuming single developer, full-time work**:

| Phase | Tasks | Duration | Dependencies |
|-------|-------|----------|--------------|
| Phase 1 | Workspace + common crate | 1 day | None |
| Phase 2 | Device client + Gateway crates | 2 days | Phase 1 |
| Phase 3 | Database migration | 1 day | None (parallel) |
| Phase 4 | Config server refactor | 3 days | Phase 2, 3 |
| Phase 5 | Integration updates | 2 days | Phase 4 |
| Phase 6 | Cleanup + documentation | 1 day | Phase 5 |
| **Total** | | **10 days** | |

**Buffer**: Add 20% = **12 days total**

---

## Part 11: Success Criteria

### **Technical Success**
- [ ] All 4 crates build independently
- [ ] All tests pass (unit + integration)
- [ ] Database supports ONE network per device (simplified model)
- [ ] cortex_agent can connect and receive network config
- [ ] cortex_server gateway runs successfully
- [ ] Config server manages devices correctly

### **Operational Success**
- [ ] Existing deployments can upgrade without data loss
- [ ] Device connections maintain during upgrade
- [ ] Network instances remain stable
- [ ] No performance regression

### **Code Quality Success**
- [ ] Each crate has < 3000 lines of code
- [ ] Clear separation of concerns
- [ ] Comprehensive test coverage (>70%)
- [ ] Documentation updated

---

## Part 12: Open Questions

**Q1**: Should we support downgrade/rollback after crate separation?
**Q2**: ~~Maximum number of networks per device?~~ (Not applicable - ONE network only)
**Q3**: Should network config have soft-delete support?
**Q4**: Network config versioning needed?
**Q5**: ~~Network priority/ordering field?~~ (Not applicable - ONE network only)

---

## Appendix A: File Mapping

| Current File | New Location | Notes |
|--------------|--------------|-------|
| `src/logging.rs` | `easytier_common/src/logging.rs` | Shared logging |
| `src/stun_wrapper.rs` | `easytier_device_client/src/stun_wrapper.rs` | Device-side only |
| `src/easytier_web_client.rs` | `easytier_device_client/src/web_client.rs` | Renamed |
| `src/easytier_core_ffi.rs` | `easytier_network_gateway/src/core_wrapper.rs` | Improved with builder API |
| `src/client_manager/` | `easytier_config_server/src/client_manager/` | Uses devices table for network config |
| `src/config_srv.rs` | `easytier_config_server/src/config_srv.rs` | Uses devices table for network config |
| `src/network_config_srv_ffi.rs` | `easytier_config_server/src/ffi.rs` | Renamed |
| `src/config.rs` | `easytier_config_server/src/config.rs` | Server config only |
| `src/db/` | `easytier_config_server/src/db/` | Database entities and migrations |

---

## Appendix B: Dependency Graph

```
easytier_common (no internal deps)
       ↑
       ├──────────────────────────┬─────────────────────────┐
       │                          │                         │
easytier_device_client    easytier_network_gateway    easytier_config_server
       ↑                          ↑                         ↑
       │                          │                         │
   cortex_agent            cortex_server              cortex_server
   (Python FFI)            (Go FFI)                   (Go FFI)
```

---

## Next Steps

**For Review**:
1. Review this plan
2. Answer open questions in Part 12
3. Approve or suggest changes

**After Approval**:
1. I'll create detailed migration scripts
2. Create file-by-file changes list
3. Begin implementation (with your permission at each phase)

**Would you like me to**:
- A) Proceed with Phase 1 (create workspace and easytier_common)?
- B) Create detailed migration SQL scripts first?
- C) Modify this plan based on your feedback?

