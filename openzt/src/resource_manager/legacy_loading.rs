use std::{
    io,
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

use openzt_configparser::ini::{Ini, MergeMode as IniMergeMode};
use std::sync::LazyLock;
use regex::Regex;
use tracing::{error, info, warn};
use walkdir::WalkDir;
use zip::read::ZipFile;

use super::ztd::ZtdArchive;
use crate::{
    animation::Animation,
    mods::{self, DeletePatch, MergePatch, ReplacePatch, SetPalettePatch, MergeMode},
    resource_manager::{
        handlers::{get_handlers, RunStage},
        lazyresourcemap::{add_lazy, add_ztfile, check_file, get_file, get_file_names, get_num_resources, remove_resource},
        openzt_mods::{get_num_mod_ids, load_open_zt_mod},
        ztfile::{modify_ztfile_as_animation, ZTFile, ZTFileType},
    },
};

pub trait FromZipFile<T> {
    fn from_zip_file(file: &mut ZipFile) -> io::Result<T>;
}

pub const OPENZT_DIR0: &str = "openzt_resource";

// Note: We are excluding ztat* files until we need to override anything inside them, as they have a rediculous amount of files
fn get_ztd_resources(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut resources = Vec::new();
    if !dir.is_dir() {
        return resources;
    }
    let walker = WalkDir::new(dir).follow_links(true).max_depth(if recursive { 0 } else { 1 });
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("Error walking directory: {}", e);
                continue;
            }
        };
        let Some(filename) = entry.file_name().to_str() else {
            error!("Error getting filename: {:?}", entry);
            continue;
        };
        if filename.to_lowercase().ends_with(".ztd") && !filename.starts_with("ztat") {
            resources.push(entry.path().to_path_buf());
        }
    }
    resources
}

pub fn load_resources(paths: Vec<String>) {
    use std::time::Instant;
    let now = Instant::now();
    let mut resource_count = 0;

    paths.iter().rev().for_each(|path| {
        let resources = get_ztd_resources(Path::new(path), false);
        resources.iter().for_each(|resource| {
            info!("Loading resource: {}", resource.display());
            let file_name = resource.to_str().unwrap_or_default().to_lowercase();
            if file_name.ends_with(".ztd") {
                match handle_ztd(resource) {
                    Ok(count) => resource_count += count,
                    Err(err) => error!("Error loading ztd: {} -> {}", file_name, err),
                }
            }
        });
    });

    let elapsed = now.elapsed();
    info!(
        "Loaded {} mods and {} ({}) resources in: {:.2?}",
        get_num_mod_ids(),
        get_num_resources(),
        resource_count,
        elapsed
    );

    let now = Instant::now();

    info!("Running BeforeOpenZTMods handlers");
    for handler in get_handlers().iter() {
        if handler.stage() == RunStage::BeforeOpenZTMods {
            get_file_names().into_iter().for_each(|file| {
                handler.handle(&file);
            });
        }
    }

    // TODO: Implement patching
    // apply_patches();

    info!("Running AfterOpenZTMods handlers");
    for handler in get_handlers().iter() {
        if handler.stage() == RunStage::AfterOpenZTMods {
            get_file_names().into_iter().for_each(|file| {
                handler.handle(&file);
            });
        }
    }

    let mut filtered_files = Vec::new();

    get_file_names().into_iter().for_each(|file| {
        let extension = Path::new(&file).extension().unwrap_or_default().to_ascii_lowercase();
        match extension.to_str().unwrap_or_default() {
            "uca" | "ucs" | "ucb" => filtered_files.push(file),
            "cfg" => {
                let inner_filtered = parse_cfg(&file);
                filtered_files.extend(inner_filtered);
            }
            _ => {}
        }
    });

    info!("Loaded {} filtered files", filtered_files.len());

    info!("Running AfterFiltering handlers");
    for handler in get_handlers().iter() {
        if handler.stage() == RunStage::AfterFiltering {
            filtered_files.clone().into_iter().for_each(|file| {
                handler.handle(&file);
            });
        }
    }

    let elapsed = now.elapsed();
    info!("Extra handling took an extra: {:.2?}", elapsed);
}

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

    // Remove old file and add new one
    remove_resource(&patch.target);
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
    let target_path = Path::new(&patch.target);
    let target_ext = target_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let valid_extensions = ["ini", "ai", "cfg", "uca", "ucs", "ucb", "scn", "lyt"];
    if !valid_extensions.contains(&target_ext) {
        anyhow::bail!("Target file '{}' is not an INI file (extension: {}). Merge only works with INI files.",
                     patch.target, target_ext);
    }

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
    let file_type = ZTFileType::try_from(target_path)
        .map_err(|e| anyhow::anyhow!("Invalid target file type: {}", e))?;
    let c_string = std::ffi::CString::new(merged_content.clone())?;
    let ztfile = ZTFile::Text(c_string, file_type, merged_content.len() as u32);

    // Remove old file and add merged one
    remove_resource(&patch.target);
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

