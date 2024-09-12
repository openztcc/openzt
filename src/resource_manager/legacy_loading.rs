use std::{
    io,
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex},
};

use bf_configparser::ini::Ini;
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::{error, info};
use walkdir::WalkDir;
use zip::read::ZipFile;

use super::ztd::ZtdArchive;
use crate::{
    mods,
    resource_manager::{
        handlers::{get_handlers, RunStage},
        lazyresourcemap::{add_lazy, get_file, get_file_names, get_num_resources},
        openzt_mods::{get_num_mod_ids, load_open_zt_mod},
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
            LegacyCfgType::Free => parse_simple_cfg(&ini, "freeform"),
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
                if value.len() == 1 {
                    results.push(value[0].clone());
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
    Free,
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
            cfg_type: LegacyCfgType::Free,
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

static LEGACY_CFG_REGEX: Lazy<Regex> = Lazy::new(|| {
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
