#!/bin/bash

# Script to generate list of detours to test
# Excludes detours already tested in the openzt crate

set -e

# Detours that are already tested (used in openzt crate)
TESTED_DETOURS=(
    "BFAPP_LOADSTRING"
    "BFENTITY_GET_BLOCKING_RECT"
    "BFENTITY_GET_BLOCKING_RECT_ZTPATH"
    "BFENTITY_GET_FOOTPRINT"
    "BFENTITY_IS_ON_TILE"
    "BFMAP_GET_NEIGHBOUR"
    "BFMAP_TILE_TO_WORLD"
    "BFREGISTRY_ADD"
    "BFREGISTRY_ADDUI"
    "BFREGISTRY_PRTGET"
    "BFRESOURCE_ATTEMPT"
    "BFRESOURCE_PREPARE"
    "BFRESOURCEMGR_CONSTRUCTOR"
    "BFTILE_GET_LOCAL_ELEVATION"
    "BFUIMGR_DISPLAY_MESSAGE"
    "BFVERSIONINFO_GET_VERSION_STRING"
    "LOAD_DEBUG_SETTINGS_FROM_INI"
    "LOAD_LANG_DLLS"
    "ZOOLOGGING_LOG"
    "ZTAPP_UPDATEGAME"
    "ZTHABITAT_GET_GATE_TILE_IN"
    "ZTMAPVIEW_CHECK_TANK_PLACEMENT"
    "ZTUI_EXPANSIONSELECT_SETUP"
    "ZTUI_GENERAL_ENTITY_TYPE_IS_DISPLAYED"
    "ZTUI_GENERAL_GET_INFO_IMAGE_NAME"
)

# Function to check if a detour is already tested
is_tested() {
    local detour="$1"
    for tested in "${TESTED_DETOURS[@]}"; do
        if [ "$detour" == "$tested" ]; then
            return 0
        fi
    done
    return 1
}

# Function to get priority tier for a detour
get_priority() {
    local detour="$1"
    
    # Tier 1: Load-time critical
    if [[ "$detour" =~ ^BFAPP_ ]] || \
       [[ "$detour" =~ ^BFGAMEAPP_ ]] || \
       [[ "$detour" =~ ^BFRESOURCEMGR_ ]] || \
       [[ "$detour" =~ ^BFCONFIGFILE_ ]] || \
       [[ "$detour" =~ ^BFINIFILE_ ]] || \
       [[ "$detour" =~ ^BFFONTCACHE_ ]] || \
       [[ "$detour" =~ ^BFLOG_ ]] || \
       [[ "$detour" =~ ^BFDIAGNOSTIC_ ]]; then
        echo "1"
    # Tier 2: Early game systems
    elif [[ "$detour" =~ ^BFMGR_ ]] || \
         [[ "$detour" =~ ^BFUIMGR_ ]] || \
         [[ "$detour" =~ ^BFREGISTRY_ ]] || \
         [[ "$detour" =~ ^BFVERSION_ ]] || \
         [[ "$detour" =~ ^ZTAPP_ ]] || \
         [[ "$detour" =~ ^ZTWORLDMGR_ ]]; then
        echo "2"
    # Tier 3: Resource loading
    elif [[ "$detour" =~ ^BFRESOURCE_ ]] || \
         [[ "$detour" =~ ^BFCATEGORY_ ]] || \
         [[ "$detour" =~ ^BFENTITYTYPE_ ]] || \
         [[ "$detour" =~ ^BFANIMCACHE_ ]]; then
        echo "3"
    # Tier 4: UI Systems
    elif [[ "$detour" =~ ^ZTUI_ ]] || \
         [[ "$detour" =~ ^ZTSCENARIOUI_ ]] || \
         [[ "$detour" =~ ^ZTGENERALUI_ ]]; then
        echo "4"
    # Tier 5: Gameplay systems
    else
        echo "5"
    fi
}

# Extract all detour constants from gen.rs
echo "Extracting detours from openzt-detour/src/gen.rs..." >&2

# Parse the gen.rs file to extract detour names
# Looking for patterns like: pub const FUNCTION_NAME: FunctionDef<...>
DETOURS=()

while IFS= read -r line; do
    if [[ "$line" =~ pub[[:space:]]+const[[:space:]]+([A-Z_]+):[[:space:]]*FunctionDef ]]; then
        detour_name="${BASH_REMATCH[1]}"
        
        # Skip if already tested
        if ! is_tested "$detour_name"; then
            priority=$(get_priority "$detour_name")
            DETOURS+=("$priority:$detour_name")
        fi
    fi
done < openzt-detour/src/gen.rs

# Sort by priority (tier) then alphabetically
printf '%s\n' "${DETOURS[@]}" | sort -t: -k1n,1 -k2 | cut -d: -f2

echo "Generated list of $(printf '%s\n' "${DETOURS[@]}" | wc -l) detours to test" >&2