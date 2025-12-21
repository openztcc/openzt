# OpenZT Patch System - TOML Schema Design & Implementation Plan

## Overview

Design and implement a comprehensive TOML schema for patch operations on Zoo Tycoon mod files, allowing modders to modify game files at both file-level and element-level granularity. This enables mods that alter the same files to coexist.

## Implementation Status

### ✅ Completed (Phases 1-2)
- **Phase 1**: All patch data structures implemented in `openzt/src/mods.rs`
  - 13 operation variants (Replace, Merge, Delete, SetPalette, SetKey, SetKeys, AppendValue, AppendValues, RemoveKey, RemoveKeys, AddSection, ClearSection, RemoveSection)
  - `MergeMode` enum (PatchPriority, BasePriority)
  - Named patch syntax using `IndexMap<String, Patch>` for better logging
  - TOML deserialization working with tests passing
  - Dependencies: `indexmap` with serde, `toml` with preserve_order
  - **SetPalette** operation added for modifying animation file palette references

- **Phase 2**: INI parser enhancements in `openzt-configparser/src/ini.rs`
  - `has_section()` - check section existence
  - `clear_section()` - remove all keys from section
  - `merge_with_priority()` - merge with configurable priority
  - All methods handle case-sensitivity correctly

- **Documentation**: Complete schema documentation in `PATCH_SCHEMA.md`

### 🚧 Remaining (Phases 3-7)

**NOTE**: Phases 1-2 were completed before adding new features from scrutiny feedback. The following phases need to incorporate:
- File-level error handling (on_error: continue/abort/abort_mod)
- Conditional patching (mod_loaded, key_exists, value_equals)
- Section collision handling (on_exists for add_section)

- Phase 3: File-level patch operations (replace, merge, delete, set_palette) + conditional support ✅
- Phase 4: Element-level patch operations (set_key, append_value, etc.) + conditional support + on_exists for add_section ✅
- Phase 5: Resource map update methods (already handled in Phase 3/4) ✅
- Phase 6: Patch orchestration, conditional evaluation, and error handling (on_error modes) ✅
- **Phase 6.5: Core Improvements** ✅
  - Add condition.target field for top-level conditionals
  - Wire up apply_patches integration to handle_ztd()
  - Code quality improvements (DRY validation, remove dead code)
  - Validate only on_error="continue" is used (others not yet supported)
- **Phase 6.6: Snapshot/Rollback & Dry-Run** (FUTURE PLAN)
  - Implement selective snapshot/rollback for abort_mod
  - Add dry_run flag support
  - Enable abort and abort_mod error handling modes
- **Phase 6.7: ModDefinition Refactoring** ✅
  - Split `PatchFile` into `PatchMeta` and separate patches field
  - Update `ModDefinition` to use `patch_meta` and `patches` fields
  - Refactor test code to load patches as part of ModDefinition
  - Update TOML structure to use `[patch_meta]` section
- **Phase 6.8: Variable Substitution** ✅ (Commit 0e830ba)
  - Implement `{variable}` syntax in patch values
  - Support habitat, location, and string variable types
  - Current mod and cross-mod reference support
  - Variable resolution in set_key, set_keys, append_value, append_values, add_section operations
  - Comprehensive error handling for undefined variables, invalid syntax, and mod loading issues
  - Unit tests for parsing and substitution logic
- Phase 7: Comprehensive integration tests (including conditionals, error modes, on_exists)

## Requirements Summary

Based on user requirements:
- **Merge priority**: Configurable per patch - patch file can take precedence (overwrite) or base file can take precedence (preserve existing)
- **Patch scope**: Can target any file (base game, other mods, same mod)
- **Element operations**: Section-level and key-level operations
- **Ordering**: Patches execute in mod load order (no explicit priority)

## Patch Lifecycle

**When patches are applied**:
- Patches execute during mod loading at game startup
- Each mod's patches run when that mod is loaded (order determined by mod load system, out of scope for this plan)
- Patches operate on the cumulative state: base game + all previously loaded mods + their patches

