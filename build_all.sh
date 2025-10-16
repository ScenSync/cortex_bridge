#!/bin/bash

# Build all EasyTier crates in dependency order
set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Building EasyTier Bridge Workspace${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════${NC}"
echo ""

# Function to build a crate
build_crate() {
    local crate_name=$1
    echo -e "${BLUE}[BUILD]${NC} Building ${crate_name}..."
    
    if cargo build -p "$crate_name" 2>&1 | tee "/tmp/${crate_name}_build.log"; then
        echo -e "${GREEN}✓${NC} ${crate_name} built successfully"
        
        # Generate C headers
        if cd "$crate_name" && cbindgen --config cbindgen.toml --output "include/${crate_name//-/_}.h" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Generated ${crate_name//-/_}.h"
        fi
        cd - > /dev/null
        
        echo ""
        return 0
    else
        echo -e "${RED}✗${NC} ${crate_name} build failed"
        echo "See /tmp/${crate_name}_build.log for details"
        return 1
    fi
}

# Change to workspace root
cd "$(dirname "$0")"

# Build in dependency order
build_crate "easytier_common" || exit 1
build_crate "easytier_device_client" || exit 1
build_crate "easytier_network_gateway" || exit 1
build_crate "easytier_config_server" || exit 1

echo -e "${GREEN}════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}All crates built successfully!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════${NC}"
echo ""
echo "Generated headers:"
find . -name "*.h" -path "*/include/*" | while read -r header; do
    echo "  - $header"
done
echo ""

# Display library files
echo "Generated libraries:"
find target/debug -name "libeasytier_*.so" -o -name "libeasytier_*.dylib" -o -name "easytier_*.dll" | while read -r lib; do
    echo "  - $lib"
done

