# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OpenZT is a DLL for injection into Zoo Tycoon (2001) that extends and modifies game functionality. It's written in Rust and uses function hooking/detouring to intercept game functions and direct memory manipulation.

**Key Architecture Points:**
- **Target**: Windows 32-bit (`i686-pc-windows-msvc`) using nightly Rust toolchain
- **Library Type**: `cdylib` (C-compatible dynamic library)
- **Injection Methods**: Either rename to `lang301-openzt.dll` in game directory or use OpenZT Loader
- **Module Pattern**: Features split into modules with init functions behind feature flags

## Commands

### Build Commands
```bash
# Debug build
cargo build

# Release build  
cargo build --release

# Build with specific features
cargo build --features "experimental,ini,capture_ztlog"
```

### Running OpenZT
```bash
# Via loader (debug)
./run-via-loader.bat

# Via loader (release)
./run-via-loader-release.bat

# Via loader (suspended for debugger)
./run-via-loader-pause.bat
```

### Development Commands
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Lint with clippy
cargo clippy --all-features --all-targets -- -D warnings

# Run tests
cargo test --all

# Check build
cargo check --all-targets

# Generate and open documentation
./open-docs.bat
```

## Architecture

### Core Modules
- **`lib.rs`**: Main entry point, initializes modules based on feature flags
- **`resource_manager/`**: Intercepts and modifies game resource loading
- **`console.rs`**: Socket communication with openzt-console
- **`string_registry.rs`**: Custom string management for game UI
- **`expansions.rs`**: Handles game expansion packs
- **`ztui.rs`**: UI modifications and hooks

### Key Patterns

**Struct Definition** - All structs must use `#[repr(C)]`:
```rust
#[derive(Debug)]
#[repr(C)]
pub struct UIElement {
    vftable: u32,
    // fields...
}
```

**Detours/Hooks** - Use retour crate with offsets:
```rust
#[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
    unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) }
}
```

**Global State** - Use Lazy<Mutex<T>> for thread safety:
```rust
static EXPANSION_ARRAY: Lazy<Mutex<Vec<Expansion>>> = Lazy::new(|| {
    Mutex::new(Vec::new())
});
```

**Calling ZT Functions** - Use full addresses (not offsets):
```rust
let get_element_fn: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(0x0040157d) };
```

## Critical Rules

1. **NO Zoo Tycoon Assets**: Never commit any Zoo Tycoon code, assets, or configs. This is a complete reimplementation.

2. **Feature Flags**: New features start behind `experimental` flag:
   - Default features: `["experimental", "ini"]`
   - Release features: `["ini"]`
   - Test stability before removing flag

3. **Module Init Pattern**: All modules must have init() function called from lib.rs behind feature flag

4. **Memory Safety**: 
   - Use `get_from_memory` and `save_to_memory` for struct access
   - Always use Mutex for global state
   - Careful with raw pointers and transmutes

5. **Resource Handlers**: Register handlers for file types via `resource_manager::add_handler()`

6. **Testing**: Manual testing required - no automated game testing framework exists

## Code Comments
Avoid adding comments that merely restate what the code is doing or that reference the development process (e.g., "BUG:", "TODO:" unless they're meant to stay). Comments should add value by explaining complex logic or design decisions, not narrate the obvious or temporary state of the code.