**Execution flow**:
1. Base game files loaded into resource system
2. Mod A loads → Mod A's patches execute → Mod A's files added to resources
3. Mod B loads → Mod B's patches execute (can patch base game OR Mod A's files) → Mod B's files added
4. Mod C loads → patches can target any file from base/Mod A/Mod B → etc.

**Key implications**:
- A mod loaded later can patch files from earlier mods
- Patches are NOT re-applied dynamically during gameplay
- `target` paths match against current resource state, not specific mod IDs
- Mod load order is critical for patch compatibility (handled elsewhere in OpenZT)

## Proposed TOML Schema

### Patch File Structure

Each patch file has a top-level configuration section followed by named patch operations:

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

**Top-Level Conditions** (`[patch_meta.condition]` table):
- Optional conditions that apply to the entire file
- If top-level conditions fail, the entire patch file is skipped (logged as warning)
- Uses same condition types as individual patches: `mod_loaded`, `key_exists`, `value_equals`
- All conditions must pass (AND logic)

### Named Patches

Patches use named subtables with the syntax `[patches.patch_name]`. Each patch must have a unique name used in logs and error messages. Patches execute in the order they appear in the file (order preserved via `IndexMap`).

### File-Level Operations

```toml
# Replace entire file
[patches.replace_elephant_config]
operation = "replace"
target = "animals/elephant.ai"
source = "patches/elephant.ai"

# Merge INI files (with configurable priority)
[patches.merge_game_settings]
operation = "merge"
target = "config/settings.ini"
source = "patches/settings.ini"
merge_mode = "patch_priority"  # or "base_priority" (defaults to "patch_priority")

# Delete file
[patches.remove_old_animal]
operation = "delete"
target = "animals/oldanimal.ai"
```

### Variable Substitution

Patch values support variable substitution using `{variable}` syntax (added in Phase 6.8). Variables are resolved at patch application time:

**Variable Types**:
- `{habitat.name}` - Current mod's habitat string ID
- `{location.name}` - Current mod's location string ID
- `{string.id}` - String content from registry
- `{mod.habitat.name}` - Cross-mod habitat reference
- `{mod.location.name}` - Cross-mod location reference

**Examples**:
```toml
# Reference habitat ID
[patches.set_habitat]
operation = "set_key"
target = "animals/elephant.ai"
section = "Habitat"
key = "cHabitat"
value = "{habitat.savanna}"  # Resolves to habitat's string ID

# Cross-mod reference
[patches.cross_mod_habitat]
operation = "set_key"
target = "animals/alien.ai"
section = "Habitat"
key = "cHabitat"
value = "{lunar.habitat.crater}"  # Reference another mod's habitat

# Multiple variables
[patches.configure_exhibit]
operation = "set_keys"
target = "exhibits/moon.cfg"
section = "Config"
keys = {
    cHabitat = "{habitat.lunar}",
    cLocation = "{location.moon}",
    NameID = "{string.10500}"
}
```

**Supported in**: set_key, set_keys, append_value, append_values, add_section operations

### Element-Level Operations (INI files only)

```toml
# Set single key-value
[patches.increase_resolution]
operation = "set_key"
target = "config/settings.ini"
section = "Graphics"
key = "Resolution"
value = "1920x1080"

# Set multiple keys in one section
[patches.configure_audio]
operation = "set_keys"
target = "config/settings.ini"
section = "Audio"
keys = { Volume = "100", Enabled = "true" }

# Append to array (repeated key)
[patches.add_swim_behavior]
operation = "append_value"
target = "animals/elephant.ai"
section = "Behaviors"
key = "Action"
value = "swim"

# Append multiple values
[patches.add_elephant_behaviors]
operation = "append_values"
target = "animals/elephant.ai"
section = "Behaviors"
key = "Action"
values = ["climb", "jump"]

# Remove key(s)
[patches.remove_debug_log_level]
operation = "remove_key"
target = "config/settings.ini"
section = "Debug"
key = "LogLevel"

[patches.cleanup_debug_settings]
operation = "remove_keys"
target = "config/settings.ini"
section = "Debug"
keys = ["LogLevel", "DebugMode"]

# Section operations
[patches.add_new_feature_section]
operation = "add_section"
target = "config/settings.ini"
section = "NewFeature"
keys = { Enabled = "true", Value = "100" }
on_exists = "error"  # Options: "error" (default), "merge", "skip", "replace"

[patches.reset_cache_settings]
operation = "clear_section"  # Remove all keys, keep section
target = "config/settings.ini"
section = "Cache"

[patches.remove_deprecated_section]
operation = "remove_section"  # Remove entire section
target = "config/settings.ini"
section = "Deprecated"
```

