use std::path::Path;
use std::str;

use anyhow;
use openzt_configparser::ini::{Ini, MergeMode as IniMergeMode};
use tracing::{error, info, warn};

use crate::{
    animation::Animation,
    mods::{
        self,
        AddSectionPatch,
        AppendValuePatch,
        AppendValuesPatch,
        ClearSectionPatch,
        DeletePatch,
        ErrorHandling,
        MergeMode,
        MergePatch,
        OnExists,
        Patch,
        PatchCondition,
        PatchMeta,
        RemoveKeyPatch,
        RemoveKeysPatch,
        RemoveSectionPatch,
        ReplacePatch,
        SetKeyPatch,
        SetKeysPatch,
        SetPalettePatch,
    },
    resource_manager::{
        lazyresourcemap::{add_ztfile, check_file, get_file, remove_resource},
        openzt_mods::get_mod_ids,
        ztfile::{modify_ztfile_as_animation, ZTFile, ZTFileType},
    },
};

// ============================================================================
// Phase 3: File-Level Patch Operations
// ============================================================================

/// Apply a replace patch: replaces an entire file in the resource system
///
/// This loads the source file from the current mod and replaces the target file
/// in the resource map. The target file must exist, and the source file must
/// exist in the current mod.
///
/// # Arguments
/// * `patch` - The replace patch configuration
/// * `mod_path` - Path to the current mod being loaded (for resolving source files)
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if the target file doesn't exist, source file doesn't exist, or other errors occur
fn apply_replace_patch(patch: &ReplacePatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying replace patch '{}': {} -> {}", patch_name, patch.source, patch.target);

    // Check if target file exists in resource system
    if !check_file(&patch.target) {
        anyhow::bail!("Target file '{}' not found in resource system", patch.target);
    }

    // Load source file from mod
    let source_path = mod_path.join(&patch.source);
    if !source_path.exists() {
        anyhow::bail!("Source file '{}' not found in mod at path: {}", patch.source, source_path.display());
    }

    let source_data = std::fs::read(&source_path)?;
    let file_type = ZTFileType::try_from(Path::new(&patch.target))
        .map_err(|e| anyhow::anyhow!("Invalid target file type: {}", e))?;

    // Create ZTFile based on file type
    let ztfile = match file_type {
        ZTFileType::Ini | ZTFileType::Ai | ZTFileType::Ani | ZTFileType::Cfg
        | ZTFileType::Lyt | ZTFileType::Scn | ZTFileType::Uca | ZTFileType::Ucs
        | ZTFileType::Ucb | ZTFileType::Txt | ZTFileType::Toml => {
            let content = String::from_utf8(source_data)?;
            let content_len = content.len() as u32;
            let c_string = std::ffi::CString::new(content)?;
            ZTFile::Text(c_string, file_type, content_len)
        }
        _ => {
            ZTFile::RawBytes(source_data.into_boxed_slice(), file_type, 0)
        }
    };

    // Update resource (add_ztfile automatically replaces if exists)
    add_ztfile(mod_path, patch.target.clone(), ztfile)?;

    info!("Successfully applied replace patch '{}'", patch_name);
    Ok(())
}

