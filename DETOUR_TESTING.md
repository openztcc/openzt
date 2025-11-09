# Automated Detour Testing Strategy

This document outlines the automated testing strategy for validating generated detours that are NOT already tested through usage in the openzt crate.

## Overview

The testing focuses on validating detour signatures by:
1. Testing each detour individually during game startup
2. Recording success/failure for each detour
3. Prioritizing detours likely to be called during loading phase

## Excluded Detours (Already Tested)

The following detours are already used in the openzt crate and therefore validated:
- `BFAPP_LOADSTRING`
- `BFENTITY_GET_BLOCKING_RECT`
- `BFENTITY_GET_BLOCKING_RECT_ZTPATH`
- `BFENTITY_GET_FOOTPRINT`
- `BFENTITY_IS_ON_TILE`
- `BFMAP_GET_NEIGHBOUR`
- `BFMAP_TILE_TO_WORLD`
- `BFREGISTRY_ADD`
- `BFREGISTRY_ADDUI`
- `BFREGISTRY_PRTGET`
- `BFRESOURCE_ATTEMPT`
- `BFRESOURCE_PREPARE`
- `BFRESOURCEMGR_CONSTRUCTOR`
- `BFTILE_GET_LOCAL_ELEVATION`
- `BFUIMGR_DISPLAY_MESSAGE`
- `BFVERSIONINFO_GET_VERSION_STRING`
- `LOAD_DEBUG_SETTINGS_FROM_INI`
- `LOAD_LANG_DLLS`
- `ZOOLOGGING_LOG`
- `ZTAPP_UPDATEGAME`
- `ZTHABITAT_GET_GATE_TILE_IN`
- `ZTMAPVIEW_CHECK_TANK_PLACEMENT`
- `ZTUI_EXPANSIONSELECT_SETUP`
- `ZTUI_GENERAL_ENTITY_TYPE_IS_DISPLAYED`
- `ZTUI_GENERAL_GET_INFO_IMAGE_NAME`

## Testing Architecture

### Components

1. **Test Configuration File** (`detour_test_config.txt`)
   - Single line containing the name of the detour to test
   - Read by the DLL on startup

2. **Success Signal File** 
   - Created at path specified by compile-time environment variable
   - Presence indicates detour was successfully called

3. **Test Orchestration Script** (`test_detours.sh`)
   - Bash script that manages the testing loop
   - Updates configuration, launches game, checks results

4. **Result Files**
   - `successful_detours.txt` - List of working detours
   - `failed_detours.txt` - List of failed detours

### Test Flow

```
┌─────────────────────┐
│ Orchestration Script│
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Write detour name   │
│ to config file      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Launch Zoo Tycoon   │
│ in Wine             │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ DLL reads config    │
│ and activates       │
│ single detour       │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ If detour called,   │
│ create success file │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Script checks for   │
│ success file        │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Record result and   │
│ continue to next    │
└─────────────────────┘
```

## Detour Priority Ranking

### Tier 1: Load-Time Critical (Most likely during startup)
- `BFAPP_*` - Application initialization
- `BFGAMEAPP_*` - Game application setup
- `BFRESOURCEMGR_*` - Resource manager initialization
- `BFCONFIGFILE_*` - Configuration loading
- `BFINIFILE_*` - INI file parsing
- `BFFONTCACHE_*` - Font system initialization
- `BFLOG_*` - Logging system setup
- `BFDIAGNOSTIC_*` - Diagnostic initialization

### Tier 2: Early Game Systems
- `BFMGR_*` - Manager systems
- `BFUIMGR_*` - UI manager
- `BFREGISTRY_*` - Registry operations
- `BFVERSION_*` - Version checking
- `ZTAPP_*` - ZT application layer
- `ZTWORLDMGR_*` - World manager setup

### Tier 3: Resource Loading
- `BFRESOURCE_*` - Resource operations
- `BFCATEGORY_*` - Category loading
- `BFENTITYTYPE_*` - Entity type registration
- `BFANIMCACHE_*` - Animation cache

### Tier 4: UI Systems (May load on demand)
- `ZTUI_*` - UI components
- `ZTSCENARIOUI_*` - Scenario UI
- `ZTGENERALUI_*` - General UI

