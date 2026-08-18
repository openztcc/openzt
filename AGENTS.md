# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ⚠️ CRITICAL: ALWAYS USE openzt.bat (Windows) or specific cargo commands (Linux)

**On Windows**: NEVER use `cargo` directly - ALWAYS use `./openzt.bat` for ANY cargo command (build, check, clippy, test, run, etc.).

The project uses a specific toolchain and target configuration that is managed by `openzt.bat`. Running `cargo` directly will use the wrong toolchain/target.

**On Linux**: Batch files do not work. Use specific cargo commands directly:
- Build: `cargo build --manifest-path openzt/Cargo.toml --lib --target=i686-pc-windows-gnu`
- Check: `cargo check --manifest-path openzt/Cargo.toml --lib --target=i686-pc-windows-gnu`
- Clippy: `cargo clippy --manifest-path openzt/Cargo.toml --lib --target=i686-pc-windows-gnu`
- Test: `cargo test --manifest-path openzt/Cargo.toml --lib --target=i686-pc-windows-gnu`

**⚠️ CRITICAL: ALWAYS specify `--target=i686-pc-windows-gnu` on Linux** - The `thiscall` and `stdcall` ABIs are Windows-specific. Without the target, cargo will use the Linux host target and fail with ABI errors. These errors are expected when cross-compiling and the code will compile correctly on Windows with the proper target specified.

**Windows Examples:**
- ❌ `cargo check` → ✅ `./openzt.bat check`
- ❌ `cargo build` → ✅ `./openzt.bat build`
- ❌ `cargo clippy` → ✅ `./openzt.bat clippy`
- ❌ `cargo test` → ✅ `./openzt.bat test`

If `openzt.bat` is missing a command you need, ADD IT to `openzt.bat` rather than running cargo directly.

## Project Overview

OpenZT is a DLL injection framework for Zoo Tycoon (2001) written in Rust. It provides mod support, bug fixes, and feature enhancements through function detouring and memory manipulation.

**Target**: 32-bit Windows (`i686-pc-windows-msvc`)
**Output**: `openzt.dll` (copied to Zoo Tycoon directory and loaded automatically)

## Critical Rules

1. **NEVER commit Zoo Tycoon assets, code, configs, or decompiled content** - This is a clean-room reimplementation
2. **ALWAYS USE openzt.bat for building, testing, running or just anything that would usually require use of `cargo`** - If openzt.bat is missing functionality add it rather than running cargo directly
3. **New features start behind `experimental` feature flag** in Cargo.toml
4. **All structs must use `#[repr(C)]`** for memory layout compatibility

## Development Commands

**IMPORTANT**: Always use `./openzt.bat` for cargo actions on the openzt crate (build, check, clippy, docs). This ensures correct toolchain selection and target configuration.

