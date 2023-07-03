use std::collections::HashMap;
use std::cell::RefCell;
use tracing::{info, error};
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::debug_dll::{get_from_memory, get_string_from_memory};

static BF_REGISTRY: Lazy<Mutex<HashMap<String, u32>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn command_list_registry(args: Vec<&str>) -> Result<String, &'static str> {
    list_registry()
}

pub fn list_registry() -> Result<String, &'static str> {
    let data_mutex = BF_REGISTRY.lock().unwrap();
    let mut string_array = Vec::new();
    for (key, value) in data_mutex.iter() {
        // info!("{}: {:#08x}", key, value);
        string_array.push(format!("{}: {:#08x}", key, value));
    }
    Ok(string_array.join("\n"))
}

pub fn add_to_registry(key: &String, value: u32) {
    println!("Adding {} to registry", key);
    let mut data_mutex = BF_REGISTRY.lock().unwrap();
    data_mutex.insert(key.to_string(), value);
}

pub fn get_from_registry(key: String) -> Option<u32> {
    println!("Getting {} from registry", key);
    let data_mutex = BF_REGISTRY.lock().unwrap();
    
    data_mutex.get(&key).cloned()
}

pub fn read_bf_registry() {
    let start_ptr: u32 = 0x63f590 as u32;
    if get_from_memory::<u32>(start_ptr) == 0 {
        return;
    }
    let end_ptr: u32 = 0x63f594 as u32;

    let start_address = get_from_memory::<u32>(start_ptr);
    let end_address = get_from_memory::<u32>(end_ptr);

    info!("BFRegistry: {:#08x} -> {:#08x}", start_address, end_address);
    
    let mut current_address = start_address;
    while current_address < end_address {
        if current_address == 0 || get_from_memory::<u32>(current_address) == 0 {
            break;
        }
        current_address = current_address + 0x1c;
    }
}