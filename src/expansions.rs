use crate::debug_dll::{get_from_memory, get_string_from_memory, save_to_memory};
use crate::add_to_command_register;

use tracing::info;
use retour_utils::hook_module;

use std::fmt;
use std::fmt::Display;

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

    // use super::get_string_ptr;

    use super::{get_expansions, save_mutex, add_expansion, Expansion};

    // #[hook(unsafe extern "thiscall" BFConfigFile_getKeys, offset=0x00009cf3)]
    // pub fn bf_config_file_get_keys(this: u32, param_1: u32, param_2: u32) -> u32 {
    //     let result = unsafe { BFConfigFile_getKeys.call(this, param_1, param_2) };
    //     let string_array: u32 = get_from_memory(result);
    //     let string_array_end: u32 = get_from_memory(result + 0x4);
    //     let string_array_buffer_end: u32 = get_from_memory(result + 0x8);
    //     let header: String = get_string_from_memory(param_2);
    //     if header == "Member" && string_array != 0 {
    //         info!("BFConfigFile::getKeys this: {:#x}, param_1: {:#x}, param_2: {}, result: {:#x} -> {:#x}", this, param_1, header, result, string_array);
    //         save_to_memory(result + 0x4, string_array_end + 0x4);
    //         save_to_memory(result + 0x8, string_array_buffer_end + 0x4);
    //         save_to_memory(string_array_end, get_string_ptr("RSN".to_string()).unwrap());
    //         // read_string_list_from_memory(string_array, get_from_memory(result + 0x4));
    //     }
    //     result
    // }

    //uint __cdecl ZTUI::general::entityTypeIsDisplayed(int *param_1,char **param_2,char **param_3)
    #[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
    pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
        let result = unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) };
        info!("ZTUI::general::entityTypeIsDisplayed this: {:#x}, param_1: {}, param_2: {}, result: {:#x}", bf_entity, get_string_from_memory(get_from_memory(param_1)), get_string_from_memory(get_from_memory(param_2)), result);
        result
    }

    pub fn read_string_list_from_memory(start_ptr: u32, end_ptr: u32) {
        let mut current_ptr = start_ptr;
        while current_ptr < end_ptr {
            let string = get_string_from_memory(get_from_memory(current_ptr));
            info!("String: {:#x} {}", current_ptr, string);
            current_ptr += 4;
        }
    }

    #[hook(unsafe extern "stdcall" ZTUI_expansionselect_setup, offset=0x001291fb)]
    pub fn ztui_expansionselect_setup() {
        info!("ZTUI::expansionselect::setup");
        unsafe { ZTUI_expansionselect_setup.call() };
        get_expansions();

        let name = "TEST";

        // let name = CString::new("TEST").unwrap();
        let name_ptr = CString::new(name).unwrap().into_raw() as u32;

        add_expansion(Expansion { expansion_id: 0x101, name_id: 5012, name_string_start_ptr: name_ptr, name_string_end_ptr: name_ptr + name.len() as u32 + 1, name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1});

        // save_mutex();
    }
}

pub fn init() {
    add_to_command_register("list_expansion".to_string(), command_get_expansions);
    add_to_command_register("get_current_expansion".to_string(), command_get_current_expansion);
    unsafe { custom_expansion::init_detours().unwrap() };
}

