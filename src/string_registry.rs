use std::sync::Mutex;
use std::collections::HashMap;

use once_cell::sync::Lazy;

use retour_utils::hook_module;

use tracing::info;

use configparser::ini::Ini;

use crate::add_to_command_register;

use crate::debug_dll::get_base_path;

use crate::load_ini::load_items_from_section;

static STRING_REGISTRY: Lazy<Mutex<HashMap<u32, String>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn add_to_string_registry(string_id: u32, string_val: String) {
    info!("Registring string {} to registry as id {}", string_val, string_id);
    let mut data_mutex = STRING_REGISTRY.lock().unwrap();
    data_mutex.insert(string_id, string_val);
}

fn get_string_from_registry(string_id: u32) -> Result<String, &'static str> {
    let string = {
        let data_mutex = STRING_REGISTRY.lock().unwrap();
        data_mutex.get(&string_id).cloned()
    };
    match string {
        Some(string) => Ok(string),
        None => Err("String not found")
    }
}

pub fn command_add_string(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() < 2 {
        return Err("Invalid number of arguments");
    }
    match args[0].parse::<u32>() {
        Ok(string_id) => {
            let concat_string = args[1..].join(" ");
            add_to_string_registry(string_id, concat_string);
            Ok("String added".to_string())
        },
        Err(_) => Err("Invalid string id")
    }
}

pub fn command_get_string(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() != 1 {
        return Err("Invalid number of arguments");
    }
    match args[0].parse::<u32>() {
        Ok(string_id) => {
            match get_string_from_registry(string_id) {
                Ok(string) => Ok(string),
                Err(_) => Err("String not found")
            }
        },
        Err(_) => Err("Invalid string id")
    }
}

#[hook_module("zoo.exe")]
pub mod zoo_string {
    use tracing::info;

    use crate::debug_dll::get_string_from_memory;
    use crate::string_registry::get_string_from_registry;

    #[hook(unsafe extern "thiscall" BFApp_loadString, offset = 0x00004e0a)]
    fn bf_app_load_string(_this_ptr: u32, string_id: u32, string_buffer: u32) -> u32 {
        match get_string_from_registry(string_id) {
            Ok(string) => {
                info!("BFMap::loadString string_id: {}, override: {} -> {}", string_id, string, string.len());
                unsafe { std::ptr::copy(string.as_ptr(), string_buffer as *mut u8, string.len()) };
                return string.len() as u32;
            },
            Err(_) => {
                let return_value = unsafe { BFApp_loadString.call(_this_ptr, string_id, string_buffer) };
                return_value
            }
        }
        
    }
}

pub fn init() {
    unsafe { zoo_string::init_detours().unwrap() };
    add_to_command_register("add_string".to_string(), command_add_string);
    add_to_command_register("get_string".to_string(), command_get_string);
    load_overrides_from_ini();
}

fn get_ini_path() -> String {
    let mut base_path = get_base_path();
    base_path.push("zoo.ini");
    base_path.to_str().unwrap().to_string()
}

fn load_overrides_from_ini() {
    let mut zoo_ini = Ini::new();
    zoo_ini.load(get_ini_path()).unwrap();
    load_items_from_section(&zoo_ini, &"strings").iter().for_each(|(key, value)| {
        match key.parse::<u32>() {
            Ok(key) => {
                if let Some(string_override) = value {
                    add_to_string_registry(key, string_override.to_string());
                }
            },
            Err(_) => {}
        }
    });
}