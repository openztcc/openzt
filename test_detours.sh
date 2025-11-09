#!/bin/bash

# Test Detour Script for OpenZT
# This script tests individual detours to verify their signatures work correctly

set -e

# Configuration
WINE_PREFIX="${WINE_PREFIX:-$HOME/.wine}"
ZOO_TYCOON_PATH="${ZOO_TYCOON_PATH:-/path/to/zoo.exe}"
CONFIG_FILE="${DETOUR_TEST_CONFIG:-detour_test_config.txt}"
SUCCESS_FILE="${DETOUR_TEST_SUCCESS:-detour_success.txt}"
RESULTS_DIR="${RESULTS_DIR:-test_results}"
TIMEOUT="${TIMEOUT:-30}"
DLL_PATH="${DLL_PATH:-target/i686-pc-windows-msvc/debug/openzt.dll}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check prerequisites
if ! command -v wine &> /dev/null; then
    print_error "Wine is not installed or not in PATH"
    exit 1
fi

if [ ! -f "$ZOO_TYCOON_PATH" ]; then
    print_error "Zoo Tycoon executable not found at: $ZOO_TYCOON_PATH"
    print_info "Set ZOO_TYCOON_PATH environment variable to the correct path"
    exit 1
fi

if [ ! -f "$DLL_PATH" ]; then
    print_error "OpenZT DLL not found at: $DLL_PATH"
    print_info "Build the DLL first with: cargo build --features detour-testing --target=i686-pc-windows-msvc"
    exit 1
fi

# Create results directory
mkdir -p "$RESULTS_DIR"

# Initialize result files
> "$RESULTS_DIR/successful_detours.txt"
> "$RESULTS_DIR/failed_detours.txt"
> "$RESULTS_DIR/test_log.txt"

# Check if detour list exists
if [ ! -f "detours_to_test.txt" ]; then
    print_warning "detours_to_test.txt not found. Generating list..."
    ./generate_detour_list.sh > detours_to_test.txt
fi

# Load detour list
mapfile -t DETOURS < detours_to_test.txt

if [ ${#DETOURS[@]} -eq 0 ]; then
    print_error "No detours to test found in detours_to_test.txt"
    exit 1
fi

print_info "Found ${#DETOURS[@]} detours to test"
print_info "Results will be saved to: $RESULTS_DIR"
print_info "Starting tests..."
echo ""

# Statistics
TOTAL=${#DETOURS[@]}
SUCCESS_COUNT=0
FAIL_COUNT=0
CURRENT=0

# Testing loop
for detour in "${DETOURS[@]}"; do
    CURRENT=$((CURRENT + 1))
    
    # Skip empty lines or comments
    if [[ -z "$detour" || "$detour" == \#* ]]; then
        continue
    fi
    
    echo -n "[$CURRENT/$TOTAL] Testing $detour..."
    echo "[$(date)] Testing $detour" >> "$RESULTS_DIR/test_log.txt"
    
    # Clean previous test
    rm -f "$SUCCESS_FILE"
    
    # Write detour to config
    echo "$detour" > "$CONFIG_FILE"
    
    # Launch Zoo Tycoon with timeout in background
    (
        export WINEPREFIX="$WINE_PREFIX"
        export WINEDEBUG=-all  # Reduce wine debug output
        timeout "$TIMEOUT" wine "$ZOO_TYCOON_PATH" &>/dev/null
    ) &
    PID=$!
    
    # Wait for success signal or timeout
    SUCCESS=false
    for i in $(seq 1 "$TIMEOUT"); do
        if [ -f "$SUCCESS_FILE" ]; then
            SUCCESS=true
            break
        fi
        
        # Check if process is still running
        if ! kill -0 $PID 2>/dev/null; then
            break
        fi
        
        sleep 1
    done
    
    # Kill process if still running
    if kill -0 $PID 2>/dev/null; then
        kill $PID 2>/dev/null
        wait $PID 2>/dev/null || true
    fi
    
    # Record result
    if [ "$SUCCESS" = true ]; then
        echo -e " ${GREEN}✓${NC}"
        echo "$detour" >> "$RESULTS_DIR/successful_detours.txt"
        echo "[$(date)] $detour - SUCCESS" >> "$RESULTS_DIR/test_log.txt"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    else
        echo -e " ${RED}✗${NC}"
        echo "$detour" >> "$RESULTS_DIR/failed_detours.txt"
        echo "[$(date)] $detour - FAILED" >> "$RESULTS_DIR/test_log.txt"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    
    # Small delay between tests to ensure clean state
    sleep 1
done

# Clean up test files
rm -f "$CONFIG_FILE" "$SUCCESS_FILE"

# Print summary
echo ""
print_info "Testing complete!"
echo ""
echo "================== SUMMARY =================="
echo -e "Total detours tested: ${YELLOW}$TOTAL${NC}"
echo -e "Successful: ${GREEN}$SUCCESS_COUNT${NC}"
echo -e "Failed: ${RED}$FAIL_COUNT${NC}"
if [ $SUCCESS_COUNT -gt 0 ]; then
    SUCCESS_RATE=$(echo "scale=1; $SUCCESS_COUNT * 100 / $TOTAL" | bc)
    echo -e "Success rate: ${GREEN}${SUCCESS_RATE}%${NC}"
fi
echo "============================================="
echo ""
print_info "Results saved to:"
echo "  - Successful: $RESULTS_DIR/successful_detours.txt"
echo "  - Failed: $RESULTS_DIR/failed_detours.txt"
echo "  - Full log: $RESULTS_DIR/test_log.txt"

# Exit with error if any tests failed
if [ $FAIL_COUNT -gt 0 ]; then
    exit 1
fi