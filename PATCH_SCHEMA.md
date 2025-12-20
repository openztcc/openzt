# OpenZT Patch System - Schema Documentation

## Overview

This document describes the patch system TOML schema for OpenZT, which allows modders to define operations on Zoo Tycoon mod files. The schema enables mods that alter the same files to coexist through file-level and element-level operations.

## Current Implementation Status

### ✅ Completed
1. **Phase 1: Data Structures** - All patch types defined in `openzt/src/mods.rs`
   - `Patch` enum with 13 operation variants (including SetPalette)
   - `MergeMode` enum for merge priority control
   - `ErrorHandling` and `OnExists` enums for error handling and collision resolution
   - `PatchCondition` support for conditional patching
   - TOML deserialization working and tested
   - **Refactored Structure**: Split into `PatchMeta` (metadata) and separate `patches` field in `ModDefinition`
   - TOML uses `[patch_meta]` section for on_error and conditions, `[patches.*]` for individual patches

2. **Phase 2: INI Parser Enhancements** - New methods in `openzt-configparser/src/ini.rs`
   - `has_section()` - check if section exists
   - `clear_section()` - remove all keys from section
   - `merge_with_priority()` - merge with configurable priority (patch_priority vs base_priority)
   - `MergeMode` enum exported from configparser

3. **Phase 3: File-Level Patch Operations** - Implemented in `openzt/src/resource_manager/legacy_loading.rs`
   - `apply_replace_patch()` - replace entire files in resource system (lines 157-195)
   - `apply_merge_patch()` - merge INI files with configurable priority (lines 210-271)
   - `apply_delete_patch()` - remove files from resource system (lines 281-299)
   - `apply_set_palette_patch()` - modify animation palette references (lines 313-343)
   - `remove_resource()` - public function added to lazyresourcemap.rs (line 251-253)
   - Full logging and error handling for all operations
   - Proper validation (file types, extensions, existence checks)

4. **Phase 4: Element-Level Operations** - Implemented in `openzt/src/resource_manager/legacy_loading.rs`
   - `apply_set_key_patch()` - set single key-value pairs (lines 435-450)
   - `apply_set_keys_patch()` - set multiple key-value pairs (lines 462-479)
   - `apply_append_value_patch()` - append single value to arrays (lines 491-506)
   - `apply_append_values_patch()` - append multiple values to arrays (lines 518-536)
   - `apply_remove_key_patch()` - remove single keys (lines 548-569)
   - `apply_remove_keys_patch()` - remove multiple keys (lines 581-611)
   - `apply_add_section_patch()` - add sections with on_exists collision handling (lines 623-671)
   - `apply_clear_section_patch()` - clear all keys from sections (lines 683-705)
   - `apply_remove_section_patch()` - remove entire sections (lines 717-738)
   - Helper functions: `validate_ini_file()`, `load_ini_from_resources()`, `save_ini_to_resources()` (lines 369-423)
   - Full logging, error handling, and validation for all operations
   - Conditional patching support (patch-level conditions will be evaluated in Phase 6)

5. **Phase 5: Resource Map Updates** - Implemented in `openzt/src/resource_manager/lazyresourcemap.rs`
   - `update_resource()` public API added (lines 255-266)
   - Refactored patch operations to eliminate redundant remove_resource() calls
   - `add_ztfile()` already handles replacement via `insert_custom()`
   - Case-insensitive lookups already working correctly

6. **Phase 6: Patch Orchestration** ✅ (COMPLETED)
   - Apply patches with error handling and conditional evaluation (lines 784-930)
   - Top-level condition evaluation (skip file if conditions fail)
   - Patch-level condition evaluation (skip individual patches)
   - Comprehensive logging with patch names
   - Error handling modes (currently only 'continue' supported)

6.7. **Phase 6.7: ModDefinition Refactoring** ✅ (COMPLETED)
   - Refactored `PatchFile` into `PatchMeta` and separate patches field
   - Updated `ModDefinition` struct with `patch_meta` and `patches` fields
   - Updated test code to load patches as part of ModDefinition
   - Updated TOML structure to use `[patch_meta]` section
   - All tests passing, builds successfully

### 🚧 Remaining Work

6.6. **Phase 6.6: Snapshot/Rollback & Dry-Run** (not yet implemented)
   - Implement selective snapshot/rollback for abort_mod
   - Add dry_run flag support
   - Enable abort and abort_mod error handling modes

7. **Phase 7: Comprehensive Testing** (not yet implemented)
   - Integration tests for all patch types
   - Edge case testing (missing files, sections, keys)
   - Cross-mod patching tests
   - Conditional patching tests
   - Error handling mode tests

## TOML Schema Reference

### Patch File Structure

Each patch file has a patch metadata section followed by named patch operations:

```toml
# Patch metadata section (optional)
[patch_meta]
on_error = "continue"  # Options: "continue" (default), "abort", "abort_mod"

# Top-level conditions (optional) - if these fail, entire file is skipped
[patch_meta.condition]
mod_loaded = "SomeRequiredMod"
# ... other conditions

# Individual patches
[patches.first_patch]
operation = "set_key"
# ... patch details

[patches.second_patch]
operation = "merge"
# ... patch details
```

**Error Handling** (`on_error` field at top level):
- **`continue`** (default): Log error/warning and continue to next patch in this file
- **`abort`**: Stop processing remaining patches in this file, continue loading mod (other patch files still process)
- **`abort_mod`**: Stop loading this mod entirely (use for critical patches)

**Current Limitation**: Only `on_error = "continue"` is currently supported. The `abort` and `abort_mod` modes require snapshot/rollback functionality which will be added in a future update.

**Top-Level Conditions** (`[patch_meta.condition]` table):
- Optional conditions that apply to the entire file
- If top-level conditions fail, the entire patch file is skipped (logged as warning)
- Uses same condition types as individual patches: `mod_loaded`, `key_exists`, `value_equals`
- All conditions must pass (AND logic)

**Target Requirement for key_exists/value_equals**:
When using `key_exists` or `value_equals` in top-level conditions, you MUST specify a `target` field:

```toml
[patch_meta.condition]
target = "config/settings.ini"
key_exists = { section = "Graphics", key = "HDTextures" }
```

**Note**: The `mod_loaded` condition does not require a target.

### Named Patches

Patches use named subtables with the syntax `[patches.patch_name]`. Each patch must have a unique name that will be used in logs and error messages. Patches are executed in the order they appear in the file (order is preserved via `IndexMap`).

### File-Level Operations

#### Replace Entire File
```toml
[patches.replace_elephant_config]
operation = "replace"
target = "animals/elephant.ai"
source = "patches/elephant.ai"
```

#### Merge INI Files
```toml
[patches.merge_game_settings]
operation = "merge"
target = "config/settings.ini"
source = "patches/settings.ini"
merge_mode = "patch_priority"  # or "base_priority" (default: "patch_priority")
```

**Merge Modes:**
- `patch_priority` (default): Patch file values overwrite existing values when keys conflict
- `base_priority`: Existing values preserved, patch only adds new keys

#### Delete File
```toml
[patches.remove_deprecated_animal]
operation = "delete"
target = "animals/oldanimal.ai"
```

#### Set Animation Palette Reference
```toml
[patches.update_elephant_palette]
operation = "set_palette"
target = "animals/elephant/adult/male/n"  # Animation file (no extension)
palette = "animals/elephant/custom.pal"   # New palette file path
```

**Purpose**: Changes the palette filename reference inside an animation file header without modifying animation data.

**Requirements**:
- Target must be an animation file (no extension: 'N', 'S', 'NE', etc.)
- Palette path must end with `.pal` extension
- Palette file must exist in mod archive
- Target file must exist in resource system
- Target file must be parseable as a valid animation

**Use Cases**:
- Swapping color palettes for animals/objects
- HD texture mod palette updates
- Seasonal/themed palette variations

**Conditional Example**:
```toml
[patches.hd_palette_swap]
operation = "set_palette"
target = "animals/elephant/n"
palette = "animals/elephant/hd.pal"
condition.mod_loaded = "HDTexturePack"
```

### Element-Level Operations (INI Files Only)

#### Set Single Key-Value
```toml
[patches.increase_resolution]
operation = "set_key"
target = "config/settings.ini"
section = "Graphics"
key = "Resolution"
value = "1920x1080"
```

#### Set Multiple Keys in Section
```toml
[patches.configure_audio]
operation = "set_keys"
target = "config/settings.ini"
section = "Audio"
keys = { Volume = "100", Enabled = "true", MusicVolume = "80" }
```

#### Append Value to Array (Repeated Key)
```toml
[patches.add_swim_behavior]
operation = "append_value"
target = "animals/elephant.ai"
section = "Behaviors"
key = "Action"
value = "swim"
```

#### Append Multiple Values
```toml
[patches.add_elephant_behaviors]
operation = "append_values"
target = "animals/elephant.ai"
section = "Behaviors"
key = "Action"
values = ["climb", "jump", "run"]
```

#### Remove Single Key
```toml
[patches.remove_debug_log_level]
operation = "remove_key"
target = "config/settings.ini"
section = "Debug"
key = "LogLevel"
```

#### Remove Multiple Keys
```toml
[patches.cleanup_debug_settings]
operation = "remove_keys"
target = "config/settings.ini"
section = "Debug"
keys = ["LogLevel", "DebugMode", "Verbose"]
```

