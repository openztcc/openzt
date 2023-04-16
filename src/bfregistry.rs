use std::collections::HashMap;
use std::cell::RefCell;
use tracing::info;

use crate::debug_dll::{get_from_memory, get_string_from_memory};

thread_local! {
    static BF_REGISTRY: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
}

pub fn add_to_registry(key: &String, value: u32) {
    BF_REGISTRY.with(|registry| {
        registry.borrow_mut().insert(key.to_string(), value);
    });
}

pub fn get_from_registry(key: String) -> Option<u32> {
    return BF_REGISTRY.with(|registry| {
        info!("GetFromRegistry {}", key);
        for (key, value) in registry.borrow().clone().into_iter() {
            info!("{}: {:#08x}", key, value);
        }
        info!("GotFromRegistry {:#08x}", registry.borrow().get(&key).cloned().unwrap());
        registry.borrow().get(&key).cloned()
    });
}

pub fn read_bf_registry() {
    let start_ptr: u32 = 0x63f590 as u32;
    if get_from_memory::<u32>(start_ptr) == 0 {
        return;
    } else {
        info!("BFRegistry: {:#08x}", get_from_memory::<u32>(start_ptr));
    }
    let end_ptr: u32 = 0x63f594 as u32;

    let start_address = get_from_memory::<u32>(start_ptr);
    let end_address = get_from_memory::<u32>(end_ptr);
    // let start_val = *start_ptr;
    // let end_val = *end_ptr;

    info!("BFRegistry: {:#08x} -> {:#08x}", start_address, end_address);
    
    let mut current_address = start_address;
    while current_address < end_address {
        if current_address == 0 || get_from_memory::<u32>(current_address) == 0 {
            break;
        }
        info!("BFRegistry: {:#08x}", current_address);
        info!("BFRegistry: {:#08x}", get_from_memory::<u32>(current_address));
        info!("BFRegistry: {}", get_string_from_memory(get_from_memory::<u32>(current_address)));
        
        current_address = current_address + 0x1c;
    }
}