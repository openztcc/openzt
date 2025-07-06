# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`retour-utils` is a Rust utility library that provides procedural macros to simplify creating function detours/hooks with the `retour` crate. It's designed for runtime function interception in dynamic libraries, commonly used for game modding, application instrumentation, and dynamic analysis.

## Build Commands

```bash
# Build the project
cargo build

# Build with all features
cargo build --all-features

# Build for release
cargo build --release

# Build for specific target (e.g., Windows)
cargo build --target x86_64-pc-windows-msvc
```

## Testing Commands

```bash
# Run all tests
cargo test

# Run tests with all features
cargo test --all-features

# Run a specific test
cargo test test_name

# Run tests in a specific crate
cargo test -p retour-utils-impl
```

## Development Commands

```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check

# Run clippy linting
cargo clippy --all-features

# Check code without building
cargo check --all-features

# Generate and open documentation
cargo doc --open
```

## Architecture

The project is a Rust workspace with two crates:

1. **Main crate** (`/`): Exports the `hook_module` macro and provides the runtime support via `init_detour` function
   - `/src/lib.rs`: Core API with `LookupData` enum for symbol/offset-based hooking
   - `/src/error.rs`: Error types

2. **Proc macro crate** (`/impl/`): Implements the `#[hook_module]` procedural macro
   - Processes module definitions with `#[hook]` attributes
   - Generates static detours and initialization code
   - Published separately as `retour-utils-impl`

### Key Design Patterns

- **Symbol-based hooking**: Functions are hooked by their exported symbol names
- **Offset-based hooking**: Functions can be hooked by memory offset from module base
- **Static detours**: Uses `retour::StaticDetour` for type-safe function replacement (default)
- **Generic detours**: Uses `retour::GenericDetour` for runtime-controllable hooks (with `generic` flag)
- **Cross-platform**: Uses `minidl` for platform-agnostic dynamic library loading

### Detour Types

**StaticDetour** (default behavior):
- Syntax: `#[hook(DetourName, symbol = "func")]`
- Initialized once during `init_detours()`
- Lives for entire program lifetime
- Access original via `DetourName.call()`

**GenericDetour** (with `generic` flag):
- Syntax: `#[hook(DetourName, symbol = "func", generic)]`
- Manual control via `enable_DetourName()` and `disable_DetourName()`
- Can be enabled/disabled at runtime
- Access original via `call_original_DetourName()`
- NOT initialized by `init_detours()`

### Macro Expansion

The `#[hook_module("library.dll")]` macro:
1. Generates a `MODULE_NAME` constant
2. Creates `retour::StaticDetour` or `retour::GenericDetour` instances based on flags
3. Generates an `init_detours()` function that initializes static detours only
4. For generic detours, generates enable/disable/call_original functions
5. Preserves the original function signatures for type safety

## Testing Strategy

- **Procedural macro tests**: Uses `trybuild` framework in `/tests/build-tests/`
- Tests verify both successful compilation and expected error cases
- Examples in `/examples/` demonstrate real-world usage patterns

## Platform Support

- Primary development on Windows (CI tests x86_64 and i686 MSVC targets)
- Unix/Linux support available but not actively tested in CI
- Requires Rust nightly toolchain (see `rust-toolchain.toml`)