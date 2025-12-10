# OpenZT Patch System - Schema Documentation

## Overview

This document describes the patch system TOML schema for OpenZT, which allows modders to define operations on Zoo Tycoon mod files. The schema enables mods that alter the same files to coexist through file-level and element-level operations.

## Current Implementation Status

### ✅ Completed
1. **Phase 1: Data Structures** - All patch types defined in `openzt/src/mods.rs`
   - `PatchFile`, `Patch` enum with 12 operation variants
   - `MergeMode` enum for merge priority control
   - TOML deserialization working and tested

2. **Phase 2: INI Parser Enhancements** - New methods in `openzt-configparser/src/ini.rs`
   - `has_section()` - check if section exists
   - `clear_section()` - remove all keys from section
   - `merge_with_priority()` - merge with configurable priority (patch_priority vs base_priority)
   - `MergeMode` enum exported from configparser

### 🚧 Remaining Work
3. **Phase 3-6: Patch Application System** (not yet implemented)
   - Load patch.toml from mod archives
   - Implement file-level operations (replace, merge, delete)
   - Implement element-level operations (set_key, append_value, etc.)
   - Resource map integration
   - Error handling and logging
   - Patch orchestration in mod load order

7. **Phase 7: Comprehensive Testing** (not yet implemented)
   - Integration tests for all patch types
   - Edge case testing (missing files, sections, keys)
   - Cross-mod patching tests

## TOML Schema Reference

### Named Patches

Patches use named subtables with the syntax `[patch.patch_name]`. Each patch must have a unique name that will be used in logs and error messages. Patches are executed in the order they appear in the file (order is preserved via `IndexMap`).

### File-Level Operations

#### Replace Entire File
```toml
[patch.replace_elephant_config]
operation = "replace"
target = "animals/elephant.ai"
source = "patches/elephant.ai"
target_mod = "base_game"  # Optional
```

#### Merge INI Files
```toml
[patch.merge_game_settings]
operation = "merge"
target = "config/settings.ini"
source = "patches/settings.ini"
merge_mode = "patch_priority"  # or "base_priority" (default: "patch_priority")
target_mod = "other_mod"  # Optional
```

**Merge Modes:**
- `patch_priority` (default): Patch file values overwrite existing values when keys conflict
- `base_priority`: Existing values preserved, patch only adds new keys

#### Delete File
```toml
[patch.remove_deprecated_animal]
operation = "delete"
target = "animals/oldanimal.ai"
target_mod = "some_mod"  # Optional
```

### Element-Level Operations (INI Files Only)

#### Set Single Key-Value
```toml
[patch.increase_resolution]
operation = "set_key"
target = "config/settings.ini"
section = "Graphics"
key = "Resolution"
value = "1920x1080"
```

#### Set Multiple Keys in Section
```toml
[patch.configure_audio]
operation = "set_keys"
target = "config/settings.ini"
section = "Audio"
keys = { Volume = "100", Enabled = "true", MusicVolume = "80" }
```

#### Append Value to Array (Repeated Key)
```toml
[patch.add_swim_behavior]
operation = "append_value"
target = "animals/elephant.ai"
section = "Behaviors"
key = "Action"
value = "swim"
```

#### Append Multiple Values
```toml
[patch.add_elephant_behaviors]
operation = "append_values"
target = "animals/elephant.ai"
section = "Behaviors"
key = "Action"
values = ["climb", "jump", "run"]
```

#### Remove Single Key
```toml
[patch.remove_debug_log_level]
operation = "remove_key"
target = "config/settings.ini"
section = "Debug"
key = "LogLevel"
```

#### Remove Multiple Keys
```toml
[patch.cleanup_debug_settings]
operation = "remove_keys"
target = "config/settings.ini"
section = "Debug"
keys = ["LogLevel", "DebugMode", "Verbose"]
```

#### Add Section with Keys
```toml
[patch.add_new_feature_section]
operation = "add_section"
target = "config/settings.ini"
section = "NewFeature"
keys = { Enabled = "true", Value = "100" }  # Optional
```

#### Clear Section (Remove All Keys)
```toml
[patch.reset_cache_settings]
operation = "clear_section"
target = "config/settings.ini"
section = "Cache"
```

#### Remove Section Entirely
```toml
[patch.remove_deprecated_section]
operation = "remove_section"
target = "config/settings.ini"
section = "Deprecated"
```

## Cross-Mod Patching

All operations support an optional `target_mod` field to specify which mod's file to patch:

```toml
[patch.buff_elephant_speed]
operation = "set_key"
target = "animals/elephant.ai"
target_mod = "base_game"  # Patch the base game's file
section = "Stats"
key = "Speed"
value = "15"
```

- If `target_mod` is omitted, the patch targets any matching file (first match wins)
- If `target_mod` is specified, only that mod's file is patched

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

pub struct PatchFile {
    pub patches: IndexMap<String, Patch>,  // Preserves insertion order
}

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

pub enum MergeMode {
    PatchPriority,  // Patch overwrites existing (default)
    BasePriority,   // Existing preserved, patch adds new only
}
```

Each operation struct contains:
- `target`: String - target file path
- `source`: String (for replace/merge) - source file path within mod
- `target_mod`: Option<String> - optional mod to target
- Operation-specific fields (section, key, value, etc.)

The `IndexMap` preserves patch order while allowing access by name for logging and error messages.

## Future Extensions

Potential enhancements not yet implemented:

1. **Conditional Patches**: Apply only if another mod is loaded
   ```toml
   [[patch]]
   operation = "set_key"
   condition = { mod_loaded = "graphics_enhancement_pack" }
   ...
   ```

2. **Inline Content**: Small patches without separate files
   ```toml
   [[patch]]
   operation = "merge_inline"
   target = "config/settings.ini"
   content = "[Graphics]\nResolution=1920x1080"
   ```

3. **Array Element Operations**: Target specific array indices or value matching
4. **Patch Validation Tool**: CLI tool to validate patch.toml files before packaging

## Testing

A test patch.toml file is provided at `openzt/resources/test/patch.toml` with examples of all operation types. The deserialization test (`test_parse_patches`) validates the schema parsing.

## See Also

- Full implementation plan: `.claude/plans/rippling-riding-rabbit.md`
- Mod metadata schema: See `meta.toml` examples in `openzt/resources/test/`
- Zoo Tycoon INI format: Handled by `openzt-configparser` crate