### Build Commands
```bash
# Build only (no game launch)
./openzt.bat build                           # Debug with command-console
./openzt.bat build --release                 # Release with command-console
./openzt.bat build --test                    # Debug test build
./openzt.bat build --test --release          # Release test build

# Build and run
./openzt.bat run                             # Debug with command-console
./openzt.bat run --release                   # Release with command-console

# Extra Cargo args/features go after `--`
./openzt.bat build -- --features egui-overlay
./openzt.bat check -- --features egui-overlay
./openzt.bat run -- --features egui-overlay
./openzt.bat build -- --features egui-overlay --features debug-blit

# Build and run with --wait flag (waits for game to exit before returning)
./openzt.bat run --wait                      # Debug, wait for exit
./openzt.bat run --release --wait            # Release, wait for exit

# Integration tests
./openzt.bat integration-tests               # Run all integration tests (builds release, displays results)

# Code quality checks
./openzt.bat check                           # Run cargo check on openzt
./openzt.bat check -- --features egui-overlay # Required when touching egui overlay code
./openzt.bat clippy                          # Run cargo clippy on openzt
./openzt.bat test                            # Run cargo test on openzt

# Documentation
./openzt.bat docs
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

### Creating Console Commands

Adding new console commands to OpenZT is done using the `lua_fn!` macro in `openzt/src/scripting.rs`. This macro handles function registration, metadata for `help()`, and Lua integration automatically.

#### Registration Location

All commands are registered in the `init()` function in `openzt/src/scripting.rs` (around line 136). Add your command at the end of this function, before the closing brace.

#### Command Patterns

**No Arguments:**
```rust
lua_fn!("ping", "Test console connectivity", "ping()", || {
    Ok("pong".to_string())
});
```

**Single Argument:**
```rust
lua_fn!("get_string", "Get game string by ID", "get_string(id)", |id: u32| {
    Ok(format!("String: {}", id))
});
```

**Multiple Arguments:**
```rust
lua_fn!("set_setting", "Set a configuration value", "set_setting(section, key, value)",
    |section: String, key: String, value: String| {
        Ok(format!("Set {}.{} = {}", section, key, value))
    }
);
```

**Optional Arguments:**
```rust
lua_fn!("help", "List functions or search by keyword", "help([search_term])",
    |search: Option<String>| {
        match search {
            Some(term) => Ok(format!("Searching for: {}", term)),
            None => Ok("All functions:".to_string()),
        }
    }
);
```

#### Return Value Patterns

**Simple string:**
```rust
Ok("result".to_string())
```

**Tuple (value, error):**
```rust
Ok(("Success message".to_string(), None::<String>))
// On error: Ok((String::new(), "Error message".to_string()))
```

**Unit/void:**
```rust
Ok(())
```

#### Error Handling

Functions should return errors as a tuple with the error string in the second position:

```rust
lua_fn!("get_entity", "Get entity by ID", "get_entity(id)", |id: u32| {
    match find_entity(id) {
        Some(entity) => Ok((entity.name, None::<String>)),
        None => Ok((String::new(), format!("Entity {} not found", id))),
    }
});
```

In Lua, callers check for errors like:
```lua
result, err = get_entity(123)
if err then
    print("Error: " .. err)
else
    print("Result: " .. result)
end
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

### egui Overlay

- Enabled with `egui-overlay` and gated by `experimental`; run with `./openzt.bat run -- --features egui-overlay`
- When changing `openzt/src/ui/**` or egui overlay behavior, ALWAYS verify with `./openzt.bat check -- --features egui-overlay` or `./openzt.bat build -- --features egui-overlay`; a plain `./openzt.bat check` does not compile this code.
- UI code lives in `openzt/src/ui/mod.rs`; add/update egui widgets inside the `backend.run_frame(...)` closure.
- Rendering uses `egui-tiny-skia` into a `tiny_skia::Pixmap`, then `openzt/src/ui/blit.rs` uploads it to a click-through layered overlay window with per-pixel alpha.
- Frame timing hooks are in `openzt/src/ui/render_hook.rs`; mouse events come from the subclassed game WndProc in `openzt/src/ui/wndproc.rs`; keyboard events are forwarded from `shortcuts.rs`.
- Use `--features egui-overlay --features debug-blit` only to test the overlay/window upload path with a solid red image.

### Vanilla UI Layout Notes

- Vanilla `.lyt` files are INI-like and may repeat keys (`state=`, `button=`, `image=`, `backcolor=`); use `openzt-configparser` vector access (`get_vec`) when order or all values matter.
- `.lyt` `animation=ui/main/pause/pause` points to descriptor `ui/main/pause/pause.ani`, not directly to the binary animation frame.
- In `.ani`, join all `dir*` entries with each `animation` value to get binary animation resources: `dir0=ui`, `dir1=main`, `dir2=pause`, `animation=N` => `ui/main/pause/N`. The matching palette is usually beside the descriptor, e.g. `ui/main/pause/pause.pal`.
- Common button animation states are `N` normal, `H` hover, `S` selected/pressed, and `G` disabled/greyed. Static visual passes usually use `N`; toggle buttons (`state=2048`) may need `S` once real UI state is implemented.
- Respect `layer` ordering and anchors. `x/y` are relative to `anchor` when present; `x=left|center|right` and `y=top|bottom` anchor to the screen/layout. `dynamicheight=1` fillers repeat/extend vertically only; `dynamicwidth=1` fillers repeat/extend horizontally only.

### Inspecting Vanilla Resource Files

Vanilla resources ship inside `.ztd` archives (base game: `*.ztd` in the Zoo Tycoon install dir; expansions: `XPACK1\*.ztd`, `XPACK2\*.ztd`; patches: `Updates\*.ztd`, `dupdate\*.ztd`, `dlupdate\*.ztd`). A `.ztd` is just a renamed `.zip` - when a decompile is ambiguous or a struct-field assumption needs ground-truthing, read the real file content directly rather than guessing:

