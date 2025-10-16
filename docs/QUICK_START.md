# Quick Start Guide

## Build Everything

```bash
cd /Users/zhexuany/Repo/cortex/cortex_server/easytier_bridge
./build_all.sh
```

**Expected Output**: All 4 crates build successfully ✅

---

## For cortex_agent Developers

### 1. Copy Library
```bash
cp easytier_device_client/target/release/libeasytier_device_client.so \
   ../../cortex_agent/network/lib/
```

### 2. Use Python Example
See: `examples/cortex_agent_integration.py`

### 3. Key Changes
- **OLD**: `lib.cortex_start_web_client(config_url, machine_id)`
- **NEW**: `lib.cortex_start_web_client(pointer(config))`
  - `config.config_server_url` = "tcp://server:11020"
  - `config.organization_id` = "org-uuid"  
  - `config.machine_id` = "device-uuid"

---

## For cortex_server Developers

### 1. Copy Libraries
```bash
cp easytier_network_gateway/target/release/libeasytier_network_gateway.so \
   ../lib/
cp easytier_config_server/target/release/libeasytier_config_server.so \
   ../lib/
```

### 2. Use Go Examples
- Gateway: `examples/cortex_server_gateway.go`
- Config Server: `examples/cortex_server_config_server.go`

### 3. Database Migration
Migrations run automatically when config_server initializes.

**Verify**:
```sql
SHOW TABLES LIKE 'device_networks';
SELECT COUNT(*) FROM device_networks;
```

---

## Test on Staging

### 1. Start Server Components

**Terminal 1** - Gateway:
```bash
cd cortex_server
go run examples/easytier_gateway_test.go
```

**Terminal 2** - Config Server:
```bash
cd cortex_server  
go run examples/easytier_config_server_test.go
```

### 2. Connect Test Device

**Terminal 3** - Device:
```bash
cd cortex_agent
python3 examples/test_device_connection.py
```

### 3. Verify
- Check device appears in database: `SELECT * FROM devices;`
- Admin approves device: `UPDATE devices SET status='online' WHERE id='...';`
- Create network config (see examples/)
- Verify device receives config and starts network

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

### Check Database
```sql
-- After migration
SHOW CREATE TABLE device_networks;
SELECT COUNT(*) FROM device_networks;
SELECT d.name, dn.network_instance_id 
FROM devices d 
JOIN device_networks dn ON d.id = dn.device_id;
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
# Rollback
cd easytier_config_server
sea-orm-cli migrate down
```

### Import Errors
```bash
# Verify library
file target/debug/libeasytier_device_client.dylib
ldd target/debug/libeasytier_device_client.so  # Linux
```

---

## Documentation Index

| Document | Purpose |
|----------|---------|
| README.md | Architecture overview |
| SEPARATION_PLAN.md | Detailed design (1915 lines) |
| MIGRATION_GUIDE.md | Step-by-step migration |
| IMPLEMENTATION_SUMMARY.md | What was delivered |
| QUICK_START.md | This file |

---

## Contact & Support

For questions about:
- **Architecture**: See SEPARATION_PLAN.md
- **Migration**: See MIGRATION_GUIDE.md
- **Build Issues**: Check build_all.sh output
- **Integration**: See examples/ directory