fn handle_ztd(resource: &Path) -> anyhow::Result<i32> {
    let mut load_count = 0;
    let mut zip = ZtdArchive::new(resource)?;

    let ztd_type = load_open_zt_mod(&mut zip)?;

    if ztd_type == mods::ZtdType::Openzt {
        return Ok(0);
    }

    let archive = Arc::new(Mutex::new(zip));

    archive.lock().unwrap().file_names().filter(|s| !s.ends_with("/")).for_each(|file_name| {
        add_lazy(file_name.to_string(), archive.clone());
        load_count += 1;
    });

    Ok(load_count)
}

fn parse_cfg(file_name: &String) -> Vec<String> {
    if let Some(legacy_cfg) = get_legacy_cfg_type(file_name) {
        info!("Legacy cfg: {} {:?}", file_name, legacy_cfg.cfg_type);

        let Some((_archive_name, file)) = get_file(file_name) else {
            error!("Error getting file: {}", file_name);
            return Vec::new();
        };
        let mut ini = Ini::new_cs();
        ini.set_comment_symbols(&[';', '#', ':']);
        let Ok(input_string) = str::from_utf8(&file) else {
            error!("Error converting file to string: {}", file_name);
            return Vec::new();
        };
        if let Err(e) = ini.read(input_string.to_string()) {
            error!("Error reading ini {}: {}", file_name, e);
            return Vec::new();
        }

        match legacy_cfg.cfg_type {
            LegacyCfgType::Ambient => parse_simple_cfg(&ini, "ambient"),
            LegacyCfgType::Animal => parse_simple_cfg(&ini, "animals"), //parse_subtypes_cfg(&ini, "animals"),
            LegacyCfgType::Building => parse_simple_cfg(&ini, "building"),
            LegacyCfgType::Fence => parse_simple_cfg(&ini, "fences"),  //parse_subtypes_cfg(&ini, "fences"),
            LegacyCfgType::Filter => parse_simple_cfg(&ini, "filter"), //parse_subtypes_cfg(&ini, "filter"),
            LegacyCfgType::Food => parse_simple_cfg(&ini, "food"),
            LegacyCfgType::Freeform => parse_simple_cfg(&ini, "freeform"),
            // LegacyCfgType::Fringe => Vec::new(),
            LegacyCfgType::Guest => parse_simple_cfg(&ini, "guest"),
            // LegacyCfgType::Help => Vec::new(),
            LegacyCfgType::Item => parse_simple_cfg(&ini, "items"),
            LegacyCfgType::Path => parse_simple_cfg(&ini, "paths"),
            LegacyCfgType::Rubble => parse_simple_cfg(&ini, "other"),
            // LegacyCfgType::Scenario => Vec::new(),
            LegacyCfgType::Scenery => {
                let mut results = parse_simple_cfg(&ini, "objects");
                results.append(&mut parse_simple_cfg(&ini, "foliage"));
                results.append(&mut parse_simple_cfg(&ini, "other"));
                results
            }
            LegacyCfgType::Staff => parse_simple_cfg(&ini, "staff"), //parse_subtypes_cfg(&ini, "staff"),
            LegacyCfgType::Tile => Vec::new(),
            LegacyCfgType::Wall => parse_simple_cfg(&ini, "tankwall"), //parse_subtypes_cfg(&ini, "tankwall"),
            // LegacyCfgType::Expansion => Vec::new(),
            // LegacyCfgType::Show => Vec::new(),
            // LegacyCfgType::Tank => Vec::new(),
            // LegacyCfgType::UIInfoImage => Vec::new(),
            // LegacyCfgType::Economy => Vec::new(),
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn parse_simple_cfg(file: &Ini, section_name: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Some(section) = file.get_map().unwrap_or_default().get(section_name) {
        for (_, value) in section.iter() {
            if let Some(value) = value {
                // If there are multiple values, we take the last one, this occurs only once in vanilla ZT and the values are equal.
                //  Unclear what happens in vanilla ZT if the values are different.
                match value.len().cmp(&1) {
                    std::cmp::Ordering::Equal => results.push(value[0].clone()),
                    std::cmp::Ordering::Greater => results.push(value[value.len() - 1].clone()),
                    _ => {}
                }
            };
        }
    }
    results
}

#[derive(Debug)]
enum LegacyCfgType {
    Ambient,
    Animal,
    Building,
    Fence,
    Filter,
    Food,
    Freeform,
    Fringe,
    Guest,
    Help,
    Item,
    Path,
    Rubble,
    Scenario,
    Scenery,
    Staff,
    Tile,
    Wall,
    Expansion,
    Show,
    Tank,
    UIInfoImage,
    Economy,
}

#[derive(Debug)]
struct LegacyCfg {
    cfg_type: LegacyCfgType,
    file_name: String,
}

fn map_legacy_cfg_type(file_type_str: &str, file_name: String) -> Result<LegacyCfg, String> {
    match file_type_str {
        "ambient" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Ambient,
            file_name,
        }),
        "animal" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Animal,
            file_name,
        }),
        "bldg" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Building,
            file_name,
        }),
        "fences" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Fence,
            file_name,
        }),
        "filter" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Filter,
            file_name,
        }),
        "food" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Food,
            file_name,
        }),
        "free" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Freeform,
            file_name,
        }),
        "fringe" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Fringe,
            file_name,
        }),
        "guests" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Guest,
            file_name,
        }),
        "help" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Help,
            file_name,
        }),
        "items" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Item,
            file_name,
        }),
        "paths" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Path,
            file_name,
        }),
        "rubble" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Rubble,
            file_name,
        }),
        "scenar" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Scenario,
            file_name,
        }),
        "scener" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Scenery,
            file_name,
        }),
        "staff" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Staff,
            file_name,
        }),
        "tile" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Tile,
            file_name,
        }),
        "twall" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Wall,
            file_name,
        }),
        "xpac" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Expansion,
            file_name,
        }),
        "shows" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Show,
            file_name,
        }),
        "tanks" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Tank,
            file_name,
        }),
        "ui/infoimg" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::UIInfoImage,
            file_name,
        }),
        "economy" => Ok(LegacyCfg {
            cfg_type: LegacyCfgType::Economy,
            file_name,
        }),
        _ => Err(format!("Unknown legacy cfg type: {}", file_type_str)),
    }
}

static LEGACY_CFG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^((ambient|animal|bldg|fences|filter|food|free|fringe|guests|help|items|paths|rubble|scenar|scener|staff|tile|twall|xpac)[\w\-. ]*?\.cfg)|((shows|tanks|ui\/infoimg|economy)\.cfg)$")
        .unwrap()
});

fn get_legacy_cfg_type(file_name: &str) -> Option<LegacyCfg> {
    let capture = LEGACY_CFG_REGEX.captures(file_name)?;
    match capture.iter().collect::<Vec<_>>().as_slice() {
        [_, Some(file_name), Some(file_type), None, None] => map_legacy_cfg_type(file_type.as_str(), file_name.as_str().to_string()).ok(),
        [_, None, None, Some(file_name), Some(file_type)] => map_legacy_cfg_type(file_type.as_str(), file_name.as_str().to_string()).ok(),
        _ => None,
    }
}
