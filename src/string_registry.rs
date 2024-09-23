use std::sync::Mutex;

use once_cell::sync::Lazy;
use retour_utils::hook_module;
use tracing::info;

use crate::command_console::{add_to_command_register, CommandError};

const STRING_REGISTRY_ID_OFFSET: u32 = 100_000;

const GLOBAL_BFAPP: u32 = 0x00638148;

static STRING_REGISTRY: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn add_string_to_registry(string_val: String) -> u32 {
    let mut data_mutex = STRING_REGISTRY.lock().unwrap();
    info!(
        "Added string to registry: {} -> {}",
        string_val.clone(),
        data_mutex.len() as u32 + STRING_REGISTRY_ID_OFFSET
    );
    data_mutex.push(string_val);
    data_mutex.len() as u32 + STRING_REGISTRY_ID_OFFSET - 1
}

pub fn get_string_from_registry(string_id: u32) -> Result<String, &'static str> {
    info!("Getting string from registry: {}", string_id);
    let string = {
        let data_mutex = STRING_REGISTRY.lock().unwrap();
        data_mutex.get((string_id - STRING_REGISTRY_ID_OFFSET) as usize).cloned()
    };
    match string {
        Some(string) => Ok(string),
        None => {
            info!("String not found");
            Err("String not found")
        }
    }
}

fn is_user_type_id(param_1: u32) -> bool {
    (19000..=21999).contains(&param_1) || (49000..=51999).contains(&param_1) || (74000..=76999).contains(&param_1)
}

fn command_get_string(args: Vec<&str>) -> Result<String, CommandError> {
    if args.is_empty() {
        return Err(Into::into("Usage: make_sel <id>"));
    }
    let string_id = args[0].parse::<u32>()?;

    let bfapp_load_string: extern "thiscall" fn(u32, u32, u32) -> u32 = unsafe { std::mem::transmute(0x00404e0a) };

    if let Ok(string) = get_string_from_registry(string_id) {
        Ok(format!("OpenZT: {}", string))
    } else {
        info!("String not in registry, calling ZT");
        let buffer = &mut [0u8; 200];
        let length = bfapp_load_string(GLOBAL_BFAPP, string_id, buffer.as_mut_ptr() as u32);
        if length == 0 {
            return Err(Into::into("String not found"));
        }
        let string_slice = &buffer[..length as usize];
        return Ok(String::from_utf8_lossy(string_slice).to_string());
    }
}

#[hook_module("zoo.exe")]
pub mod zoo_string {
    use tracing::info;

    use super::{is_user_type_id, STRING_REGISTRY_ID_OFFSET};
    use crate::{string_registry::get_string_from_registry, util::save_string_to_memory};

    #[hook(unsafe extern "thiscall" BFApp_loadString, offset = 0x00004e0a)]
    fn bf_app_load_string(this_ptr: u32, string_id: u32, string_buffer: u32) -> u32 {
        if is_user_type_id(string_id) {
            info!("BFApp::loadString {:#x} {} {:#x}", this_ptr, string_id, string_buffer);
        }
        if string_id >= STRING_REGISTRY_ID_OFFSET {
            if let Ok(string) = get_string_from_registry(string_id) {
                info!("BFApp::loadString string_id: {}, override: {} -> {}", string_id, string, string.len());
                save_string_to_memory(string_buffer, &string);
                return string.len() as u32 + 1;
            }
        }
        unsafe { BFApp_loadString.call(this_ptr, string_id, string_buffer) }
    }

    // #[hook(unsafe extern "thiscall" BFWorldMgr_unknown, offset = 0x00010d48)]
    // fn bf_world_mgr_unknown(this_ptr: u32, base_user_id: u32) -> u32 {
    //     let return_value = unsafe { BFWorldMgr_unknown.call(this_ptr, base_user_id) };
    //     info!("BFWorldMgr::unknown {:#x} {} -> {:#x}", this_ptr, base_user_id, return_value);
    //     return_value
    // }

    // #[hook(unsafe extern "cdecl" BFEntityType_getUserDataIndex, offset = 0x0001fe27a)]
    // fn bf_entity_type_get_user_data_index(param_1: u32) -> u8 {
    //     let return_value = unsafe { BFEntityType_getUserDataIndex.call(param_1) };
    //     info!("BFEntityType::getUserDataIndex {} -> {}", param_1, return_value);
    //     return_value
    // }

    // #[hook(unsafe extern "thiscall" BFEntityType_getUserData, offset = 0x0001fe1ea)]
    // fn bf_entity_type_get_user_data(this_ptr: u32, user_data_index: u32, param_2: u32) -> u32 {
    //     let return_value =
    //         unsafe { BFEntityType_getUserData.call(this_ptr, user_data_index, param_2) };
    //     info!(
    //         "BFEntityType::getUserData {:#x} {} {} -> {:#x}",
    //         this_ptr, user_data_index, param_2, return_value
    //     );
    //     return_value
    // }
}

pub fn init() {
    if unsafe { zoo_string::init_detours() }.is_err() {
        info!("Failed to initialize string_registry detours");
    }
    add_to_command_register("get_string".to_string(), command_get_string)
}
