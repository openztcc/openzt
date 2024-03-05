use crate::debug_dll::{get_from_memory, get_string_from_memory, save_to_memory};
use crate::add_to_command_register;
use crate::resource_manager::{add_handler, Handler};
use crate::string_registry::add_string_to_registry;

use tracing::info;
use retour_utils::hook_module;

use std::fmt;
use std::fmt::Display;
use std::io::Read;
use std::ffi::CString;

use core::fmt::Error;


use std::path::PathBuf;
use zip::read::ZipFile;

use anyhow::Context;

use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;

use bf_configparser::ini::Ini;

const EXPANSION_LIST_START: u32 = 0x00639030;
const EXPANSION_SIZE: u32 = 0x14;
const EXPANSION_CURRENT: u32 = 0x00638d4c;

const MAX_EXPANSION_SIZE: usize = 14;

// There are no acessors for Expansions, ZT accesses expansions by directly iterating over the array, adding to the array also saves ptrs to ZT's memory keeping things in sync
static EXPANSION_ARRAY: Lazy<Mutex<Vec<Expansion>>> = Lazy::new(|| {
    Mutex::new(Vec::new())
    // Mutex::new(read_expansions_from_memory())
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
    // let mut data_mutex = EXPANSION_ARRAY.lock().unwrap();
    // let array_ptr = data_mutex.as_mut_ptr();
    // let array_end_ptr = unsafe { array_ptr.offset(data_mutex.len() as isize) };
    // let array_buffer_end_ptr = unsafe { array_ptr.offset(data_mutex.capacity() as isize) };
    // info!("Saving expansions to {:#x} to {:#x} cap {:#x} ({} {})", array_ptr as u32, array_end_ptr as u32, array_buffer_end_ptr as u32, data_mutex.len(), data_mutex.capacity());
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
    use crate::ztui::{get_current_buy_tab, get_current_tab, get_selected_sex, BuyTab};
    use crate::ztworldmgr::{read_zt_entity_type_from_memory, ZtEntityTypeClass};


    use super::{get_expansions, save_mutex, add_expansion, Expansion, save_current_expansion, read_current_expansion, add_expansion_with_string_id};

    const RANDOM_SEX_STRING_PTR: u32 = 0x0063e420;

    //uint __cdecl ZTUI::general::entityTypeIsDisplayed(int *param_1,char **param_2,char **param_3)
    #[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
    pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
        // info!("ZTUI::general::entityTypeIsDisplayed {:#x} {:#x} {} {:#x} {}", bf_entity, param_1, get_string_from_memory(get_from_memory(param_1)), param_2, get_string_from_memory(get_from_memory(param_2)));
        // info!("{}", read_zt_entity_type_from_memory(bf_entity));
        let result = unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) };
        // info!("ZTUI::general::entityTypeIsDisplayed this: {:#x}, param_1: {}, param_2: {}, result: {:#x}", bf_entity, get_string_from_memory(get_from_memory(param_1)), get_string_from_memory(get_from_memory(param_2)), result);
        // info!("{}", read_zt_entity_from_memory(bf_entity));
        // info!("{}", get_string_from_memory(get_from_memory::<u32>(bf_entity + 0x98)));
        // zt_sub_type: get_string_from_memory(get_from_memory::<u32>(inner_class_ptr + 0xa4)),
        // name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
        // config_file_ptr: get_from_memory::<u32>(inner_class_ptr + 0x30),

        // TODO: Extract to function, pass in entity, current_expansion and current_buytab, don't even call if not in buy tab
        // Compare answer to ZT's answer and log if different

        let current_expansion = read_current_expansion();

        let entity = read_zt_entity_type_from_memory(bf_entity);

        let current_buy_tab = get_current_buy_tab();
        if get_current_buy_tab().is_none() {
            return 0
        }
        match current_buy_tab.unwrap() {
            BuyTab::AnimalTab => {
                if entity.class() != ZtEntityTypeClass::Animal {
                    return 0
                }
                match get_selected_sex() {
                    Some(sex) => {
                        if sex as String != entity.zt_sub_type() {
                            return 0
                        }
                    }
                    None => {
                        return 0
                    }
                }
            },
            BuyTab::ShelterTab => {   // Following three require parsing the [Member] section in config files
                // Something like `get_members(entity).contains("shelter")`
                // if entity.zt_type() == "shelter" { 
                //     return 1
                // }
            },
            BuyTab::ToysTab => {
                // if entity.zt_type() == "toy" {
                //     return 1
                // }
            },
            BuyTab::ShowToysTab => {
                // if entity.zt_type() == "show_toy" {
                //     return 1
                // }
            },
            BuyTab::BuildingTab => { // Likely needs parsing the [Member] section in config files
                if entity.class() != ZtEntityTypeClass::Building {
                    return 0
                }
            },
            BuyTab::SceneryTab => {
                if entity.class() != ZtEntityTypeClass::Scenery || entity.zt_type() != "other" {
                    return 0
                }
            },
            BuyTab::FenceTab => {
                if !matches!(entity.class(), ZtEntityTypeClass::Fence | ZtEntityTypeClass::TankWall | ZtEntityTypeClass::TankFilter) {
                    return 0
                }
                if entity.zt_type() == "g" {
                    return 0
                }
            },
            BuyTab::PathTab => {
                if entity.class() != ZtEntityTypeClass::Path {
                    return 0
                }
            },
            BuyTab::FoliageTab => { // Likely needs parsing the [Member] section in config files
                // if entity.zt_type() == "foliage" {
                //     return 1
                // }
            },
            BuyTab::RocksTab => { // Likely needs parsing the [Member] section in config files
                // if entity.zt_type() == "rock" {
                //     return 1
                // }
            },
            BuyTab::StaffTab => {
                if !matches!(entity.class(), ZtEntityTypeClass::Keeper | ZtEntityTypeClass::MaintenanceWorker | ZtEntityTypeClass::TourGuide) || (entity.zt_sub_type() != "" && entity.zt_sub_type() != get_string_from_memory(RANDOM_SEX_STRING_PTR)) {
                    return 0
                }
            }
            BuyTab::PaintTerrainTab | BuyTab::TerraformTab => {
                return 0
            },
        }

        // if result == 1 {
        //     info!("Will show {} {} {} {}", get_string_from_memory(param_1), get_string_from_memory(param_2), entity.zt_type(), entity.zt_sub_type());
        // } else if current_expansion.expansion_id == 0x0 && result != 1 && entity.zt_sub_type() == "m" {
        //     info!("All selected but not return 1??????? for {} {} {} {}", get_string_from_memory(param_1), get_string_from_memory(param_2), entity.zt_type(), entity.zt_sub_type());
        // }

        // If all selected return 1
        // if selected in [member]
        // If zoo selected; return 1 if no expansions in member

        // If animal and above is true, return 1 if selected_sex = entity_sex

        result
        // 1
        // if entity.zt_sub_type() == "m" { 1 } else { 0 }
    }

    #[hook(unsafe extern "stdcall" ZTUI_expansionselect_setup, offset=0x001291fb)]
    pub fn ztui_expansionselect_setup() {
        info!("ZTUI::expansionselect::setup");
        unsafe { ZTUI_expansionselect_setup.call() }; //TODO: Remove this call once all functionality has been replicated

        // TODO: Get expansion resource files and parse them into Expansion structs, we can then remove the original call and read from ZT memory
        // expansions should be sorted by expansion_id
        // IDEA: Can we parse files asynchronously before this and just sort the expansions by id and add to memory here?

        // get_expansions();
        // Expansion { expansion_id: 0x0 name_id: 0x5974 name_string: all }

        add_expansion_with_string_id(0x0, "all".to_string(), 0x5974, false);

        let name_all = "all";
        let name_ptr_all = CString::new(name_all).unwrap().into_raw() as u32;
        let name_ptr_all_end = name_ptr_all + name_all.len() as u32 + 1;
        add_expansion(Expansion { expansion_id: 0x0, name_id: 0x5974, name_string_start_ptr: name_ptr_all, name_string_end_ptr: name_ptr_all_end, name_string_buffer_end_ptr: name_ptr_all_end }, false);

        save_current_expansion(0x0);
    }

    // #[hook(unsafe extern "thiscall" BFConfigFile_getKeys, offset=0x00009cf3)]
    // pub fn bf_config_file_get_keys(this: u32, param_1: u32, param_2: u32) -> u32 {
    //     info!("BFConfigFile::getKeys this: {:#x}, param_1: {:#x}, param_2: {:#x}", this, param_1, param_2);
    //     unsafe { BFConfigFile_getKeys.call(this, param_1, param_2) }
    //     // result
    // }

    #[hook(unsafe extern "thiscall" BFUIMgr_getElement, offset=0x0000157d)]
    fn bf_ui_mgr_get_element(this: u32, param_1: u32) -> u32 {
        let result = unsafe { BFUIMgr_getElement.call(this, param_1) };
        if matches!(param_1, 2000 | 2001) {
            info!("BFUIMgr::getElement this: {:#x}, param_1: {:#x}, result: {:#x}", this, param_1, result);
        }
        result
    }


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
    info!("EXPANSION FILE!!!!!!!!!!");
    match parse_expansion_config(file) {
        Ok(_) => info!("Expansion config parsed successfully"),
        Err(e) => info!("Error parsing expansion config: {}", e)
    }
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
    let name = expansion_cfg.get("expansion", "name").context("No name found in expansion config")?;
    let name_ptr = CString::new(name.clone()).unwrap().into_raw() as u32;
    let listid: u32 = expansion_cfg.get_parse("expansion", "listid").map_err(anyhow::Error::msg)?.context("No listid found in expansion config")?;

    info!("Adding expansion: {}", name);
    add_expansion(Expansion { expansion_id: id, name_id: listid, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr + name.len() as u32 + 1, name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1}, false);
    // let expansion = Expansion { expansion_id: id, name_id: listid, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr + name.len() as u32 + 1, name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1};
    // info!("Adding expansion: {}", expansion);

    Ok(())
}

pub fn init() {
    add_to_command_register("list_expansion".to_string(), command_get_expansions);
    add_to_command_register("get_current_expansion".to_string(), command_get_current_expansion);
    add_handler(Handler::new(Some("xpac".to_string()), Some("cfg".to_string()), handle_expansion_config).unwrap());
    unsafe { custom_expansion::init_detours().unwrap() };
}