**Section Collision Handling** (`on_exists` for `add_section`):
- **`error`** (default): Fail patch if section already exists
- **`merge`**: Merge provided keys with existing section (patch keys overwrite existing)
- **`skip`**: Skip operation if section exists (log warning)
- **`replace`**: Delete existing section entirely and replace with new keys

### Cross-Mod Patching

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

## Schema Versioning

To support future breaking changes to patch and mod formats, OpenZT uses schema versioning in `meta.toml`. Each mod declares which schema versions it uses:

```toml
# In meta.toml
[schema_version]
patch = "1.0"       # Patch system schema version
meta = "1.0"        # meta.toml format version
habitat = "1.0"     # Habitat definition schema (if applicable)
location = "1.0"    # Location definition schema (if applicable)
```

**Current Versions** (as of this plan):
- `patch`: `"1.0"` - Patch system with operations, conditions, error handling
- `meta`: `"1.0"` - Basic meta.toml with id, name, version, author, description
- `habitat`: `"1.0"` - Habitat definition format (if mod includes habitats)
- `location`: `"1.0"` - Location definition format (if mod includes locations)

**Version Checking**:
- OpenZT logs warnings for unknown schema versions
- Future versions may reject incompatible schemas
- Omitted schema_version entries assume latest version (risky for forward compatibility)

## Rust Data Structures

### Core Types

```rust
// In openzt/src/mods.rs
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

// Variable substitution types (in openzt/src/resource_manager/openzt_mods/patches.rs)
enum VariableType {
    Habitat,   // {habitat.name} or {mod.habitat.name}
    Location,  // {location.name} or {mod.location.name}
    String,    // {string.id}
}

struct ParsedVariable {
    var_type: VariableType,
    mod_id: Option<String>,  // None = current mod, Some = cross-mod reference
    identifier: String,      // habitat/location name or string ID
}

pub struct SubstitutionContext {
    pub current_mod_id: String,
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

fn default_on_error() -> ErrorHandling {
    ErrorHandling::Continue
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorHandling {
    Continue,   // Continue to next patch (default)
    Abort,      // Stop processing this patch file
    AbortMod,   // Stop loading this mod
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Patch {
    Replace(ReplacePatch),
    Merge(MergePatch),
    Delete(DeletePatch),
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

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MergeMode {
    PatchPriority,  // Patch file values overwrite existing values (default)
    BasePriority,   // Existing file values preserved, patch only adds new keys
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnExists {
    Error,    // Fail if section exists (default)
    Merge,    // Merge keys with existing section
    Skip,     // Skip if exists
    Replace,  // Replace entire section
}

fn default_on_exists() -> OnExists {
    OnExists::Error
}

// Conditional patch support
#[derive(Deserialize, Debug, Clone)]
pub struct PatchCondition {
    #[serde(default)]
    pub mod_loaded: Option<String>,
    #[serde(default)]
    pub key_exists: Option<KeyCheck>,
    #[serde(default)]
    pub value_equals: Option<ValueCheck>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyCheck {
    pub section: String,
    pub key: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ValueCheck {
    pub section: String,
    pub key: String,
    pub value: String,
}
```

**Dependencies** (in `openzt/Cargo.toml`):
- `indexmap = { version = "2", features = ["serde"] }`
- `toml = { version = "0.9.0", features = ["preserve_order"] }`

