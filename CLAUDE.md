# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OpenZT is a DLL injection framework for Zoo Tycoon (2001) written in Rust. It provides mod support, bug fixes, and feature enhancements through function detouring and memory manipulation.

**Target**: 32-bit Windows (`i686-pc-windows-msvc`) using nightly Rust
**Output**: `openzt.dll` (injected into the running game via the openzt-loader executable)

## Critical Rules

1. **NEVER commit Zoo Tycoon assets, code, configs, or decompiled content** - This is a clean-room reimplementation
2. **New features start behind `experimental` feature flag** in Cargo.toml
3. **All structs must use `#[repr(C)]`** for memory layout compatibility

## Development Commands

### Build Commands
```bash
# Debug build
cargo +nightly build --lib --target=i686-pc-windows-msvc

# Release build  
cargo +nightly build --lib --release --target=i686-pc-windows-msvc

# Documentation
cargo +nightly rustdoc --manifest-path openzt/Cargo.toml --lib --target i686-pc-windows-msvc --open -- --document-private-items
```

### Running/Testing
```bash
# Via loader (preferred)
run-via-loader.bat           # Debug
run-via-loader-release.bat   # Release
run-via-loader-pause.bat     # Suspended for debugger
```

### Lua Console (Runtime Scripting)

The console executes Lua code directly on the game thread. Connect after OpenZT is running:

```bash
cd openzt-console && cargo run
```

**Example Commands**:
```lua
-- List all available functions
help()

-- Search for specific functions
help("cash")

-- Game management
get_date()                           -- Get current in-game date
add_cash(10000)                      -- Add $10000 to budget
enable_dev_mode(true)                -- Enable developer mode
zoostats()                           -- Display zoo statistics

-- Settings
get_setting("AI", "cKeeperMaxTiredness")
set_setting("AI", "cKeeperMaxTiredness", "100")
list_settings()                      -- List all settings
list_settings("AI")                  -- List AI settings only

-- Entity management
get_selected_entity()                -- Get selected entity details
sel_type()                           -- Get selected entity type config
sel_type("-v")                       -- Verbose entity type info
make_sel(9500)                       -- Make entity type selectable

-- World/Habitat info
list_entities()                      -- List all entities in world
list_exhibits()                      -- List all exhibits/habitats
get_zt_world_mgr()                   -- World manager debug info

-- Expansions
list_expansion()                     -- List loaded expansions
get_current_expansion()              -- Get active expansion
get_members()                        -- List expansion member sets

-- Resources
list_resources()                     -- List BF resource directories
list_openzt_mods()                   -- List OpenZT mod IDs
get_string(9211)                     -- Get game string by ID

-- UI
ui("click_continue")                 -- Click continue button
continue()                           -- Shorthand for above
get_buy_tab()                        -- Get current buy tab
```

**Error Handling**:
```lua
-- Functions return (nil, error_string) on failure
result, err = get_string(999999)
if err then
    print("Error: " .. err)
else
    print("Result: " .. result)
end

-- Or check for nil
local date = get_date()
if date then
    print("Date: " .. date)
end
```

**Migration Note**: The old command-style syntax (e.g., `add_cash 1000`) is deprecated. Use Lua function calls (e.g., `add_cash(1000)`) instead. See `MIGRATION_TEMPLATE.md` for details on migrating remaining commands.

## Architecture Patterns

### Module Structure
- **Entry point**: `lib.rs` calls `init()` functions behind feature flags
- **Module pattern**: Each feature module has an `init()` function called from `lib.rs`
- **Feature flags**: Defined in `Cargo.toml` - new features use `experimental` flag

### Memory Management
```rust
// Global state pattern
use once_cell::sync::Lazy;
static GLOBAL_STATE: Lazy<Mutex<MyState>> = Lazy::new(|| Mutex::new(MyState::default()));

// Struct definitions
#[repr(C)]
#[derive(Debug)]
struct GameStruct {
    field: u32,
}
```

### Function Detouring
```rust
// Detour setup (subtract 0x400000 from Ghidra addresses)
static_detour! {
    static MY_DETOUR: unsafe extern "stdcall" fn(u32) -> u32;
}

// Calling game functions
let game_fn: unsafe extern "stdcall" fn(u32) -> u32 = 
    std::mem::transmute(0x12345678); // Full address
```

### Resource Handling
```rust
// Register resource handlers in init()
resource_manager::add_handler("bfb", Box::new(BfbHandler));
```

## Workspace Structure

- **`openzt/`**: Main DLL crate with game hooks and features
- **`openzt-loader/`**: DLL injection executable
- **`openzt-console/`**: Socket-based runtime console
- **`openzt-configparser/`**: Custom INI parser for Zoo Tycoon configs
- **`field_accessor_as_string*/`**: Derive macro crates

## Key Features

### Core Systems
- **Resource Management**: Custom file loading/modification via `resource_mgr/`
- **String Registry**: Game text injection via `string_registry.rs`
- **Lua Scripting**: Runtime Lua execution on game thread via TCP console (port 8080)
- **Settings**: Enhanced INI configuration loading
- **Expansion Packs**: Custom expansion support

### Development Features
- **Feature flags**: `default = ["experimental", "ini"]`, `release = []`
- **Conditional compilation**: Most features behind flags for testing
- **Hot-swappable**: DLL can be reloaded during development

## Testing

No automated game testing framework exists. Manual testing required:

1. Build the DLL
2. Test via loader OR manual installation
3. Verify features work in-game
4. Test console commands if applicable
5. Check for game crashes or memory issues

## Code Quality

- Avoid obvious comments that restate code
- Document complex game memory layouts and reverse engineering discoveries
- Use meaningful variable names for game offsets and structures
- Follow existing patterns for detour setup and global state management