#### Add Section with Keys
```toml
[patches.add_new_feature_section]
operation = "add_section"
target = "config/settings.ini"
section = "NewFeature"
keys = { Enabled = "true", Value = "100" }
on_exists = "error"  # Options: "error" (default), "merge", "skip", "replace"
```

**Section Collision Handling** (`on_exists` for `add_section`):
- **`error`** (default): Fail patch if section already exists
- **`merge`**: Merge provided keys with existing section (patch keys overwrite existing)
- **`skip`**: Skip operation if section exists (log warning)
- **`replace`**: Delete existing section entirely and replace with new keys

#### Clear Section (Remove All Keys)
```toml
[patches.reset_cache_settings]
operation = "clear_section"
target = "config/settings.ini"
section = "Cache"
```

#### Remove Section Entirely
```toml
[patches.remove_deprecated_section]
operation = "remove_section"
target = "config/settings.ini"
section = "Deprecated"
```

### Conditional Patching

Patches can include conditions at two levels:

1. **File-level conditions** (`[patch_meta.condition]` table): If these fail, the entire patch file is skipped
2. **Patch-level conditions** (within individual patches): If these fail, only that specific patch is skipped

```toml
# Top-level condition - entire file skipped if this fails
[patch_meta.condition]
mod_loaded = "RequiredExpansion"

# Individual patch with its own condition
[patches.dolphin_compat_fix]
operation = "set_key"
target = "animals/dolphin.ai"
section = "Habitat"
key = "WaterDepth"
value = "10"
condition.mod_loaded = "DolphinExpansion"  # Only this patch skipped if fails

# Only patch if a key exists
[patches.fix_legacy_setting]
operation = "set_key"
target = "config/settings.ini"
section = "Graphics"
key = "AntiAliasing"
value = "4x"
condition.key_exists = { section = "Graphics", key = "AntiAliasing" }

# Only patch if key has specific value
[patches.upgrade_old_format]
operation = "set_key"
target = "config/settings.ini"
section = "Graphics"
key = "TextureQuality"
value = "high"
condition.value_equals = { section = "Graphics", key = "TextureQuality", value = "medium" }

# Multiple conditions (all must be true - AND logic)
[patches.complex_conditional]
operation = "merge"
target = "animals/elephant.ai"
source = "patches/elephant_advanced.ai"
condition.mod_loaded = "AdvancedAI"
condition.key_exists = { section = "AI", key = "AdvancedMode" }
condition.value_equals = { section = "AI", key = "AdvancedMode", value = "true" }
```

**Condition Types**:
- **`mod_loaded`**: Check if mod ID is loaded (string value)
- **`key_exists`**: Check if section and key exist (table with `section` and `key` fields)
- **`value_equals`**: Check if key has specific value (table with `section`, `key`, and `value` fields)

**Condition Logic**: All conditions must pass (AND logic). For OR logic, create separate patches or separate patch files.

## Cross-Mod Patching

Patches operate on the **current state** of files: base game files with all previously loaded mods and their patches already applied. This means mods patch a cumulative state, not individual mod files.

```toml
# Target field matches files from current state (base game + previous mods)
[patches.buff_elephant_speed]
operation = "set_key"
target = "animals/elephant.ai"  # Matches current state, not specific mod
section = "Stats"
key = "Speed"
value = "15"
```

**Note**: The `target` field matches against the current resource state. Mod load order (handled elsewhere) determines which version of a file is "current" when this patch runs.

## Edge Case Behavior

