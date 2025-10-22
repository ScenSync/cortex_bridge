# Quick Start Guide

## Build Everything

```bash
# Clone the repository
git clone git@github.com:ScenSync/cortex_bridge.git
cd cortex_bridge

# Build all crates
./build_all.sh
```

**Expected Output**: All 4 crates build successfully ✅

---

## For Device Client Integration (Python/Agent)

### 1. Build Device Client Library
```bash
cargo build --release -p easytier_device_client
```

The library will be at: `target/release/libeasytier_device_client.{so,dylib,dll}`

### 2. Integration Example
```python
from ctypes import CDLL, c_char_p

# Load the library
lib = CDLL("./lib/libeasytier_device_client.so")

# Start web client
config_server_url = "tcp://server:11020"
organization_id = "org-uuid"
machine_id = "device-uuid"

result = lib.cortex_start_web_client(
    c_char_p(config_server_url.encode()),
    c_char_p(organization_id.encode()),
    c_char_p(machine_id.encode())
)
```

See: `examples/cortex_agent_integration.py` for complete example

---

## For Server Integration (Go)

### 1. Build Server Libraries
```bash
cargo build --release -p easytier_network_gateway
cargo build --release -p easytier_config_server
```

Libraries will be at:
- `target/release/libeasytier_network_gateway.{so,a}`
- `target/release/libeasytier_config_server.{so,a}`

### 2. Integration Examples
- Gateway: `examples/server_gateway_integration.go`
- Config Server: `examples/server_config_integration.go`

### 3. Database Migration
Migrations run automatically when config_server initializes.

**Verify**:
```sql
SHOW TABLES LIKE 'device_networks';
SELECT COUNT(*) FROM device_networks;
```

---

## Testing

### 1. Unit Tests
```bash
# Test all crates
cargo test --all

# Test specific crate
cargo test -p easytier_device_client
cargo test -p easytier_config_server
```

### 2. Integration Testing
See individual examples in `examples/` directory:
- Device client example
- Server gateway example
- Config server example

### 3. Verify Database Integration
After config server starts:
```sql
-- Check tables exist
SHOW TABLES LIKE 'devices';
SHOW TABLES LIKE 'organizations';

-- Verify device registration
SELECT * FROM devices;
```

---

## Quick Commands

### Build Release
```bash
cargo build --all --release
```

### Run Tests
```bash
cargo test --all
```

### Generate Headers
```bash
cd easytier_device_client
cbindgen --config cbindgen.toml --output include/easytier_device_client.h
```

### Check Database Schema
```sql
-- Verify devices table structure
SHOW CREATE TABLE devices;

-- Check device network configurations
SELECT id, name, network_instance_id, network_disabled 
FROM devices 
WHERE network_instance_id IS NOT NULL;
```

---

## Troubleshooting

### Build Fails
```bash
cargo clean
./build_all.sh
```

### Can't Find Header
```bash
find . -name "*.h" -path "*/include/*"
```

### Migration Issues
```bash
# Check migration status
cargo run -p easytier_config_server --example check_migrations

# Manually run migrations
cd easytier_config_server
sea-orm-cli migrate up
```

### Library Loading Errors
```bash
# Verify library on macOS
file target/debug/libeasytier_device_client.dylib
otool -L target/debug/libeasytier_device_client.dylib

# Verify library on Linux
file target/debug/libeasytier_device_client.so
ldd target/debug/libeasytier_device_client.so
```

---

## Documentation Index

| Document | Purpose |
|----------|---------|
| README.md | Architecture overview |
| docs/CORTEX_BRIDGE_DESIGN.md | Detailed design documentation |
| docs/QUICK_START.md | This file - quick start guide |
| docs/README_TIMEZONE.md | Timezone configuration |

---

## Project Structure

```
cortex_bridge/
├── easytier_common/          # Shared utilities
├── easytier_device_client/   # Device-side client library
├── easytier_network_gateway/ # Server VPN gateway
├── easytier_config_server/   # Device connection manager
├── examples/                 # Integration examples
├── docs/                     # Documentation
└── build_all.sh              # Build script
```

---

## Support

For questions about:
- **Architecture**: See README.md
- **Build Issues**: Check build_all.sh output
- **Integration**: See examples/ directory
- **Database**: See easytier_config_server/src/db/

