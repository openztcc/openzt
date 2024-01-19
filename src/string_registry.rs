use retour_utils::hook_module;


#[hook_module("zoo.exe")]
mod load_string_from_resource {

    use tracing::info;

    use crate::debug_dll::{get_string_from_memory, save_string_to_memory};

    #[hook(unsafe extern "stdcall" loadStringFromResource, offset = 0x00004e72)]
    fn load_string_from_resource(handle: u32, id: u32, buffer: u32, size: u32) -> u32 {
        // info!("ZTStringLoad({:#08x}, {}, {:#08x}, {:#08x})", handle, id, buffer, size);
        if id == 9507 {
            save_string_to_memory(buffer, "Frank");
            return 5;
        }
        let return_value = unsafe { loadStringFromResource.call(handle, id, buffer, size) };
        if return_value != 0 {
            info!("{:#08x}: {}: {} -> {}", handle, id, get_string_from_memory(buffer), return_value);
        // } else {
            // info!("string not found!");
        }
    return_value
    }

    #[hook(unsafe extern "stdcall" loadStringFromResourceA, offset = 0x00004e72)]
    fn load_string_from_resource_a(handle: u32, id: u32, buffer: u32, size: u32) -> u32 {
        // info!("ZTStringLoadA({:#08x}, {}, {:#08x}, {:#08x})", handle, id, buffer, size);
        if id == 9507 {
            save_string_to_memory(buffer, "Frank");
            return 5;
        }
        let return_value = unsafe { loadStringFromResourceA.call(handle, id, buffer, size) };
        if return_value != 0 {
            info!("{:#08x}: {}: {} -> {}", handle, id, get_string_from_memory(buffer), return_value);
        } else {
            info!("{:#08x}: {} -> {}", handle, id, return_value);
        }
        return_value
    }
}

pub fn init() {
    unsafe { load_string_from_resource::init_detours().unwrap() };
}