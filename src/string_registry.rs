use std::sync::Mutex;

use once_cell::sync::Lazy;
use retour_utils::hook_module;
use tracing::info;

const STRING_REGISTRY_ID_OFFSET: u32 = 100_000;

static STRING_REGISTRY: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn add_string_to_registry(string_val: String) -> u32 {
    let mut data_mutex = STRING_REGISTRY.lock().unwrap();
    data_mutex.push(string_val);
    info!(
        "Added string to registry: {}",
        data_mutex.len() as u32 + STRING_REGISTRY_ID_OFFSET - 1
    );
    data_mutex.len() as u32 + STRING_REGISTRY_ID_OFFSET - 1
}

pub fn get_string_from_registry(string_id: u32) -> Result<String, &'static str> {
    info!("Getting string from registry: {}", string_id);
    let string = {
        let data_mutex = STRING_REGISTRY.lock().unwrap();
        data_mutex
            .get((string_id - STRING_REGISTRY_ID_OFFSET) as usize)
            .cloned()
    };
    match string {
        Some(string) => Ok(string),
        None => Err("String not found"),
    }
}

#[hook_module("zoo.exe")]
pub mod zoo_string {
    use tracing::info;

    use super::STRING_REGISTRY_ID_OFFSET;
    use crate::{debug_dll::save_string_to_memory, string_registry::get_string_from_registry};

    #[hook(unsafe extern "thiscall" BFApp_loadString, offset = 0x00004e0a)]
    fn bf_app_load_string(_this_ptr: u32, string_id: u32, string_buffer: u32) -> u32 {
        if string_id >= STRING_REGISTRY_ID_OFFSET {
            if let Ok(string) = get_string_from_registry(string_id) {
                info!(
                    "BFApp::loadString string_id: {}, override: {} -> {}",
                    string_id,
                    string,
                    string.len()
                );
                save_string_to_memory(string_buffer, &string);
                return string.len() as u32 + 1;
            }
        }
        let return_value = unsafe { BFApp_loadString.call(_this_ptr, string_id, string_buffer) };
        return_value
    }
}

pub fn init() {
    unsafe { zoo_string::init_detours().unwrap() };
}
