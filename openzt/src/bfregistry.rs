use std::{collections::HashMap, sync::Mutex};

use std::sync::LazyLock;
use openzt_detour_macro::detour_mod;
use tracing::info;

use crate::{
    command_console::{add_to_command_register, CommandError},
    util::get_from_memory,
};

static BF_REGISTRY: LazyLock<Mutex<HashMap<String, u32>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn command_list_registry(_args: Vec<&str>) -> Result<String, CommandError> {
    Ok(list_registry()?)
}

pub fn list_registry() -> Result<String, String> {
    let data_mutex = BF_REGISTRY.lock().unwrap();
    let mut string_array = Vec::new();
    for (key, value) in data_mutex.iter() {
        string_array.push(format!("{}: {:#08x}", key, value));
    }
    Ok(string_array.join("\n"))
}

pub fn add_to_registry(key: &String, value: u32) {
    let mut data_mutex = BF_REGISTRY.lock().unwrap();
    data_mutex.insert(key.to_string(), value);
}

pub fn get_from_registry(key: String) -> Option<u32> {
    let data_mutex = BF_REGISTRY.lock().unwrap();

    data_mutex.get(&key).cloned()
}

pub fn read_bf_registry() {
    let start_ptr: u32 = 0x63f590;
    if get_from_memory::<u32>(start_ptr) == 0 {
        return;
    }
    let end_ptr: u32 = 0x63f594;

    let start_address = get_from_memory::<u32>(start_ptr);
    let end_address = get_from_memory::<u32>(end_ptr);

    info!("BFRegistry: {:#08x} -> {:#08x}", start_address, end_address);

    let mut current_address = start_address;
    while current_address < end_address {
        if current_address == 0 || get_from_memory::<u32>(current_address) == 0 {
            break;
        }
        current_address += 0x1c;
    }
}

#[detour_mod]
mod zoo_bf_registry {
    use crate::{
        bfregistry::{add_to_registry, get_from_registry},
        util::{get_from_memory, get_string_from_memory},
    };
    use openzt_detour::{BFREGISTRY_PRTGET, BFREGISTRY_ADD, BFREGISTRY_ADDUI};

    // TODO: Generated signature is missing third parameter - verify if it's actually needed
    #[detour(BFREGISTRY_PRTGET)]
    unsafe extern "thiscall" fn prt_get(_this_prt: u32, class_name: u32) -> u32 {
        get_from_registry(get_string_from_memory(get_from_memory::<u32>(class_name))).unwrap()
    }

    #[detour(BFREGISTRY_ADD)]
    unsafe extern "cdecl" fn add_to_bfregistry(param_1: u32, param_2: u32) -> u32 {
        let param_1_string = get_string_from_memory(get_from_memory::<u32>(param_1));
        add_to_registry(&param_1_string, param_2);
        0x638001
    }

    #[detour(BFREGISTRY_ADDUI)]
    unsafe extern "cdecl" fn add_to_bfregistry_ui(param_1: u32, param_2: u32) -> u32 {
        let param_1_string = get_string_from_memory(get_from_memory::<u32>(param_1));
        add_to_registry(&param_1_string, param_2);
        0x638001
    }
}

#[deprecated(since = "0.1.0", note = "no longer needed")]
pub fn init() {
    if let Err(e) = unsafe { zoo_bf_registry::init_detours() } {
        info!("Error initialising bf_registry detours: {}", e);
    };
    add_to_command_register("list_bf_registry".to_owned(), command_list_registry)
}
