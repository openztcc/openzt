use crate::debug_dll::{get_from_memory, get_string_from_memory, save_to_memory};
use crate::add_to_command_register;
use crate::resource_manager::{add_handler, Handler};

use tracing::info;
use retour_utils::hook_module;

use std::fmt;
use std::fmt::Display;
use std::io::Read;

use std::path::PathBuf;
use zip::read::ZipFile;

use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;

const EXPANSION_LIST_START: u32 = 0x00639030;
const EXPANSION_SIZE: u32 = 0x14;
const EXPANSION_CURRENT: u32 = 0x00638d4c;

const MAX_EXPANSION_SIZE: usize = 14;

// There are no acessors for Expansions, ZT accesses expansions by directly iterating over the array, adding to the array also saves ptrs to ZT's memory keeping things in sync
static EXPANSION_ARRAY: Lazy<Mutex<Vec<Expansion>>> = Lazy::new(|| {
    Mutex::new(read_expansions_from_memory())
});

fn add_expansion(expansion: Expansion) -> Result<(), &'static str> {
    let mut data_mutex = EXPANSION_ARRAY.lock().unwrap();
    if data_mutex.len() >= MAX_EXPANSION_SIZE {
        return Err("Max expansion size reached");
    }
    data_mutex.push(expansion);

    inner_save_mutex(data_mutex);

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
    let expansions = read_expansions_from_memory();
    expansions.into_iter().find(|expansion| expansion.expansion_id == current_expansion_id).unwrap()
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
    use crate::ztworldmgr::read_zt_entity_type_from_memory;


    use super::{get_expansions, save_mutex, add_expansion, Expansion, save_current_expansion};

    //uint __cdecl ZTUI::general::entityTypeIsDisplayed(int *param_1,char **param_2,char **param_3)
    #[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
    pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
        info!("ZTUI::general::entityTypeIsDisplayed {} {} {}", bf_entity, get_string_from_memory(get_from_memory(param_1)), get_string_from_memory(get_from_memory(param_2)));
        info!("{}", read_zt_entity_type_from_memory(bf_entity));
        let result = unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) };
        // info!("ZTUI::general::entityTypeIsDisplayed this: {:#x}, param_1: {}, param_2: {}, result: {:#x}", bf_entity, get_string_from_memory(get_from_memory(param_1)), get_string_from_memory(get_from_memory(param_2)), result);
        // info!("{}", read_zt_entity_from_memory(bf_entity));
        // info!("{}", get_string_from_memory(get_from_memory::<u32>(bf_entity + 0x98)));
        // zt_sub_type: get_string_from_memory(get_from_memory::<u32>(inner_class_ptr + 0xa4)),
        // name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
        // config_file_ptr: get_from_memory::<u32>(inner_class_ptr + 0x30),
        result
    }

    #[hook(unsafe extern "stdcall" ZTUI_expansionselect_setup, offset=0x001291fb)]
    pub fn ztui_expansionselect_setup() {
        info!("ZTUI::expansionselect::setup");
        unsafe { ZTUI_expansionselect_setup.call() }; //TODO: Remove this call once all functionality has been replicated

        // TODO: Get expansion resource files and parse them into Expansion structs, we can then remove the original call and read from ZT memory
        // expansions should be sorted by expansion_id
        // IDEA: Can we parse files asynchronously before this and just sort the expansions by id and add to memory here?

        get_expansions();

        let name = "TEST";

        // let name = CString::new("TEST").unwrap();
        let name_ptr = CString::new(name).unwrap().into_raw() as u32;

        // TODO: Add display name to string registry and get ID back
        
        let name_id = add_string_to_registry("TEST EXPANSION".to_string());

        add_expansion(Expansion { expansion_id: 0x101, name_id: name_id, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr + name.len() as u32 + 1, name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1});

        // save_mutex();

        save_current_expansion(0x0);
    }

    // #[hook(unsafe extern "thiscall" BFConfigFile_getKeys, offset=0x00009cf3)]
    // pub fn bf_config_file_get_keys(this: u32, param_1: u32, param_2: u32) -> u32 {
    //     info!("BFConfigFile::getKeys this: {:#x}, param_1: {:#x}, param_2: {:#x}", this, param_1, param_2);
    //     unsafe { BFConfigFile_getKeys.call(this, param_1, param_2) }
    //     // result
    // }
}

fn parse_expansion(path: &PathBuf, file: &mut ZipFile) {
    info!("EXPANSION FILE!!!!!!!!!!");
                //     println!("Filename: {}", file.name());
            //     // std::io::copy(&mut file, &mut std::io::stdout());
    let mut string_buffer = String::with_capacity(file.size() as usize);
    file.read_to_string(&mut string_buffer).unwrap();
    info!("{}", string_buffer);

}

pub fn init() {
    add_to_command_register("list_expansion".to_string(), command_get_expansions);
    add_to_command_register("get_current_expansion".to_string(), command_get_current_expansion);
    add_handler(Handler::new(Some("xpac".to_string()), Some("cfg".to_string()), parse_expansion).unwrap());
    unsafe { custom_expansion::init_detours().unwrap() };
}

