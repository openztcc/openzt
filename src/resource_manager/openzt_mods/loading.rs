use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fmt,
    path::Path,
    str,
    sync::Mutex,
};

use anyhow::{anyhow, Context};
use bf_configparser::ini::{Ini, WriteOptions};
use once_cell::sync::Lazy;
use tracing::info;

use crate::{
    animation::Animation,
    mods,
    resource_manager::{
        lazyresourcemap::add_ztfile,
        openzt_mods::habitats_locations::add_location_or_habitat,
        ztd::ZtdArchive,
        ztfile::{ZTFile, ZTFileType},
    },
};

/// Used to ensure mod_ids don't clash, a mod will not load if an id is already in this map
static MOD_ID_SET: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Tries to add a new mod id to the set, returns false if the mod_id already exists
pub fn add_new_mod_id(mod_id: &str) -> bool {
    let mut binding = MOD_ID_SET.lock().unwrap();
    binding.insert(mod_id.to_string())
}

pub fn get_num_mod_ids() -> usize {
    let binding = MOD_ID_SET.lock().unwrap();
    binding.len()
}

pub fn get_mod_ids() -> Vec<String> {
    let binding = MOD_ID_SET.lock().unwrap();
    binding.iter().cloned().collect()
}

pub fn openzt_base_resource_id(mod_id: &String, resource_type: ResourceType, resource_name: &String) -> String {
    let resource_type_name = resource_type.to_string();
    format!("openzt.mods.{}.{}.{}", mod_id, resource_type_name, resource_name)
}

pub fn openzt_full_resource_id_path(base_resource_id: &String, file_type: ZTFileType) -> String {
    format!("{}.{}", base_resource_id, file_type)
}

pub fn load_open_zt_mod(archive: &mut ZtdArchive) -> anyhow::Result<mods::ZtdType> {
    let archive_name = archive.name().to_string();
    let Ok(meta_file) = archive.by_name("meta.toml") else {
        return Ok(mods::ZtdType::Legacy);
    };

    let meta = toml::from_str::<mods::Meta>(&String::try_from(meta_file).with_context(|| format!("error reading meta.toml from {}", &archive_name))?)
        .with_context(|| "Failed to parse meta.toml")?;

    if meta.ztd_type() == &mods::ZtdType::Legacy {
        return Ok(mods::ZtdType::Legacy);
    }

    let mod_id = meta.mod_id().to_string();

    if !add_new_mod_id(&mod_id) {
        return Err(anyhow!("Mod already loaded: {}", mod_id));
    }

    info!("Loading OpenZT mod: {} {}", meta.name(), meta.mod_id());

    let mut file_map: HashMap<String, Box<[u8]>> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            // TODO: Create type that wraps ZipArchive and provide archive name for better error reporting
            .with_context(|| format!("Error reading zip file at index {} from file {}", i, archive_name))?;

        if file.is_dir() {
            continue;
        }
        let file_name = file.name().to_string();

        let mut file_buffer = vec![0; file.size() as usize].into_boxed_slice();
        file.read_exact(&mut file_buffer).with_context(|| format!("Error reading file: {}", file_name))?;

        file_map.insert(file_name, file_buffer);
    }

    let keys = file_map.keys().clone();

    for file_name in keys {
        if file_name.starts_with("defs/") {
            load_def(&mod_id, file_name, &file_map)?;
        }
    }

    Ok(meta.ztd_type().clone())
}

pub enum ResourceType {
    Location,
    Habitat,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResourceType::Location => write!(f, "location"),
            ResourceType::Habitat => write!(f, "habitat"),
        }
    }
}

pub fn load_def(mod_id: &String, file_name: &String, file_map: &HashMap<String, Box<[u8]>>) -> anyhow::Result<mods::ModDefinition> {
    info!("Loading defs {} from {}", file_name, mod_id);

    let file = file_map
        .get(file_name)
        .with_context(|| format!("Error finding file {} in resource map for mod {}", file_name, mod_id))?;

    let intermediate_string = str::from_utf8(file)
        .with_context(|| format!("Error converting file {} to utf8 for mod {}", file_name, mod_id))?
        .to_string();

    let defs = toml::from_str::<mods::ModDefinition>(&intermediate_string).with_context(|| format!("Error parsing defs from OpenZT mod: {}", file_name))?;

    info!("Loading defs: {}", defs.len());

    // Habitats
    if let Some(habitats) = defs.habitats() {
        for (habitat_name, habitat_def) in habitats.iter() {
            let base_resource_id = openzt_base_resource_id(mod_id, ResourceType::Habitat, habitat_name);
            load_icon_definition(
                &base_resource_id,
                habitat_def,
                file_map,
                mod_id,
                include_str!("../../../resources/include/infoimg-habitat.ani").to_string(),
            )?;
            add_location_or_habitat(habitat_def.name(), &base_resource_id)?;
        }
    }

    // Locations
    if let Some(locations) = defs.locations() {
        for (location_name, location_def) in locations.iter() {
            let base_resource_id = openzt_base_resource_id(mod_id, ResourceType::Location, location_name);
            load_icon_definition(
                &base_resource_id,
                location_def,
                file_map,
                mod_id,
                include_str!("../../../resources/include/infoimg-location.ani").to_string(),
            )?;
            add_location_or_habitat(location_def.name(), &base_resource_id)?;
        }
    }
    Ok(defs)
}

