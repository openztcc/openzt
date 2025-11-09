#!/bin/bash

# Build script for creating test-enabled OpenZT DLL
# This sets the required environment variables for detour testing

set -e

# Configuration paths
CONFIG_PATH="${CONFIG_PATH:-$(pwd)/detour_test_config.txt}"
SUCCESS_PATH="${SUCCESS_PATH:-$(pwd)/detour_success.txt}"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building OpenZT with detour testing enabled...${NC}"
echo "Config file path: $CONFIG_PATH"
echo "Success signal path: $SUCCESS_PATH"

# Export environment variables for the build
export DETOUR_TEST_CONFIG_PATH="$CONFIG_PATH"
export DETOUR_TEST_SUCCESS_PATH="$SUCCESS_PATH"

# Build the DLL with testing features
echo -e "${YELLOW}Running cargo build...${NC}"
cargo +nightly build \
    --lib \
    --features detour-testing \
    --target=i686-pc-windows-msvc \
    "$@"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Build successful!${NC}"
    echo "DLL location: target/i686-pc-windows-msvc/debug/openzt.dll"
    echo ""
    echo "Next steps:"
    echo "1. Generate detour list: ./generate_detour_list.sh > detours_to_test.txt"
    echo "2. Run tests: ./test_detours.sh"
else
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi