use crate::debug_dll::{get_from_memory, get_string_from_memory, get_string_from_memory_bounded, save_to_memory};
use crate::add_to_command_register;
use crate::resource_manager::{add_handler, Handler};
use crate::string_registry::add_string_to_registry;
use crate::ztui::{BuyTab, get_selected_sex, get_random_sex};
use crate::ztworldmgr::{ZTEntity, ZTEntityTypeClass, ZTEntityType};

use tracing::info;
use retour_utils::hook_module;

use std::fmt;
use std::fmt::Display;
use std::io::Read;
use std::ffi::CString;
use std::path::Path;

use core::fmt::Error;


use std::path::PathBuf;
use zip::read::ZipFile;

use anyhow::Context;

use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;

use std::collections::{HashSet, HashMap};

use bf_configparser::ini::Ini;

const EXPANSION_LIST_START: u32 = 0x00639030;
const EXPANSION_SIZE: u32 = 0x14;
const EXPANSION_CURRENT: u32 = 0x00638d4c;

const MAX_EXPANSION_SIZE: usize = 14;

static MEMBER_SETS: Lazy<Mutex<HashMap<String, HashSet<String>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

fn add_member(entity_name: String, member: String) {
    let mut data_mutex = MEMBER_SETS.lock().unwrap();
    let mut set = data_mutex.entry(member).or_insert(HashSet::new());
    set.insert(entity_name);
}

pub fn is_member(entity_name: &str, member: &str) -> bool {
    let data_mutex = MEMBER_SETS.lock().unwrap();
    match data_mutex.get(member) {
        Some(set) => set.contains(entity_name),
        None => false
    }
}

pub fn get_members(member: &str) -> Option<HashSet<String>> {
    let data_mutex = MEMBER_SETS.lock().unwrap();
    data_mutex.get(member).cloned()
}

fn command_get_members(args: Vec<&str>) -> Result<String, &'static str> {
    let data_mutex = MEMBER_SETS.lock().unwrap();
    let mut result = String::new();

    for (set_name, members) in data_mutex.iter() {
        let members_as_string: Vec<String> = members.iter().cloned().collect();
        result.push_str(&format!("Set: {} -> Members: {}\n", set_name, members_as_string.join(", ")));
    }

    Ok(result)
}

// There are no accessors for Expansions, ZT accesses expansions by directly iterating over the array, adding to the array also saves ptrs to ZT's memory keeping things in sync
static EXPANSION_ARRAY: Lazy<Mutex<Vec<Expansion>>> = Lazy::new(|| {
    Mutex::new(Vec::new())
});

fn add_expansion(expansion: Expansion, save_to_memory: bool) -> Result<(), &'static str> {
    let mut data_mutex = EXPANSION_ARRAY.lock().unwrap();
    if data_mutex.len() >= MAX_EXPANSION_SIZE {
        return Err("Max expansion size reached");
    }
    data_mutex.push(expansion);

    data_mutex.sort_by_key(|k| k.expansion_id);

    if save_to_memory {
        inner_save_mutex(data_mutex);
    }

    Ok(())
}

fn get_expansion(expansion_id: u32) -> Option<Expansion> {
    let data_mutex = EXPANSION_ARRAY.lock().unwrap();
    data_mutex.iter().find(|expansion| expansion.expansion_id == expansion_id).cloned()
}

fn save_mutex() {
    inner_save_mutex(EXPANSION_ARRAY.lock().unwrap());
}

fn inner_save_mutex(mut mutex_guard: MutexGuard<Vec<Expansion>>) {
    let array_ptr = mutex_guard.as_mut_ptr();
    let array_end_ptr = unsafe { array_ptr.offset(mutex_guard.len() as isize) };
    let array_buffer_end_ptr = unsafe { array_ptr.offset(mutex_guard.capacity() as isize) };
    info!("Saving expansions to {:#x} to {:#x}; {:#x}", array_ptr as u32, array_end_ptr as u32, array_buffer_end_ptr as u32);

    save_expansion_list_to_memory(ExpansionList { array_start: array_ptr as u32, array_end: array_end_ptr as u32, buffer_end: array_end_ptr as u32 });

}

fn get_expansions() -> Vec<Expansion> {
    EXPANSION_ARRAY.lock().unwrap().clone()
}

#[derive(Debug)]
#[repr(C)]
struct ExpansionList {
    array_start: u32,
    array_end: u32,
    buffer_end: u32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Expansion {
    expansion_id: u32,
    name_id: u32,
    name_string_start_ptr: u32,
    name_string_end_ptr: u32,
    name_string_buffer_end_ptr: u32,
}

impl Expansion {
    fn name_string(&self) -> String {
        get_string_from_memory_bounded(self.name_string_start_ptr, self.name_string_end_ptr, self.name_string_buffer_end_ptr)
    }
}

fn read_expansion_list_from_memory() -> ExpansionList {
    get_from_memory(EXPANSION_LIST_START)
}

fn read_expansion_from_memory(address: u32) -> Expansion {
    get_from_memory(address)
}

fn read_expansions_from_memory() -> Vec<Expansion> {
    let expansion_list = read_expansion_list_from_memory();
    info!("Reading expansions from {:#x} to {:#x}, len {}", expansion_list.array_start, expansion_list.array_end, (expansion_list.array_end - expansion_list.array_start) / EXPANSION_SIZE);
    let mut expansions = Vec::new();
    let mut current_expansion_address = expansion_list.array_start;
    while current_expansion_address < expansion_list.array_end {
        expansions.push(read_expansion_from_memory(current_expansion_address));
        current_expansion_address += EXPANSION_SIZE;
    }
    expansions

}

fn read_current_expansion() -> Expansion {
    let current_expansion_id = get_from_memory(EXPANSION_CURRENT);
    get_expansion(current_expansion_id).unwrap()
}

fn save_current_expansion(expansion_id: u32) {
    save_to_memory(EXPANSION_CURRENT, expansion_id);
}

fn save_expansion_list_to_memory(expansion_list: ExpansionList) {
    save_to_memory(EXPANSION_LIST_START, expansion_list);
}

impl Display for Expansion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Expansion {{ expansion_id: {:#x} name_id: {:#x} name_string: {} }}", self.expansion_id, self.name_id, get_string_from_memory(self.name_string_start_ptr))
    }
}

impl Display for ExpansionList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExpansionList {{ array_start: {:#x} array_end: {:#x} buffer_end: {:#x} }}", self.array_start, self.array_end, self.buffer_end)
    }
}

fn command_get_expansions(_args: Vec<&str>) -> Result<String, &'static str> {
    let mut string_array = Vec::new();
    for expansion in read_expansions_from_memory() {
        string_array.push(expansion.to_string());
    }

    Ok(string_array.join("\n"))
}

fn command_get_current_expansion(_args: Vec<&str>) -> Result<String, &'static str> {
    Ok(read_current_expansion().to_string())
}

#[hook_module("zoo.exe")]
pub mod custom_expansion {
    use tracing::info;

    use std::ffi::CString;

    use crate::debug_dll::{get_from_memory, get_string_from_memory, save_to_memory};

    use crate::string_registry::add_string_to_registry;
    use crate::ztui::{get_current_buy_tab, get_selected_sex, BuyTab};
    use crate::ztworldmgr::{read_zt_entity_type_from_memory, ZTEntityTypeClass}; 


    use super::{get_expansions, save_mutex, add_expansion, Expansion, save_current_expansion, read_current_expansion, add_expansion_with_string_id, add_expansion_with_string_value, get_members};

    #[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
    pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
        let result = unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) };

        let current_expansion = read_current_expansion();

        let entity = read_zt_entity_type_from_memory(bf_entity);

        let current_buy_tab = get_current_buy_tab();
        if get_current_buy_tab().is_none() {
            return 0
        }

        let reimplemented_result = super::filter_entity_type(&current_buy_tab.unwrap(), &current_expansion, &entity);

        if reimplemented_result {1} else {0}
    }

    #[hook(unsafe extern "stdcall" ZTUI_expansionselect_setup, offset=0x001291fb)]
    pub fn ztui_expansionselect_setup() {
        info!("ZTUI::expansionselect::setup");
        unsafe { ZTUI_expansionselect_setup.call() }; //TODO: Remove this call once all functionality has been replicated

        add_expansion_with_string_id(0x0, "all".to_string(), 0x5974, false);

        if let Some(member_hash) = get_members("cc") && member_hash.len() > 0 {
            add_expansion_with_string_value(0x4000, "cc".to_string(), "Custom Content".to_string(), true);
        }

        save_current_expansion(0x0);
    }

    #[hook(unsafe extern "thiscall" UIImage_load, offset=0x000d3509)]
    pub fn ui_image_load(this: u32, bfconfigfile: u32, header: u32) -> u32 {

        let result = unsafe { UIImage_load.call(this, bfconfigfile, header) };

        info!("UIImage_load(0x4d3509) {:#x} {:#x} {} -> {:#x}", this, bfconfigfile, get_string_from_memory(header), result);

        result
    }

    // #[hook(unsafe extern "thiscall" BFAnimCache_findAnim, offset=0x00001fdd)]
    // pub fn bf_anim_cache_find_anim(this: u32, anim_name: u32, param_bool: u8) -> u32 {
    //     let result = unsafe { BFAnimCache_findAnim.call(this, anim_name, param_bool) };

    //     info!("BFAnimCache_findAnim(0x41fdd) {:#x} {} {:#x} -> {:#x}", this, get_string_from_memory(anim_name), param_bool, result);

    //     result
    // }

    // #[hook(unsafe extern "thiscall" UIControl_setAnimation, offset=0x000b1aa0)]
    // pub fn ui_control_set_animation(this: u32, anim_name: u32, param_bool: u8) {
    //     unsafe { UIControl_setAnimation.call(this, anim_name, param_bool) };

    //     info!("UIControl_setAnimation(0x4b1aa0) {:#x} {} {:#x}", this, get_string_from_memory(anim_name), param_bool);
    // }

    

}

fn filter_entity_type(buy_tab: &BuyTab, current_expansion: &Expansion, entity: &ZTEntityType) -> bool {
    match buy_tab {
        BuyTab::AnimalTab => {
            if !entity.is_member("animals".to_string()) {
                return false
            }
            match get_selected_sex() {
                Some(sex) => {
                    if &sex.to_string() != entity.zt_sub_type() {
                        return false
                    }
                }
                None => {
                    return false
                }
            }
        },
        BuyTab::ShelterTab => {
            if !entity.is_member("shelters".to_string()) {
                return false
            }
        },
        BuyTab::ToysTab => {
            if !entity.is_member("toys".to_string()) {
                return false
            }
        },
        BuyTab::ShowToysTab => {
            if !entity.is_member("showtoys".to_string()) {
                return false
            }
        },
        BuyTab::BuildingTab => {
            if !entity.is_member("structures".to_string()) {
                return false
            }
        },
        BuyTab::SceneryTab => {
            if !entity.is_member("scenery".to_string()) {
                return false
            }
            // TODO: Make member name a combination of name and class so name double-ups don't cause this issue
            if entity.class() == &ZTEntityTypeClass::Scenery && entity.zt_type() == "other" && entity.zt_sub_type() == "fountain" {
                return false
            }
        },
        BuyTab::FenceTab => {
            if !entity.is_member("fence".to_string()) {
                return false
            }
            if entity.zt_sub_type() == "g" {
                return false
            }
        },
        BuyTab::PathTab => {
            if !entity.is_member("paths".to_string()){
                return false
            }
        },
        BuyTab::FoliageTab => {
            if !entity.is_member("foliage".to_string()){
                return false
            }
        },
        BuyTab::RocksTab => {
            if !entity.is_member("rocks".to_string()){
                return false
            }
        },
        BuyTab::StaffTab => {
            if !entity.is_member("staff".to_string()) {
                return false
            }
            if (matches!(entity.zt_sub_type().as_str(), "m" | "f") && entity.zt_sub_type() != &get_random_sex().unwrap().to_string()) {
                return false
            }
        }
        BuyTab::DeveloperTab => {
            if !entity.is_member("developer".to_string()) {
                return false
            }
        }
        BuyTab::PaintTerrainTab | BuyTab::TerraformTab => {
            return false
        },
    }

    if buy_tab != &BuyTab::PathTab {
        if current_expansion.expansion_id == 0x1 {
            for expansion in get_expansions() {
                if expansion.expansion_id > 0x1 && entity.is_member(expansion.name_string()) && !entity.is_member("zoo".to_string()) {
                    return false
                }
            }
        }
        if current_expansion.expansion_id > 0x1 && !entity.is_member(current_expansion.name_string()) {
                return false
        }
    }

    return true
}

fn add_expansion_with_string_id(id: u32, name: String, string_id: u32, save_to_memory: bool) {
    let name_ptr = CString::new(name.clone()).unwrap().into_raw() as u32;
    let name_ptr_end = name_ptr + name.len() as u32 + 1;
    add_expansion(Expansion { expansion_id: id, name_id: string_id, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr_end, name_string_buffer_end_ptr: name_ptr_end }, save_to_memory);
}

fn add_expansion_with_string_value(id: u32, name: String, string_value: String, save_to_memory: bool) {
    let name_ptr = CString::new(name.clone()).unwrap().into_raw() as u32;
    let name_ptr_end = name_ptr + name.len() as u32 + 1;
    let name_id = add_string_to_registry(string_value);
    add_expansion(Expansion { expansion_id: 0x3fff, name_id: name_id, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr + name.len() as u32 + 1, name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1}, save_to_memory);
}


fn handle_expansion_config(path: &PathBuf, file: &mut ZipFile) {
    if let Err(e) = parse_expansion_config(file) {
        info!("Error parsing expansion config: {} {}", file.name(), e)
    }
}

fn handle_member_parsing(path: &PathBuf, file: &mut ZipFile) {
    if let Err(e) = parse_member_config(file) {
        info!("Error parsing member config: {} {}", file.name(), e)
    }
}

static FILE_NAME_OVERRIDES: Lazy<HashMap<String, String>> = Lazy::new(|| {
    vec![
        ("fences/tankwall.ai".to_string(), "fences/tankwal1.ai".to_string()), // Assumed spelling mistake
        ("fences/hedge.ai".to_string(), "fences/not_hedge.ai".to_string()), // Duplicates, this one isn't loaded
        // TODO: Below might not be needed?
        ("scenery/other/fountain.ai".to_string(), "scenery/other/other_fountain.ai".to_string()), // Duplicates, this one isn't loaded
    ].into_iter().collect()
});

fn parse_member_config(file: &mut ZipFile) -> anyhow::Result<()> {
    let mut buffer = vec![0; file.size() as usize];
    if let Err(error) = file.read(&mut buffer[..]) {
        info!("Error reading member config {}: {}", file.name(), error);
        return Ok(());
    }
    let string_buffer = String::from_utf8_lossy(&buffer[..]).to_string(); //TODO: Investigate parsing ANSI files

    let mut member_cfg = Ini::new();
    member_cfg.set_comment_symbols(&[';', '#', ':']);
    member_cfg.read(string_buffer).map_err(anyhow::Error::msg)?;

    let filepath = match FILE_NAME_OVERRIDES.contains_key(file.name()) {
        true => FILE_NAME_OVERRIDES.get(file.name()).unwrap().to_string(),
        false => file.name().to_ascii_lowercase(),
    };

    let filename = Path::new(&filepath).file_stem().unwrap().to_str().unwrap().to_string();
    let extension = Path::new(&filepath).extension().unwrap().to_str().unwrap().to_string();

    if let Some(keys) = member_cfg.get_keys("member") {
        for key in keys {
            add_member(filename.clone(), key);
        }
    }

    if matches!(extension.as_str(), "uca" | "ucb" | "ucs") && filename != "b101b026" {
        add_member(filename, "cc".to_string());
    }

    Ok(())
}


fn parse_expansion_config(file: &mut ZipFile) -> anyhow::Result<()> {
    let mut string_buffer = String::with_capacity(file.size() as usize);
    file.read_to_string(&mut string_buffer)?;
    info!("{}", string_buffer);

    let mut expansion_cfg = Ini::new();
    expansion_cfg.read(string_buffer).map_err(anyhow::Error::msg)?;

    // TODO: bf-configparser should return a custom error so we can use ? rather than map_err
    let mut id: u32= expansion_cfg.get_parse("expansion", "id").map_err(anyhow::Error::msg)?.context("No id found in expansion config")?;
    id += 1;
    let name = expansion_cfg.get("expansion", "name").context("No name found in expansion config")?.to_ascii_lowercase();
    let name_ptr = CString::new(name.clone()).unwrap().into_raw() as u32;
    let listid: u32 = expansion_cfg.get_parse("expansion", "listid").map_err(anyhow::Error::msg)?.context("No listid found in expansion config")?;

    info!("Adding expansion: {}", name);
    add_expansion(Expansion { expansion_id: id, name_id: listid, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr + name.len() as u32 + 1, name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1}, false);

    Ok(())
}

fn handle_expansion_dropdown(file: &mut ZipFile) {
    
}

pub fn init() {
    add_to_command_register("list_expansion".to_string(), command_get_expansions);
    add_to_command_register("get_current_expansion".to_string(), command_get_current_expansion);
    add_to_command_register("get_members".to_string(), command_get_members);
    add_handler(Handler::new(Some("xpac".to_string()), Some("cfg".to_string()), handle_expansion_config).unwrap());
    add_handler(Handler::new(None, Some("uca".to_string()), handle_member_parsing).unwrap());
    add_handler(Handler::new(None, Some("ucs".to_string()), handle_member_parsing).unwrap());
    add_handler(Handler::new(None, Some("ucb".to_string()), handle_member_parsing).unwrap());
    add_handler(Handler::new(None, Some("ai".to_string()), handle_member_parsing).unwrap());
    unsafe { custom_expansion::init_detours().unwrap() };
}