fn load_icon_definition(
    base_resource_id: &String,
    icon_definition: &mods::IconDefinition,
    file_map: &HashMap<String, Box<[u8]>>,
    mod_id: &String,
    base_config: String,
) -> anyhow::Result<()> {
    let icon_file = file_map.get(icon_definition.icon_path()).with_context(|| {
        format!(
            "Error loading openzt mod {}, cannot find file {} for icon_def {}",
            mod_id,
            icon_definition.icon_path(),
            icon_definition.name()
        )
    })?;

    let icon_file_palette = file_map.get(icon_definition.icon_palette_path()).with_context(|| {
        format!(
            "Error loading openzt mod {}, cannot find file {} for icon_def {}",
            mod_id,
            icon_definition.icon_palette_path(),
            icon_definition.name()
        )
    })?;

    let palette_file_name = openzt_full_resource_id_path(base_resource_id, ZTFileType::Palette);
    let palette_ztfile = ZTFile::builder()
        .file_name(palette_file_name.clone())
        .file_size(icon_file_palette.len() as u32)
        .type_(ZTFileType::Palette)
        .raw_data(icon_file_palette.clone())
        .build();
    add_ztfile(Path::new("zip::./openzt.ztd"), palette_file_name.clone(), palette_ztfile);

    let mut animation = Animation::parse(icon_file)?;
    animation.set_palette_filename(palette_file_name.clone());
    let (new_animation_bytes, icon_size) = animation.write()?;

    let new_icon_file = new_animation_bytes.into_boxed_slice();

    let mut ani_cfg = Ini::new_cs();
    ani_cfg.set_comment_symbols(&[';', '#', ':']);
    ani_cfg.read(base_config).map_err(|s| anyhow!("Error reading ini: {}", s))?;

    if ani_cfg
        .set("animation", "dir1", Some(openzt_full_resource_id_path(base_resource_id, ZTFileType::Animation)))
        .is_none()
    {
        return Err(anyhow!("Error setting dir1 for ani"));
    }

    let mut write_options = WriteOptions::default();
    write_options.space_around_delimiters = true;
    write_options.blank_lines_between_sections = 1;
    let new_string = ani_cfg.pretty_writes(&write_options);
    info!("New ani: \n{}", new_string);
    let file_size = new_string.len() as u32;
    let file_name = openzt_full_resource_id_path(base_resource_id, ZTFileType::Ani);

    let Ok(new_c_string) = CString::new(new_string) else {
        return Err(anyhow!(
            "Error loading openzt mod {} when converting .ani to CString after modifying {}",
            mod_id,
            file_name
        ));
    };

    let ztfile = ZTFile::builder()
        .file_name(file_name.clone())
        .file_size(file_size)
        .type_(ZTFileType::Ani)
        .cstring_data(new_c_string)
        .build();

    add_ztfile(Path::new("zip::./openzt.ztd"), file_name, ztfile);

    let animation_file_name = openzt_full_resource_id_path(base_resource_id, ZTFileType::Animation);
    let animation_ztfile = ZTFile::builder()
        .file_name(animation_file_name.clone())
        .file_size(icon_size as u32)
        .type_(ZTFileType::Animation)
        .raw_data(new_icon_file)
        .build();

    add_ztfile(Path::new("zip::./openzt.ztd"), animation_file_name.clone(), animation_ztfile);

    let palette_file_name = openzt_full_resource_id_path(base_resource_id, ZTFileType::Palette);
    let palette_ztfile = ZTFile::builder()
        .file_name(palette_file_name.clone())
        .file_size(icon_file_palette.len() as u32)
        .type_(ZTFileType::Palette)
        .raw_data(icon_file_palette.clone())
        .build();
    add_ztfile(Path::new("zip::./openzt.ztd"), palette_file_name, palette_ztfile);

    Ok(())
}