### Tier 5: Gameplay Systems (Unlikely during loading)
- `BFENTITY_*` - Entity operations
- `BFMAP_*` - Map operations  
- `ZTHABITAT_*` - Habitat systems
- `ZTANIMAL_*` - Animal behaviors
- `ZTGUEST_*` - Guest behaviors

## Implementation Details

### Detour Test Module (`openzt-detour/src/test.rs`)

```rust
use std::fs;
use std::path::Path;

// Compile-time configuration
const SUCCESS_FILE_PATH: &str = env!("DETOUR_TEST_SUCCESS_PATH");
const CONFIG_FILE_PATH: &str = env!("DETOUR_TEST_CONFIG_PATH");

pub fn initialize_test_mode() {
    if let Ok(config) = fs::read_to_string(CONFIG_FILE_PATH) {
        let detour_name = config.trim();
        // Enable only the specified detour
        activate_single_detour(detour_name);
    }
}

pub fn signal_detour_success(detour_name: &str) {
    // Create success file to signal to external script
    let _ = fs::write(SUCCESS_FILE_PATH, detour_name);
}

// Macro for test detours
#[macro_export]
macro_rules! test_detour {
    ($name:expr, $original_call:expr) => {{
        // Signal success when called
        $crate::test::signal_detour_success($name);
        // Call original function
        $original_call
    }}
}
```

### Test Orchestration Script (`test_detours.sh`)

```bash
#!/bin/bash

# Configuration
WINE_PREFIX="$HOME/.wine"
ZOO_TYCOON_PATH="/path/to/zoo.exe"
CONFIG_FILE="detour_test_config.txt"
SUCCESS_FILE="detour_success.txt"
RESULTS_DIR="test_results"
TIMEOUT=30

# Create results directory
mkdir -p "$RESULTS_DIR"

# Load detour list (excluding already tested ones)
mapfile -t DETOURS < detours_to_test.txt

# Testing loop
for detour in "${DETOURS[@]}"; do
    echo "Testing detour: $detour"
    
    # Clean previous test
    rm -f "$SUCCESS_FILE"
    
    # Write detour to config
    echo "$detour" > "$CONFIG_FILE"
    
    # Launch Zoo Tycoon with timeout
    timeout "$TIMEOUT" wine "$ZOO_TYCOON_PATH" &
    PID=$!
    
    # Wait for success signal or timeout
    for i in {1..30}; do
        if [ -f "$SUCCESS_FILE" ]; then
            echo "✓ $detour succeeded"
            echo "$detour" >> "$RESULTS_DIR/successful_detours.txt"
            kill $PID 2>/dev/null
            break
        fi
        sleep 1
    done
    
    # Check if timeout occurred
    if ! [ -f "$SUCCESS_FILE" ]; then
        echo "✗ $detour failed or not called"
        echo "$detour" >> "$RESULTS_DIR/failed_detours.txt"
    fi
    
    # Kill process if still running
    kill $PID 2>/dev/null
    wait $PID 2>/dev/null
    
    # Small delay between tests
    sleep 2
done

echo "Testing complete!"
echo "Successful: $(wc -l < "$RESULTS_DIR/successful_detours.txt")"
echo "Failed: $(wc -l < "$RESULTS_DIR/failed_detours.txt")"
```

## Usage

1. **Build with test mode enabled:**
```bash
DETOUR_TEST_CONFIG_PATH=/path/to/config.txt \
DETOUR_TEST_SUCCESS_PATH=/path/to/success.txt \
cargo build --features detour-testing --target=i686-pc-windows-msvc
```

2. **Generate detour list:**
```bash
# Extract all detour names from generated code
# Exclude already tested ones
./generate_detour_list.sh > detours_to_test.txt
```

3. **Run tests:**
```bash
./test_detours.sh
```

4. **Review results:**
```bash
cat test_results/successful_detours.txt
cat test_results/failed_detours.txt
```

## Next Steps

After initial testing:
1. Investigate failed detours for signature issues
2. Add successful detours to integration tests
3. Consider implementing more complex validation beyond simple calling