### File Not Found
- **Replace/Merge/Element ops**: Error (can't modify non-existent file)
- **Delete**: Warning + Skip (already doesn't exist)

### Section Not Found
- **set_key/set_keys/append_***: Create section automatically
- **remove_section/clear_section/remove_key/remove_keys**: Warning + Skip

### Key Not Found
- **set_key/set_keys/append_***: Create key (normal behavior)
- **remove_key/remove_keys**: Warning + Skip

### Invalid File Types
- Element-level operations only work on INI-like files: `.ini`, `.ai`, `.cfg`, `.uca`, `.ucs`, `.ucb`, `.scn`, `.lyt`
- Attempting element ops on other files: Error with clear message

### Animation Palette Reference (set_palette operation)
- **Target has extension**: Error - only extensionless animation files supported
- **Palette missing .pal extension**: Error - palette must be .pal file
- **Palette not in resources**: Error during validation - file must exist
- **Target not found**: Error - cannot modify non-existent file
- **Animation parsing fails**: Error - file is not a valid animation or is corrupted

## Implementation Notes

### Patch Naming
- Each patch must have a unique name within a `patch.toml` file
- Patch names are used in logs and error messages for debugging
- Names should be descriptive (e.g., `fix_elephant_speed` not `patch1`)
- Names use snake_case by convention but any valid TOML key is allowed

### Patch Application Order
- Patches execute in mod load order (based on zoo.ini path order)
- No explicit priority field - load order determines execution
- Within a mod, patches execute in the order they appear in patch.toml (preserved by `IndexMap`)

### Case Sensitivity
- INI operations are case-insensitive by default (matching Zoo Tycoon behavior)
- Section and key names automatically lowercased for lookups
- File paths are case-insensitive

### Array Handling
- Zoo Tycoon INI files support repeated keys to create arrays
- `append_value`/`append_values` operations add to these arrays
- `set_key`/`set_keys` operations replace the entire value (array becomes single value)

## Data Structures (Rust)

All patch structures are defined in `openzt/src/mods.rs`:

```rust
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Getters)]
#[get = "pub"]
pub struct ModDefinition {
    habitats: Option<HashMap<String, IconDefinition>>,
    locations: Option<HashMap<String, IconDefinition>>,

    // Patch system - split into metadata and patches
    patch_meta: Option<PatchMeta>,
    patches: Option<IndexMap<String, Patch>>,  // MUST use IndexMap for order preservation
}

/// Metadata for patch configuration
#[derive(Deserialize, Debug, Clone)]
pub struct PatchMeta {
    /// File-level on_error directive for error handling
    #[serde(default = "default_on_error")]
    pub on_error: ErrorHandling,

    /// File-level conditions - if these fail, all patches are skipped
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

impl Default for PatchMeta {
    fn default() -> Self {
        PatchMeta {
            on_error: ErrorHandling::Continue,
            condition: None,
        }
    }
}

pub enum ErrorHandling {
    Continue,   // Continue to next patch (default)
    Abort,      // Stop processing this patch file
    AbortMod,   // Stop loading this mod
}

pub enum Patch {
    Replace(ReplacePatch),
    Merge(MergePatch),
    Delete(DeletePatch),
    SetPalette(SetPalettePatch),
    SetKey(SetKeyPatch),
    SetKeys(SetKeysPatch),
    AppendValue(AppendValuePatch),
    AppendValues(AppendValuesPatch),
    RemoveKey(RemoveKeyPatch),
    RemoveKeys(RemoveKeysPatch),
    AddSection(AddSectionPatch),
    ClearSection(ClearSectionPatch),
    RemoveSection(RemoveSectionPatch),
}

pub enum MergeMode {
    PatchPriority,  // Patch overwrites existing (default)
    BasePriority,   // Existing preserved, patch adds new only
}

pub enum OnExists {
    Error,    // Fail if section exists (default)
    Merge,    // Merge keys with existing section
    Skip,     // Skip if exists
    Replace,  // Replace entire section
}

pub struct PatchCondition {
    pub mod_loaded: Option<String>,
    pub key_exists: Option<KeyCheck>,
    pub value_equals: Option<ValueCheck>,
}

pub struct SetPalettePatch {
    pub target: String,                    // Animation file path
    pub palette: String,                   // Palette file path (.pal)
    pub condition: Option<PatchCondition>, // Optional conditions
}
```

Each operation struct contains:
- `target`: String - target file path
- `source`: String (for replace/merge) - source file path within mod
- `palette`: String (for set_palette) - palette file path
- `condition`: Option<PatchCondition> - optional conditions
- Operation-specific fields (section, key, value, etc.)

**Implementation Notes**:
- Patches are stored as `patches: Option<IndexMap<String, Patch>>` in `ModDefinition`
- Metadata is stored separately as `patch_meta: Option<PatchMeta>`
- The `IndexMap` preserves patch order while allowing access by name for logging
- If only `patches` is specified without `patch_meta`, default values are used (on_error=Continue, no conditions)

## Future Extensions

Potential enhancements not yet implemented:

1. **Inline Content**: Small patches without separate files
   ```toml
   [patches.inline_merge]
   operation = "merge_inline"
   target = "config/settings.ini"
   content = "[Graphics]\nResolution=1920x1080"
   ```

2. **Array Element Operations**: Target specific array indices or value matching
3. **Patch Validation Tool**: CLI tool to validate patch.toml files before packaging
4. **Pattern Matching**: Wildcard file targeting (e.g., `target = "animals/*.ai"`)

## Testing

A test patch.toml file is provided at `openzt/resources/test/patch.toml` with examples of all operation types. The deserialization test (`test_parse_patches`) validates the schema parsing.

## See Also

- Full implementation plan: `.claude/plans/rippling-riding-rabbit.md`
- Mod metadata schema: See `meta.toml` examples in `openzt/resources/test/`
- Zoo Tycoon INI format: Handled by `openzt-configparser` crate