/// Apply a merge patch: merges two INI files together
///
/// This loads both the target and source INI files, merges them according to
/// the merge_mode, and replaces the target file with the merged result.
///
/// # Arguments
/// * `patch` - The merge patch configuration
/// * `mod_path` - Path to the current mod being loaded (for resolving source files)
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if files don't exist, aren't INI files, or other errors occur
fn apply_merge_patch(patch: &MergePatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying merge patch '{}': {} + {} (mode: {:?})",
          patch_name, patch.target, patch.source, patch.merge_mode);

    // Check if target file exists
    if !check_file(&patch.target) {
        anyhow::bail!("Target file '{}' not found in resource system", patch.target);
    }

    // Validate that target is an INI-compatible file
    validate_ini_file(&patch.target)?;

    // Load target INI file
    let target_file = get_file(&patch.target)
        .ok_or_else(|| anyhow::anyhow!("Failed to load target file '{}'", patch.target))?;
    let target_str = str::from_utf8(&target_file.1)?;
    let mut target_ini = Ini::new_cs();
    target_ini.set_comment_symbols(&[';', '#', ':']);
    target_ini.read(target_str.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to parse target INI file '{}': {}", patch.target, e))?;

    // Load source INI file from mod
    let source_path = mod_path.join(&patch.source);
    if !source_path.exists() {
        anyhow::bail!("Source file '{}' not found in mod at path: {}", patch.source, source_path.display());
    }

    let source_data = std::fs::read_to_string(&source_path)?;
    let mut source_ini = Ini::new_cs();
    source_ini.set_comment_symbols(&[';', '#', ':']);
    source_ini.read(source_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse source INI file '{}': {}", patch.source, e))?;

    // Convert MergeMode enum from mods to IniMergeMode from configparser
    let ini_merge_mode = match patch.merge_mode {
        MergeMode::PatchPriority => IniMergeMode::PatchPriority,
        MergeMode::BasePriority => IniMergeMode::BasePriority,
    };

    // Merge source into target
    target_ini.merge_with_priority(&source_ini, ini_merge_mode);

    // Write merged INI to string
    let merged_content = target_ini.writes();

    // Create ZTFile and update resource
    let file_type = ZTFileType::try_from(Path::new(&patch.target))
        .map_err(|e| anyhow::anyhow!("Invalid target file type: {}", e))?;
    let c_string = std::ffi::CString::new(merged_content.clone())?;
    let ztfile = ZTFile::Text(c_string, file_type, merged_content.len() as u32);

    // Update resource (add_ztfile automatically replaces if exists)
    add_ztfile(mod_path, patch.target.clone(), ztfile)?;

    info!("Successfully applied merge patch '{}'", patch_name);
    Ok(())
}

/// Apply a delete patch: removes a file from the resource system
///
/// # Arguments
/// * `patch` - The delete patch configuration
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` always (warnings logged if file doesn't exist)
fn apply_delete_patch(patch: &DeletePatch, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying delete patch '{}': {}", patch_name, patch.target);

    if !check_file(&patch.target) {
        warn!("Delete patch '{}': target file '{}' not found (already deleted or never existed)",
              patch_name, patch.target);
        return Ok(());
    }

    let removed = remove_resource(&patch.target);
    if removed {
        info!("Successfully applied delete patch '{}' - removed '{}'", patch_name, patch.target);
    } else {
        warn!("Delete patch '{}': failed to remove '{}' (may have been removed by another operation)",
              patch_name, patch.target);
    }

    Ok(())
}

/// Apply a set_palette patch: changes the palette reference in an animation file
///
/// This modifies the palette filename stored inside an animation file's header
/// without changing the animation data itself.
///
/// # Arguments
/// * `patch` - The set_palette patch configuration
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if validation fails or animation parsing/writing fails
fn apply_set_palette_patch(patch: &SetPalettePatch, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying set_palette patch '{}': {} -> palette: {}",
          patch_name, patch.target, patch.palette);

    // Validate target has no extension (must be animation file)
    let target_path = Path::new(&patch.target);
    if target_path.extension().is_some() {
        anyhow::bail!("Target file '{}' has an extension. Animation files must have no extension.",
                     patch.target);
    }

    // Validate palette has .pal extension
    if !patch.palette.to_lowercase().ends_with(".pal") {
        anyhow::bail!("Palette file '{}' must have .pal extension", patch.palette);
    }

    // Check if target file exists
    if !check_file(&patch.target) {
        anyhow::bail!("Target animation file '{}' not found in resource system", patch.target);
    }

    // Modify the animation file's palette reference
    modify_ztfile_as_animation(&patch.target, |animation: &mut Animation| {
        animation.set_palette_filename(patch.palette.clone());
        Ok(())
    })?;

    info!("Successfully applied set_palette patch '{}' - updated palette reference to '{}'",
          patch_name, patch.palette);
    Ok(())
}

// ============================================================================
// Phase 4: Element-Level Patch Operations (INI Files)
// ============================================================================

/// Valid INI file extensions
const VALID_INI_EXTENSIONS: &[&str] = &["ini", "ai", "cfg", "uca", "ucs", "ucb", "scn", "lyt"];

/// Check if a file extension is valid for INI operations
fn is_valid_ini_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| VALID_INI_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

/// Helper function to validate that a file is an INI-compatible file
fn validate_ini_file(target: &str) -> anyhow::Result<()> {
    let path = Path::new(target);

    if !is_valid_ini_extension(path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("none");
        anyhow::bail!(
            "Target file '{}' is not an INI file (extension: {}). Valid extensions: {}",
            target,
            ext,
            VALID_INI_EXTENSIONS.join(", ")
        );
    }

    Ok(())
}

/// Helper function to load an INI file from the resource system
fn load_ini_from_resources(target: &str) -> anyhow::Result<Ini> {
    // Check if target file exists
    if !check_file(target) {
        anyhow::bail!("Target file '{}' not found in resource system", target);
    }

    // Validate file type
    validate_ini_file(target)?;

    // Load and parse INI file
    let target_file = get_file(target)
        .ok_or_else(|| anyhow::anyhow!("Failed to load target file '{}'", target))?;
    let target_str = str::from_utf8(&target_file.1)?;
    let mut ini = Ini::new_cs();
    ini.set_comment_symbols(&[';', '#', ':']);
    ini.read(target_str.to_string())
        .map_err(|e| anyhow::anyhow!("Failed to parse INI file '{}': {}", target, e))?;

    Ok(ini)
}

/// Helper function to save a modified INI file back to the resource system
fn save_ini_to_resources(target: &str, ini: &Ini, mod_path: &Path) -> anyhow::Result<()> {
    // Write INI to string
    let content = ini.writes();

    // Create ZTFile
    let file_type = ZTFileType::try_from(Path::new(target))
        .map_err(|e| anyhow::anyhow!("Invalid target file type: {}", e))?;
    let c_string = std::ffi::CString::new(content.clone())?;
    let ztfile = ZTFile::Text(c_string, file_type, content.len() as u32);

    // Update resource (add_ztfile automatically replaces if exists)
    add_ztfile(mod_path, target.to_string(), ztfile)?;

    Ok(())
}

/// Apply a set_key patch: sets a single key-value pair in an INI section
///
/// # Arguments
/// * `patch` - The set_key patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_set_key_patch(patch: &SetKeyPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying set_key patch '{}': {} [{}] {} = {}",
          patch_name, patch.target, patch.section, patch.key, patch.value);

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Set the key (creates section if it doesn't exist)
    ini.setstr(&patch.section, &patch.key, Some(&patch.value));

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied set_key patch '{}'", patch_name);
    Ok(())
}

/// Apply a set_keys patch: sets multiple key-value pairs in an INI section
///
/// # Arguments
/// * `patch` - The set_keys patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_set_keys_patch(patch: &SetKeysPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying set_keys patch '{}': {} [{}] ({} keys)",
          patch_name, patch.target, patch.section, patch.keys.len());

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Set all keys
    for (key, value) in &patch.keys {
        ini.setstr(&patch.section, key, Some(value));
    }

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied set_keys patch '{}' - set {} keys", patch_name, patch.keys.len());
    Ok(())
}

/// Apply an append_value patch: appends a single value to an array (repeated key)
///
/// # Arguments
/// * `patch` - The append_value patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_append_value_patch(patch: &AppendValuePatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying append_value patch '{}': {} [{}] {} += {}",
          patch_name, patch.target, patch.section, patch.key, patch.value);

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Append the value (creates section if it doesn't exist)
    ini.addstr(&patch.section, &patch.key, &patch.value);

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied append_value patch '{}'", patch_name);
    Ok(())
}

/// Apply an append_values patch: appends multiple values to an array
///
/// # Arguments
/// * `patch` - The append_values patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_append_values_patch(patch: &AppendValuesPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying append_values patch '{}': {} [{}] {} += {} values",
          patch_name, patch.target, patch.section, patch.key, patch.values.len());

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Append all values
    for value in &patch.values {
        ini.addstr(&patch.section, &patch.key, value);
    }

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied append_values patch '{}' - appended {} values",
          patch_name, patch.values.len());
    Ok(())
}

/// Apply a remove_key patch: removes a single key from an INI section
///
/// # Arguments
/// * `patch` - The remove_key patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully (warnings logged if key doesn't exist)
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_remove_key_patch(patch: &RemoveKeyPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying remove_key patch '{}': {} [{}] remove key '{}'",
          patch_name, patch.target, patch.section, patch.key);

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Try to remove the key
    let removed = ini.remove_key(&patch.section, &patch.key);

    if removed.is_none() {
        warn!("Remove_key patch '{}': key '{}' not found in section '{}' of '{}'",
              patch_name, patch.key, patch.section, patch.target);
        return Ok(());
    }

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied remove_key patch '{}'", patch_name);
    Ok(())
}

/// Apply a remove_keys patch: removes multiple keys from an INI section
///
/// # Arguments
/// * `patch` - The remove_keys patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully (warnings logged for missing keys)
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_remove_keys_patch(patch: &RemoveKeysPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying remove_keys patch '{}': {} [{}] remove {} keys",
          patch_name, patch.target, patch.section, patch.keys.len());

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Remove all keys, tracking successes
    let mut removed_count = 0;
    for key in &patch.keys {
        let removed = ini.remove_key(&patch.section, key);
        if removed.is_some() {
            removed_count += 1;
        } else {
            warn!("Remove_keys patch '{}': key '{}' not found in section '{}'",
                  patch_name, key, patch.section);
        }
    }

    if removed_count == 0 {
        warn!("Remove_keys patch '{}': no keys were removed (all keys not found)", patch_name);
        return Ok(());
    }

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied remove_keys patch '{}' - removed {} of {} keys",
          patch_name, removed_count, patch.keys.len());
    Ok(())
}

/// Apply an add_section patch: adds a new section with keys to an INI file
///
/// # Arguments
/// * `patch` - The add_section patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully
/// * `Err(_)` if the file doesn't exist, isn't an INI file, section collision occurs with on_exists=error, or other errors
fn apply_add_section_patch(patch: &AddSectionPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying add_section patch '{}': {} [{}] with {} keys (on_exists: {:?})",
          patch_name, patch.target, patch.section, patch.keys.len(), patch.on_exists);

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Check if section already exists
    let section_exists = ini.has_section(&patch.section);

    if section_exists {
        match patch.on_exists {
            OnExists::Error => {
                anyhow::bail!(
                    "Add_section patch '{}': section '{}' already exists in '{}' (on_exists=error)",
                    patch_name, patch.section, patch.target
                );
            }
            OnExists::Skip => {
                warn!("Add_section patch '{}': section '{}' already exists, skipping (on_exists=skip)",
                      patch_name, patch.section);
                return Ok(());
            }
            OnExists::Merge => {
                info!("Add_section patch '{}': section '{}' exists, merging keys (on_exists=merge)",
                      patch_name, patch.section);
                // Fall through to add keys (they will merge with existing)
            }
            OnExists::Replace => {
                info!("Add_section patch '{}': section '{}' exists, replacing (on_exists=replace)",
                      patch_name, patch.section);
                // Clear the section first
                ini.clear_section(&patch.section);
            }
        }
    }

    // Add all keys to the section
    for (key, value) in &patch.keys {
        ini.setstr(&patch.section, key, Some(value));
    }

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied add_section patch '{}' - {} keys added/merged",
          patch_name, patch.keys.len());
    Ok(())
}

/// Apply a clear_section patch: removes all keys from an INI section (keeps section)
///
/// # Arguments
/// * `patch` - The clear_section patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully (warnings logged if section doesn't exist)
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_clear_section_patch(patch: &ClearSectionPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying clear_section patch '{}': {} [{}]",
          patch_name, patch.target, patch.section);

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Check if section exists
    if !ini.has_section(&patch.section) {
        warn!("Clear_section patch '{}': section '{}' not found in '{}'",
              patch_name, patch.section, patch.target);
        return Ok(());
    }

    // Clear the section
    ini.clear_section(&patch.section);

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied clear_section patch '{}'", patch_name);
    Ok(())
}

/// Apply a remove_section patch: removes an entire section from an INI file
///
/// # Arguments
/// * `patch` - The remove_section patch configuration
/// * `mod_path` - Path to the current mod being loaded
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(())` if the patch was applied successfully (warnings logged if section doesn't exist)
/// * `Err(_)` if the file doesn't exist, isn't an INI file, or other errors occur
fn apply_remove_section_patch(patch: &RemoveSectionPatch, mod_path: &Path, patch_name: &str) -> anyhow::Result<()> {
    info!("Applying remove_section patch '{}': {} [{}]",
          patch_name, patch.target, patch.section);

    // Load INI file
    let mut ini = load_ini_from_resources(&patch.target)?;

    // Try to remove the section
    let removed = ini.remove_section(&patch.section);

    if removed.is_none() {
        warn!("Remove_section patch '{}': section '{}' not found in '{}'",
              patch_name, patch.section, patch.target);
        return Ok(());
    }

    // Save back to resources
    save_ini_to_resources(&patch.target, &ini, mod_path)?;

    info!("Successfully applied remove_section patch '{}'", patch_name);
    Ok(())
}

// ============================================================================
// Phase 6: Patch Orchestration, Conditional Evaluation, and Error Handling
// ============================================================================

/// Helper function to check if a mod is loaded
///
/// # Arguments
/// * `mod_id` - The mod ID to check
///
/// # Returns
/// * `true` if the mod is loaded, `false` otherwise
fn is_mod_loaded(mod_id: &str) -> bool {
    let loaded_mods = get_mod_ids();
    loaded_mods.iter().any(|id| id == mod_id)
}

/// Evaluate patch-level conditions that require target file access
///
/// # Arguments
/// * `condition` - The condition to evaluate
/// * `default_target` - Default target file path (used if condition.target is not specified)
/// * `patch_name` - Name of the patch (for logging)
///
/// # Returns
/// * `Ok(true)` if all conditions pass
/// * `Ok(false)` if any condition fails
/// * `Err(_)` if there's an error evaluating conditions
fn evaluate_patch_condition_with_target(
    condition: &Option<PatchCondition>,
    default_target: &str,
    patch_name: &str,
) -> anyhow::Result<bool> {
    let Some(cond) = condition else {
        return Ok(true);
    };

    // Use condition.target if specified, otherwise use default_target
    let target = cond.target.as_deref().unwrap_or(default_target);

    // First check mod_loaded condition (doesn't require target)
    if let Some(required_mod) = &cond.mod_loaded {
        if !is_mod_loaded(required_mod) {
            info!("Patch '{}': skipping - required mod '{}' not loaded", patch_name, required_mod);
            return Ok(false);
        }
    }

    // Check key_exists condition
    if let Some(key_check) = &cond.key_exists {
        // Check if target file exists
        if !check_file(target) {
            warn!("Patch '{}': cannot evaluate key_exists condition - target file '{}' not found",
                  patch_name, target);
            return Ok(false);
        }

        // Try to load as INI file
        match load_ini_from_resources(target) {
            Ok(ini) => {
                if ini.get(&key_check.section, &key_check.key).is_none() {
                    info!("Patch '{}': skipping - key '[{}]{}' does not exist in '{}'",
                          patch_name, key_check.section, key_check.key, target);
                    return Ok(false);
                }
            }
            Err(e) => {
                warn!("Patch '{}': cannot evaluate key_exists condition - failed to load target '{}': {}",
                      patch_name, target, e);
                return Ok(false);
            }
        }
    }

    // Check value_equals condition
    if let Some(value_check) = &cond.value_equals {
        // Check if target file exists
        if !check_file(target) {
            warn!("Patch '{}': cannot evaluate value_equals condition - target file '{}' not found",
                  patch_name, target);
            return Ok(false);
        }

        // Try to load as INI file
        match load_ini_from_resources(target) {
            Ok(ini) => {
                let actual_value = ini.get(&value_check.section, &value_check.key);
                if actual_value.as_deref() != Some(&value_check.value) {
                    info!("Patch '{}': skipping - key '[{}]{}' value does not equal '{}' (actual: {:?})",
                          patch_name, value_check.section, value_check.key, value_check.value, actual_value);
                    return Ok(false);
                }
            }
            Err(e) => {
                warn!("Patch '{}': cannot evaluate value_equals condition - failed to load target '{}': {}",
                      patch_name, target, e);
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Get the target file path from a patch (for condition evaluation)
fn get_patch_target(patch: &Patch) -> &str {
    match patch {
        Patch::Replace(p) => &p.target,
        Patch::Merge(p) => &p.target,
        Patch::Delete(p) => &p.target,
        Patch::SetPalette(p) => &p.target,
        Patch::SetKey(p) => &p.target,
        Patch::SetKeys(p) => &p.target,
        Patch::AppendValue(p) => &p.target,
        Patch::AppendValues(p) => &p.target,
        Patch::RemoveKey(p) => &p.target,
        Patch::RemoveKeys(p) => &p.target,
        Patch::AddSection(p) => &p.target,
        Patch::ClearSection(p) => &p.target,
        Patch::RemoveSection(p) => &p.target,
    }
}

/// Get the condition from a patch (for condition evaluation)
fn get_patch_condition(patch: &Patch) -> &Option<PatchCondition> {
    match patch {
        Patch::Replace(p) => &p.condition,
        Patch::Merge(p) => &p.condition,
        Patch::Delete(p) => &p.condition,
        Patch::SetPalette(p) => &p.condition,
        Patch::SetKey(p) => &p.condition,
        Patch::SetKeys(p) => &p.condition,
        Patch::AppendValue(p) => &p.condition,
        Patch::AppendValues(p) => &p.condition,
        Patch::RemoveKey(p) => &p.condition,
        Patch::RemoveKeys(p) => &p.condition,
        Patch::AddSection(p) => &p.condition,
        Patch::ClearSection(p) => &p.condition,
        Patch::RemoveSection(p) => &p.condition,
    }
}

/// Result of applying a single patch
#[derive(Debug, Clone, PartialEq)]
enum PatchResult {
    Success,
    Skipped,           // Condition failed or warning situation
    Error(String),     // Error occurred
}

/// Apply all patches with error handling and conditional evaluation
///
/// # Arguments
/// * `patch_meta` - Patch metadata containing error handling and file-level conditions
/// * `patches` - Ordered map of patches to apply (order is preserved via IndexMap)
/// * `mod_path` - Path to the current mod being loaded
///
/// # Returns
/// * `Ok(())` if patches were applied successfully (or with continue error handling)
/// * `Err(_)` if on_error=abort or on_error=abort_mod and an error occurred
pub fn apply_patches(
    patch_meta: &PatchMeta,
    patches: &indexmap::IndexMap<String, Patch>,
    mod_path: &Path
) -> anyhow::Result<()> {
    // VALIDATE: Only 'continue' mode is currently supported
    if patch_meta.on_error != ErrorHandling::Continue {
        return Err(anyhow::anyhow!(
            "Unsupported on_error mode: {:?}. Only 'continue' is currently supported. \
             Snapshot/rollback features for 'abort' and 'abort_mod' will be added in a future update.",
            patch_meta.on_error
        ));
    }

    info!("Applying patch file with {} patches (on_error: {:?})",
          patches.len(), patch_meta.on_error);

    // Step 1: Evaluate top-level conditions
    if let Some(top_level_condition) = &patch_meta.condition {
        // Check mod_loaded at file level
        if let Some(required_mod) = &top_level_condition.mod_loaded {
            if !is_mod_loaded(required_mod) {
                warn!("Patch file skipped - required mod '{}' not loaded", required_mod);
                return Ok(());
            }
        }

        // Check key_exists and value_equals with target
        if top_level_condition.key_exists.is_some() || top_level_condition.value_equals.is_some() {
            let Some(target) = &top_level_condition.target else {
                return Err(anyhow::anyhow!(
                    "Top-level condition with key_exists/value_equals requires 'target' field"
                ));
            };

            // Use existing evaluation function with target
            if !evaluate_patch_condition_with_target(&Some(top_level_condition.clone()), target, "top-level")? {
                warn!("Patch file skipped - top-level conditions failed");
                return Ok(());
            }
        }
    }

    // Step 2: Apply patches in order
    let mut results: Vec<(String, PatchResult)> = Vec::new();
    let mut patches_applied = 0;
    let mut patches_skipped = 0;
    let mut patches_failed = 0;

    for (patch_name, patch) in patches {
        info!("Processing patch '{}'", patch_name);

        // Evaluate patch-level conditions
        let target = get_patch_target(patch);
        let condition = get_patch_condition(patch);

        let condition_passed = match evaluate_patch_condition_with_target(condition, target, patch_name) {
            Ok(passed) => passed,
            Err(e) => {
                let error_msg = format!("Error evaluating condition: {}", e);
                error!("Patch '{}': {}", patch_name, error_msg);

                match patch_meta.on_error {
                    ErrorHandling::Continue => {
                        results.push((patch_name.clone(), PatchResult::Error(error_msg)));
                        patches_failed += 1;
                        continue;
                    }
                    ErrorHandling::Abort => {
                        results.push((patch_name.clone(), PatchResult::Error(error_msg)));
                        patches_failed += 1;
                        warn!("Aborting patch file due to error (on_error=abort)");
                        break;
                    }
                    ErrorHandling::AbortMod => {
                        return Err(anyhow::anyhow!("Patch '{}': {} (on_error=abort_mod)", patch_name, error_msg));
                    }
                }
            }
        };

        if !condition_passed {
            results.push((patch_name.clone(), PatchResult::Skipped));
            patches_skipped += 1;
            continue;
        }

        // Apply the patch based on its type
        let result = match patch {
            Patch::Replace(p) => apply_replace_patch(p, mod_path, patch_name),
            Patch::Merge(p) => apply_merge_patch(p, mod_path, patch_name),
            Patch::Delete(p) => apply_delete_patch(p, patch_name),
            Patch::SetPalette(p) => apply_set_palette_patch(p, patch_name),
            Patch::SetKey(p) => apply_set_key_patch(p, mod_path, patch_name),
            Patch::SetKeys(p) => apply_set_keys_patch(p, mod_path, patch_name),
            Patch::AppendValue(p) => apply_append_value_patch(p, mod_path, patch_name),
            Patch::AppendValues(p) => apply_append_values_patch(p, mod_path, patch_name),
            Patch::RemoveKey(p) => apply_remove_key_patch(p, mod_path, patch_name),
            Patch::RemoveKeys(p) => apply_remove_keys_patch(p, mod_path, patch_name),
            Patch::AddSection(p) => apply_add_section_patch(p, mod_path, patch_name),
            Patch::ClearSection(p) => apply_clear_section_patch(p, mod_path, patch_name),
            Patch::RemoveSection(p) => apply_remove_section_patch(p, mod_path, patch_name),
        };

        match result {
            Ok(()) => {
                results.push((patch_name.clone(), PatchResult::Success));
                patches_applied += 1;
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                error!("Patch '{}' failed: {}", patch_name, error_msg);

                match patch_meta.on_error {
                    ErrorHandling::Continue => {
                        results.push((patch_name.clone(), PatchResult::Error(error_msg)));
                        patches_failed += 1;
                        continue;
                    }
                    ErrorHandling::Abort => {
                        results.push((patch_name.clone(), PatchResult::Error(error_msg)));
                        patches_failed += 1;
                        warn!("Aborting patch file due to error (on_error=abort)");
                        break;
                    }
                    ErrorHandling::AbortMod => {
                        return Err(anyhow::anyhow!("Patch '{}' failed: {} (on_error=abort_mod)", patch_name, error_msg));
                    }
                }
            }
        }
    }

    // Step 3: Log comprehensive summary
    info!("Patch application complete: {} succeeded, {} skipped, {} failed",
          patches_applied, patches_skipped, patches_failed);

    for (patch_name, result) in &results {
        match result {
            PatchResult::Success => info!("  ✓ {}", patch_name),
            PatchResult::Skipped => info!("  ⊘ {} (skipped)", patch_name),
            PatchResult::Error(msg) => error!("  ✗ {}: {}", patch_name, msg),
        }
    }

    Ok(())
}