The `IndexMap` preserves patch order while allowing access by name for logging. The `preserve_order` feature on `toml` ensures deterministic deserialization order.

### Operation Structs

All operation structs follow this pattern with optional `condition` field for conditional patching:

```rust
#[derive(Deserialize, Debug, Clone)]
pub struct SetKeyPatch {
    pub target: String,
    pub section: String,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddSectionPatch {
    pub target: String,
    pub section: String,
    pub keys: HashMap<String, String>,
    #[serde(default = "default_on_exists")]
    pub on_exists: OnExists,
    #[serde(default)]
    pub condition: Option<PatchCondition>,
}

// Similar pattern for all other patch types...
```

**Note**: The `target_mod` field concept was removed in favor of targeting the cumulative resource state.

## Edge Case Handling

### File Not Found
- **Replace/Merge/Element ops**: Error (can't modify non-existent file)
- **Delete**: Warning + Skip (already doesn't exist)

### Section Not Found
- **set_key/append_value**: Create section automatically
- **remove_section/clear_section**: Warning + Skip

### Key Not Found
- **set_key/append_value**: Create key (normal behavior)
- **remove_key**: Warning + Skip

### Invalid File Types
- Element-level operations on non-INI files: Error with clear message
- Valid extensions: `.ini`, `.ai`, `.cfg`, `.uca`, `.ucs`, `.ucb`, `.scn`, `.lyt`

### Case Sensitivity
Section and key names in patches follow the INI parser's case-insensitive behavior:
- Patch targeting `section = "Graphics"` matches `[graphics]`, `[GRAPHICS]`, `[Graphics]`
- Key lookups are similarly case-insensitive
- Existing case is preserved when modifying files (no normalization)
- When creating new sections/keys, use the case specified in the patch

**Example**:
```toml
# This patch works regardless of case in target file
[patches.fix_audio]
operation = "set_key"
section = "AUDIO"      # Matches [audio], [Audio], [AUDIO]
key = "Volume"         # Matches volume, VOLUME, Volume
value = "100"
```

### Array Semantics
In Zoo Tycoon INI files, arrays are represented as sequentially repeated keys:
```ini
[Behaviors]
Action = walk
Action = eat
Action = sleep
```

**Array Operations**:
- **`append_value`/`append_values`**: Always appends to the array, even if only one value currently exists
  - If target has `Action = walk` (single value), appending creates `Action = walk` + `Action = swim` (array)
  - Sequential repeated keys are automatically treated as arrays by the INI parser
- **`set_key`**: Replaces ALL instances of a key with a single value (destroys array)
  - Use `append_value` to add to arrays, not `set_key`
- **`remove_key`**: Removes ALL instances of a key (entire array)

**Example**:
```toml
# Correct: Add to array
[patches.add_behavior]
operation = "append_value"
key = "Action"
value = "swim"

# Wrong: This replaces entire array with single value!
[patches.wrong_behavior]
operation = "set_key"
key = "Action"
value = "swim"  # Destroys walk/eat/sleep
```

## Implementation Plan

### Phase 1: Data Structures & Parsing ✅ (COMPLETED)
**File**: `openzt/src/mods.rs`
1. ✅ Add all patch struct definitions:
   - Patch enum with 12 operation variants
   - MergeMode enum (PatchPriority, BasePriority)
   - ErrorHandling enum (Continue, Abort, AbortMod)
   - OnExists enum (Error, Merge, Skip, Replace)
   - PatchCondition struct with mod_loaded, key_exists, value_equals
   - All operation structs with condition fields
2. ✅ Add patch loading during mod initialization
3. ✅ Implement TOML deserialization tests

**Structural improvements**:
- Split `PatchFile` into `PatchMeta` (metadata) and separate patches field (Phase 6.7)
- `ModDefinition` now has `patch_meta: Option<PatchMeta>` and `patches: Option<IndexMap<String, Patch>>`
- TOML structure uses `[patch_meta]` section for on_error and conditions
- Test code updated to load patches as part of ModDefinition

**New additions from scrutiny feedback**:
- Top-level on_error field in `[patch_meta]` (file-level error handling)
- Top-level condition field in `[patch_meta.condition]` (file-level conditionals)
- Conditional patching support (PatchCondition, KeyCheck, ValueCheck)
- AddSectionPatch.on_exists field for collision handling
- Renamed from [patch.*] to [patches.*] syntax

### Phase 2: INI Parser Enhancements
**File**: `openzt-configparser/src/ini.rs`
1. Add `merge_with_priority(&mut self, other: &Ini, mode: MergeMode)` - merge with configurable priority (patch overwrites vs base preserved)
2. Add `clear_section(&mut self, section: &str)` - remove all keys from section
3. Add `has_section(&self, section: &str) -> bool` - check section existence
4. Add `remove_section(&mut self, section: &str)` - remove entire section
5. Add tests for new methods

### Phase 3: File-Level Operations
**File**: `openzt/src/resource_manager/legacy_loading.rs` (line 97 TODO)
1. Implement `apply_replace_patch()` - replace file in LazyResourceMap
2. Implement `apply_merge_patch()` - load both files, merge INIs with merge_mode, update resource map
3. Implement `apply_delete_patch()` - remove from LazyResourceMap
4. Implement `apply_set_palette_patch()` - modify animation file palette reference using `modify_ztfile_as_animation()` and `Animation::set_palette_filename()`
5. Add logging for each operation

### Phase 4: Element-Level Operations ✅ (COMPLETED)
**File**: `openzt/src/resource_manager/legacy_loading.rs` → `openzt/src/resource_manager/openzt_mods/patches.rs`
1. ✅ Implement `apply_set_key_patch()` - load INI, modify, save back (with variable substitution support)
2. ✅ Implement `apply_set_keys_patch()` - batch key updates (with variable substitution support)
3. ✅ Implement `apply_append_value_patch()` - add to array (with variable substitution support)
4. ✅ Implement `apply_append_values_patch()` - add multiple values (with variable substitution support)
5. ✅ Implement `apply_remove_key_patch()` - remove key
6. ✅ Implement `apply_remove_keys_patch()` - batch key removal
7. ✅ Implement `apply_add_section_patch()` - create section with keys, on_exists handling (with variable substitution support)
8. ✅ Implement `apply_clear_section_patch()` - clear all keys
9. ✅ Implement `apply_remove_section_patch()` - remove section
10. ✅ Helper functions for INI loading/saving

### Phase 5: Resource Map Updates
**File**: `openzt/src/resource_manager/lazyresourcemap.rs`
1. Add `update_resource()` method to replace existing resource content
2. Add `remove_resource()` method for delete operations
3. Ensure case-insensitive lookups work correctly

### Phase 6: Orchestration & Error Handling
**File**: `openzt/src/resource_manager/legacy_loading.rs`
1. Create `apply_patches(patch_file: PatchFile)` function
2. **First**, evaluate top-level conditions (if present):
   - Check top-level `condition` field in PatchFile
   - If any top-level condition fails, skip entire patch file (log warning), return early
3. Check file-level `on_error` configuration from PatchFile
4. Before applying each patch, evaluate patch-level conditions:
   - Check `mod_loaded` against loaded mod list
   - Check `key_exists` by loading target file and checking section/key
   - Check `value_equals` by comparing actual value
   - Skip patch (with warning) if any patch-level condition fails
5. Apply patches in order (iterate patches maintaining IndexMap order):
   - Match on patch variant: Replace → apply_replace_patch(), Merge → apply_merge_patch(), Delete → apply_delete_patch(), SetPalette → apply_set_palette_patch(), etc.
   - For SetPalette: validate target is animation (no extension), palette has .pal extension, both files exist, animation can be parsed
6. Handle errors according to on_error setting:
   - `Continue`: Log error, continue to next patch
   - `Abort`: Log error, stop processing this patch file, return
   - `AbortMod`: Log error, propagate error to mod loader to stop mod loading
7. Collect results (success/skip/error/condition_failed/file_skipped) with patch names
8. Log comprehensive summary including patch names in messages
9. Add validation for common mistakes (invalid file extensions, etc.)

### Phase 7: Testing
**Files**: `openzt/resources/test/patch.toml` and test code
1. Update existing `patch.toml` with comprehensive examples
2. Create test mods with various patch types
3. Test edge cases (missing files, sections, keys)
4. Test cross-mod patching (patches operating on cumulative state)
5. Test merge priority modes (patch_priority vs base_priority)
6. Test error handling modes (continue, abort, abort_mod)
7. Test conditional patching:
   - mod_loaded conditions
   - key_exists conditions
   - value_equals conditions
   - Multiple conditions (AND logic)
8. Test add_section with on_exists modes (error, merge, skip, replace)
9. Test array semantics (append to single values, append to arrays)
10. Test case-insensitive section/key matching
11. Test set_palette operation:
   - Valid animation file with valid palette
   - Error cases: target has extension, palette wrong extension, files not found
   - Animation parsing validation (invalid animation file)
   - Conditional palette swapping
12. Test variable substitution:
   - Current mod habitat/location/string references
   - Cross-mod habitat/location references
   - Mixed content (variables + literal text)
   - Multiple variables in one value
   - Error cases: undefined variables, invalid syntax, unclosed braces, mod not loaded
   - Variable substitution in all supported operations (set_key, set_keys, append_value, append_values, add_section)

## Critical Files

- `openzt/src/mods.rs` - Patch struct definitions and deserialization
- `openzt/src/resource_manager/openzt_mods/patches.rs` - Patch application orchestration, variable substitution system
- `openzt-configparser/src/ini.rs` - INI parser enhancements
- `openzt/src/resource_manager/lazyresourcemap.rs` - Resource update methods
- `openzt/src/resource_manager/openzt_mods/habitats_locations.rs` - Habitat/location ID resolution for variables
- `openzt/resources/test/patch.toml` - Updated test examples

## Design Rationale

**Named patch syntax** (`[patches.patch_name]` instead of `[[patches]]`):
- Enables better error messages ("Failed to apply patch 'fix_elephant_speed'" vs "Failed to apply patch #3")
- Makes patch files self-documenting with descriptive names
- Preserves execution order via `IndexMap` (insertion order maintained)
- Consistent with existing OpenZT pattern (habitats/locations use named subtables)
- Allows easy access by name for debugging and logging

**IndexMap vs Vec**:
- Preserves insertion order like Vec
- Allows O(1) name-based access for logging
- Natural mapping from TOML named subtables
- Requires `preserve_order` feature on `toml` crate for deterministic deserialization

**Tagged enum** (`#[serde(tag = "operation")]`): Type-safe, prevents invalid field combinations, clear error messages

**Configurable merge_mode**: File merges can either prioritize patch values (overwrite existing) or base values (preserve existing, only add new keys). Defaults to patch_priority as this is the most common use case (patches intending to change behavior). Simpler than trying to distinguish arrays from single values.

**Optional target_mod field**: Enables cross-mod patching explicitly, defaults to "any matching file" for convenience

**Separate set_key/set_keys operations**: Balance readability (single) with efficiency (batch)

**Element operations specify intent explicitly**: Operations like `append_value` clearly indicate array modification, while `set_key` indicates replacement, avoiding ambiguity about merge behavior

**Variable substitution design** (Phase 6.8):
- **Curly brace syntax** `{variable}` is familiar from shell scripting, template systems, and other configuration formats
- **Dot-separated components** make parsing simple and unambiguous (`habitat.name` vs `mod.habitat.name`)
- **Resolution at patch application time** ensures variables refer to the correct runtime state (after mods are loaded)
- **Context-aware resolution** allows relative references (current mod) and absolute references (cross-mod)
- **Type safety** via enum discriminants prevents mixing habitat/location/string lookups
- **Early validation** with clear error messages helps modders debug issues before game launch
- **Limited scope** (only in value fields of specific operations) keeps implementation simple and predictable
- **String registry integration** enables dynamic text content in patches without hardcoding strings

## Future Extensions

Potential enhancements for future schema versions:

### Dry-Run Mode Enhancement
Built-in support for previewing patch effects without applying them:
```toml
# In patch.toml
dry_run = true
```

**Benefits**:
- Test patches before deployment
- Preview what files will be modified
- Validate patch syntax and conditionals
- Safe testing in production environments

**Output**: Detailed logs showing each operation that would occur:
```
DRY RUN: Would set [Graphics]Resolution = '1920x1080' in 'config/settings.ini'
DRY RUN: Would merge 'animals/elephant.ai' with 'patches/elephant.ai' (mode: PatchPriority)
```

### Rollback & Transaction Support
Automatic snapshot/rollback for failed patches:

**Current Implementation (v1.0)**:
- Selective snapshots of affected resources before patch application
- Automatic rollback on `on_error = "abort_mod"`
- Only snapshots resources actually modified by patches (memory efficient)
- Snapshots discarded on successful completion

**Future Enhancements**:
- Manual rollback API for advanced mod authors
- Persistent transaction log for debugging
- Cross-mod dependency rollback (if Mod B depends on Mod A patches)
- Checkpoint/restore at arbitrary points in mod loading

### Array Element Operations
Fine-grained control over array elements:
```toml
# Insert at specific position
[patch.prepend_behavior]
operation = "insert_value"
position = "start"  # or "end", index number, "before:value", "after:value"
key = "Action"
value = "prepare"

# Remove specific array element by value
[patch.remove_obsolete]
operation = "remove_array_element"
key = "Action"
value = "obsolete_behavior"

# Replace specific array element
[patch.update_behavior]
operation = "replace_array_element"
key = "Action"
index = 2  # or match by value
new_value = "updated_behavior"
```

### Pattern Matching and Wildcards
Target multiple files or keys with patterns:
```toml
# Wildcard file targeting
[patches.nerf_all_animals]
operation = "set_key"
target = "animals/*.ai"  # Match all .ai files in animals/
section = "Stats"
key = "Strength"
value = "50"

# Pattern-based key removal
[patches.cleanup_debug]
operation = "remove_keys"
section = "Settings"
key_pattern = "Debug*"  # Remove all keys starting with "Debug"
```

### Value Transformations
Mathematical and string operations on existing values:
```toml
# Multiply existing value
[patches.double_speed]
operation = "transform_value"
section = "Stats"
key = "Speed"
transform = "multiply:2"

# Append to string
[patches.suffix_name]
operation = "transform_value"
section = "Info"
key = "Name"
transform = "append: (Modified)"
```

### Copy/Reference Operations
Copy values between files or sections:
```toml
[patches.sync_speeds]
operation = "copy_value"
source_file = "animals/elephant.ai"
source_section = "Stats"
source_key = "Speed"
target_section = "Stats"
target_key = "MaxSpeed"
```

### Inline Content
Small patches without separate source files:
```toml
[patches.inline_merge]
operation = "merge_inline"
target = "config/settings.ini"
content = """
[NewSection]
Key1 = Value1
Key2 = Value2
"""
```

### Patch Validation Tool
Command-line tool for modders:
```bash
openzt.bat validate-patches --mod MyMod
# Shows:
# - What files would be modified
# - What changes would be made
# - Potential conflicts with other mods
# - Errors/warnings before running game
```

### Advanced Conditionals
More complex condition logic:
```toml
# OR logic (any condition must pass)
condition.any = [
    { mod_loaded = "ModA" },
    { mod_loaded = "ModB" }
]

# NOT logic (condition must fail)
condition.not = { key_exists = { section = "Old", key = "Deprecated" } }

# Version checking
condition.openzt_version = ">= 1.5.0"
condition.mod_version = { mod = "BaseExpansion", version = ">= 2.0" }
```