```python
import zipfile
z = zipfile.ZipFile(r"C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\research.ztd")
print(z.read("research/branres.cfg").decode("latin1"))
```

To find which archive holds a given resource path, or to enumerate everything matching a pattern:

```python
import zipfile, glob
archives = glob.glob(r"C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\**\*.ztd", recursive=True)
for archive in archives:
    for name in zipfile.ZipFile(archive).namelist():
        if name.lower().startswith("research/") and name.lower().endswith(".cfg"):
            print(archive, name)
```

As with `private/resources/decompiles/`, this is for local verification only - never commit extracted vanilla assets/content (see "Critical Rules" above).

### Inspecting the Vanilla DLL's PE Header (vtables, section layout)

Confirmed vtable findings (addresses, confirmed slots, and the evidence for each) are written up per-class
in `private/docs/vtables/` - check there before re-deriving a class's vtable from scratch, and add a new file there
once you've confirmed one.

`private/resources/dll/zoo.dll` is a real 32-bit PE file. When a decompile references a vtable or a raw address and
you need to independently verify it (rather than trust the decompile's naming), read the actual bytes:

```bash
python3 private/scripts/parse_pe_header.py private/resources/dll/zoo.dll
```

prints the image base and section table (name, virtual address, file offset, size) with no dependencies
beyond the Python standard library (`pefile` is not installed in this environment and isn't required).

To resolve a specific virtual address - e.g. a vtable found via Ghidra's symbol naming convention, which
embeds the address directly (`cls_0x635100__vftable_635100_00635100` -> VA `0x00635100`; usually found by
grepping a class's constructor/destructor decompile for `__vftable_`) - and dump its first N entries as
pointers (i.e. what a vtable's first N slots would look like):

```bash
python3 private/scripts/parse_pe_header.py private/resources/dll/zoo.dll --va 0x00635100 --dump 20
```

Each resolved pointer can be cross-checked against `openzt-detour/src/generated.rs`'s registered
`FunctionDef` addresses - a real match (e.g. a vtable slot landing exactly on an already-registered
address) is strong independent confirmation both that the vtable address is right and that the slot's
purpose is understood.

To get a rough sense of how much vtable-shaped data exists in a section overall (useful for scoping how
much of the DLL is still unaccounted for - see `private/docs/vtables/PROGRESS.md`), scan for runs of consecutive
words that each resolve into `.text`:

```bash
python3 private/scripts/parse_pe_header.py private/resources/dll/zoo.dll --scan-runs .rdata
```

This is a mechanical, symbol-free heuristic - it cannot tell where one class's vtable ends and an
adjacently-packed next class's begins, so treat its run lengths as an upper bound on entry count, not a
class count.

**Do not determine a vtable's length by dumping N entries and eyeballing where the pointers "stop looking
valid."** Multiple classes' vtables are frequently packed back-to-back in `.rdata` with no gap or marker
between them, and every slot on both sides is an equally legitimate `.text` function pointer - a scan like
this has no way to detect that boundary and will silently attribute one class's vtable slots to another
(confirmed the hard way: an initial 10-entry read of `ZTResearchMgr`'s vtable at `0x00635100` turned out to
actually be `ZTResearchMgr`'s real 4-entry vtable immediately followed by all of `ZTMarketingMgr`'s vtable,
which starts, with no gap, at `0x00635110`). The pointer values only reveal something's *wrong* when the
following data happens to not look like a code pointer at all (e.g. a stray float constant, or an address
outside `.text`) - which is not guaranteed, and gave a false sense of confidence in this case since the
first several slots past the true boundary still resolved cleanly into `.text`.

One candidate approach: grep every decompile for `__vftable_` symbols with a higher address than your target
(`grep -r "__vftable_" private/resources/decompiles/`), then confirm the nearest candidate by finding *that* class's
own constructor directly assigning it (`*this = &cls_0xNNNNNN__vftable_...`), and take the gap between the two
confirmed constructor-assignment addresses as the vtable's size. **This is not reliable either** - it only
proves where the *next known* vtable starts, not that nothing else sits between them. Confirmed the hard way on
`ZTMarketingMgr` (`0x00635110`): the nearest next constructor-confirmed vtable in the available decompiles was
`0x006351a0`, suggesting 36 slots, but several of those "slots" resolved to addresses already registered under
unrelated names in `openzt-detour/src/generated.rs` (mouse/keyboard handling, terraforming) - the gap actually
contains other undecompiled classes' vtables packed in between, not one 36-entry vtable.

The approach that actually holds up: grep every decompile for **offset-based virtual calls on the global
instance** - patterns like `(**(code**)(*(int*)GLOBAL_X + N))(...)` or `(*(code*)**(undefined4**)GLOBAL_X)(...)`
(`grep -rn "GLOBAL_<ClassName>" private/resources/decompiles/`, then look for calls dereferencing through it). Each hit
is a *positive, load-bearing proof* that offset `N` is a real vtable slot - some other piece of the actual game
code dereferences and calls it - and the call site's arguments/context (e.g. called from `ZTApp_updateSim.c`,
`ZTWorldMgr_save.c`, `ZTWorldMgr_load.c`) often reveal the slot's purpose for free. This is how `ZTMarketingMgr`
was confirmed to have (at least) 4 slots - dtor/update/save/load at offsets `0x0/0x4/0x8/0xc` - matching the
exact same offsets independently confirmed for `ZTResearchMgr`'s vtable via the same technique, strongly
suggesting both implement a shared small "Mgr" interface.

The limitation: this only proves a **lower bound**. A slot that exists but is never called anywhere in the
available decompiles stays invisible to this method, so "N confirmed slots" means "at least N," not "exactly
N." Treat a raw pointer-dump (`--dump`) past the last confirmed offset only as a sanity check for obviously
wrong data (a stray float constant, an address outside `.text`) or for cross-referencing against
`generated.rs`/named functions to catch "you've walked into another class's vtable" - never as the sole basis
for a boundary claim.

Once a class's address is confirmed, `parse_pe_header.py` also automates the two mechanical steps that
`private/docs/vtables/*.md` files repeat for every class - use these instead of re-deriving them by hand each session:

```bash
# Auto-detect a candidate length via the repeating-pointer boundary heuristic (private/docs/vtables/README.md),
# instead of guessing a --dump size and grepping the output for a repeat by hand:
python3 private/scripts/parse_pe_header.py private/resources/dll/zoo.dll --find-length --va 0x0062ea54

# Diff a class against its confirmed base and print the three ready-to-paste markdown tables
# (confirmed overrides / new slots / full raw dump) every class doc needs:
python3 private/scripts/parse_pe_header.py private/resources/dll/zoo.dll --diff \
    --va 0x0062ea54 --length 204 --base-va 0x0062ed84 --base-length 204 --base-name ZTStaff
```

`--find-length`'s output is still only a *candidate* boundary, not a confirmed one - per the methodology
above, cross-check it against an independently-known class address before treating it as final. `--diff`'s
markdown records every non-matching slot as `*unknown*`; only replace that with a real name/role if you have
independent call-site evidence (a `virt_meth_0xADDR` reference, a cast at a call site) - never guess.

As with the resource-file inspection above, this is for local verification only - never commit extracted
vanilla binary content.

## Testing

### Integration Tests

OpenZT includes an integration testing framework that runs tests in a live game environment. These tests verify mod loading, patch application, and resource management using the actual game engine.

**Running Integration Tests**:
```bash
# Run all integration tests (builds release, launches game, displays results automatically)
./openzt.bat integration-tests
```

The `integration-tests` command:
- Builds the DLL in release mode with the `integration-tests` feature flag
- Launches Zoo Tycoon and waits for tests to complete
- Displays test results automatically after the game exits
- Shows paths to log files for detailed debugging

**Checking Test Results**:
```bash
# View the integration test log
cat "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\openzt_integration_tests.log"

# View detailed OpenZT logs (patch application, errors, etc.)
cat "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\openzt.log"
```

**Test Output**:
```
=== OpenZT Integration Tests ===

Running dependency resolution tests...
  ✓ test_simple_dependency_chain
  ✓ test_circular_dependency_handling
  ✓ test_optional_dependency_warning
  ... (11 tests)

Running patch rollback tests...
  ✓ test_continue_mode_applies_directly
  ✓ test_abort_mode_rolls_back_on_failure
  ... (9 tests)

Running loading order tests...
  ✓ test_category_ordering
  ✓ test_cross_file_habitat_reference
  ... (8 tests)

Running legacy attributes tests...
  ✓ test_legacy_animal_attributes_loaded
  ✓ test_legacy_fence_attributes_loaded
  ... (24 tests)

Results: 52 passed, 0 failed
ALL TESTS PASSED
```

**Test Categories**:

1. **Dependency Resolution Tests** (`openzt/src/integration_tests/dependency_resolution.rs`)
   - Test simple dependency chains
   - Test circular dependency detection and handling
   - Test optional dependencies and warnings
   - Test `before` dependencies
   - Test disabled mods exclusion
   - Test validation of dependency violations

2. **Patch Rollback Tests** (`openzt/src/integration_tests/patch_rollback.rs`)
   - Test patch error handling modes (continue, abort, abort_mod)
   - Verify shadow resource system for transactional patch application
   - Test patch operations (set_key, merge, delete, etc.)

3. **Loading Order Tests** (`openzt/src/integration_tests/loading_order.rs`)
   - Verify deterministic mod definition file loading order
   - Test category ordering (NoPatch → Mixed → PatchOnly)
   - Verify alphabetical sorting within categories
   - Test cross-file habitat/location references in patches

4. **Legacy Attributes Tests** (`openzt/src/integration_tests/legacy_attributes.rs`)
   - Test loading of legacy entity attributes from .cfg files
   - Test default subtype assignment (animal, staff, fence, wall)
   - Test explicit subtype specification
   - Test patch-based legacy attribute substitution
   - Test fallback behavior for invalid subtypes
   - Test cNameID string ID resolution

**Creating New Tests**:

1. Add test functions to appropriate test module:
```rust
pub fn run_all_tests() -> Vec<TestResult> {
    vec![
        test_existing_feature(),
        test_your_new_feature(),  // Add here
    ]
}

fn test_your_new_feature() -> TestResult {
    let test_name = "test_your_new_feature";

    // Setup test data
    // ...

    // Perform test operations
    // ...

    // Verify results
    if expected == actual {
        TestResult::pass(test_name)
    } else {
        TestResult::fail(test_name, format!("Expected {}, got {}", expected, actual))
    }
}
```

2. For tests requiring mod resources, use the embedded test mod pattern:
```rust
// In loading_order.rs - embed test TOML files
const DEF_FILE: &str = include_str!("../../resources/test/your-test/defs/test.toml");

// Add to create_test_mod_file_map()
file_map.insert(
    "defs/test.toml".to_string(),
    DEF_FILE.as_bytes().to_vec().into_boxed_slice(),
);
```

3. Create test resource files in `openzt/resources/test/your-test/`:
```
your-test/
├── meta.toml
└── defs/
    └── test.toml
```

**Embedded Test Mod Pattern**:

Integration tests use an embedded mod approach where test resources are compiled directly into the binary:

- Test files are embedded using `include_str!()` and `include_bytes!()`
- No ZIP file creation or installation required
- Changes to test resources take effect on next build
- Zero runtime overhead - resources are in memory at compile time

**Important Notes**:

- Tests run in a live game environment with initialized memory structures
- The game launches and exits automatically when tests complete
- Use the `--wait` flag to wait for the game to exit before returning control (recommended for automated workflows and CI)
- Test log is always written to `C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\openzt_integration_tests.log`
- Load order tracking is only enabled with `integration-tests` feature flag
- Tests create temporary files (e.g., `animals/test.ai`) for verification
- **Habitat/Location Registration**: Always use the TOML key identifier (e.g., "test_habitat_a"), NOT the display name (e.g., "Test Habitat A") when looking up habitats/locations in tests

### Game Launch Checks

The build script automatically checks if Zoo Tycoon is already running before attempting to launch:

```bash
./openzt.bat run --release

# If Zoo Tycoon is already running, you'll see:
# ERROR: Zoo Tycoon is already running.
# Please close the existing instance before launching a new one.
```

This prevents DLL copy failures due to file locks and ensures clean testing environments.

### Manual Testing

For features not covered by integration tests:

1. Build and run with `./openzt.bat run --release`
2. Verify features work in-game
3. Test console commands if applicable
4. Check for game crashes or memory issues

## Code Quality

- Avoid obvious comments that restate code
- Document complex game memory layouts and reverse engineering discoveries
- Use meaningful variable names for game offsets and structures
- Follow existing patterns for detour setup and global state management
