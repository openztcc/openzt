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

# Testing (use specific nightly version for consistency)
cargo +nightly-2024-05-02 test --manifest-path openzt/Cargo.toml --release --target=i686-pc-windows-gnu

# Documentation
cargo +nightly rustdoc --manifest-path openzt/Cargo.toml --lib --target i686-pc-windows-msvc --open -- --document-private-items
```

### Running/Testing
```bash
# Via loader (preferred)
run-via-loader.bat           # Debug
run-via-loader-release.bat   # Release
run-via-loader-pause.bat     # Suspended for debugger

# Console (after OpenZT is running)
cd openzt-console && cargo run
```

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
- **Console**: Runtime command execution via socket connection
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
