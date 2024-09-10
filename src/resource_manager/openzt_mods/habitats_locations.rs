use anyhow::anyhow;
use crate::string_registry::add_string_to_registry;
use tracing::info;

use std::ffi::CString;

use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;

use bf_configparser::ini::Ini;
use bf_configparser::ini::WriteOptions;

use crate::animation::Animation;

use std::path::Path;

use crate::mods;

use anyhow::Context;

use crate::resource_manager::ztfile::{ZTFile, ZTFileType};
use crate::resource_manager::openzt_mods::loading::openzt_full_resource_id_path;

// Map between the id ZT uses to reference locations/habitats and the string ptr of the animation (icon) resource
pub static LOCATIONS_HABITATS_RESOURCE_MAP: Lazy<Mutex<HashMap<u32, u32>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Map between the animation (icon resource) and the id ZT uses to reference location/habitats, this is used to lookup the id needed to add the habitat/location to an animal
pub static LOCATIONS_HABITATS_ID_MAP: Lazy<Mutex<HashMap<String, u32>>> = Lazy::new(|| Mutex::new(HashMap::new()));


pub fn add_location_or_habitat(name: &String, icon_resource_id: &String) -> anyhow::Result<()> {
    let mut resource_binding = LOCATIONS_HABITATS_RESOURCE_MAP.lock().unwrap();

    let mut id_binding = LOCATIONS_HABITATS_ID_MAP.lock().unwrap();

    let string_id = add_string_to_registry(name.clone());

    info!("Adding location/habitat: {} {} -> {}", name, icon_resource_id, string_id);

    let icon_resource_id_cstring = CString::new(icon_resource_id.clone())
        .with_context(|| format!("Failed to create cstring for location/habitat {} with icon_resource_id {}", name, icon_resource_id))?;
    resource_binding.insert(string_id, icon_resource_id_cstring.into_raw() as u32);
    id_binding.insert(name.clone(), string_id);

    Ok(())
}

pub fn get_location_or_habitat_by_id(id: u32) -> Option<u32> {
    let binding = LOCATIONS_HABITATS_RESOURCE_MAP.lock().unwrap();
    binding.get(&id).cloned()
}

pub fn get_location_or_habitat_by_name(name: &String) -> Option<u32> {
    let binding = LOCATIONS_HABITATS_ID_MAP.lock().unwrap();
    binding.get(name).cloned()
}

pub fn get_location_habitat_ids() -> Vec<u32> {
    let binding = LOCATIONS_HABITATS_RESOURCE_MAP.lock().unwrap();
    binding.keys().cloned().collect()
}
