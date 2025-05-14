use retour_utils::hook_module;
use tracing::error;

pub fn init_hooks() {
    if unsafe { zoo_resource_mgr::init_detours() }.is_err() {
        error!("Error initialising custom expansion detours");
    };
}

#[hook_module("zoo.exe")]
mod zoo_resource_mgr {
    use bf_configparser::ini::Ini;
    use tracing::info;

    use crate::{
        resource_manager::{
            bfresourcemgr::BFResourcePtr,
            lazyresourcemap::{check_file, get_file_ptr},
            legacy_loading::{load_resources, OPENZT_DIR0},
            openzt_mods::get_location_or_habitat_by_id,
        },
        util::{get_ini_path, get_string_from_memory, save_to_memory},
    };

    ///When Zoo Tycoon tries to load a resource we check if it's a resource we've already loaded and return that instead
    #[hook(unsafe extern "thiscall" BFResource_attempt, offset = 0x00003891)]
    fn zoo_bf_resource_attempt(this_ptr: u32, file_name: u32) -> u8 {
        if bf_resource_inner(this_ptr, file_name) {
            return 1;
        }
        unsafe { BFResource_attempt.call(this_ptr, file_name) }
    }

    ///When Zoo Tycoon tries to load a resource we check if it's a resource we've already loaded and return that instead
    #[hook(unsafe extern "thiscall" BFResource_prepare, offset = 0x000047f4)]
    fn zoo_bf_resource_prepare(this_ptr: u32, file_name: u32) -> u8 {
        if bf_resource_inner(this_ptr, file_name) {
            return 1;
        }

        unsafe { BFResource_prepare.call(this_ptr, file_name) }
    }

    fn bf_resource_inner(this_ptr: u32, file_name: u32) -> bool {
        let mut file_name_string = get_string_from_memory(file_name).to_lowercase();
        if file_name_string.starts_with(OPENZT_DIR0) {
            match parse_openzt_resource_string(file_name_string.clone()) {
                Ok(resource_name) => {
                    file_name_string = resource_name;
                }
                Err(e) => {
                    info!("Failed to parse openzt resource string: {} {}", file_name_string, e);
                    return false;
                }
            }
        }
        if check_file(&file_name_string)
            && let Some(ptr) = get_file_ptr(&file_name_string)
        {
            let mut bfrp = unsafe { Box::from_raw(ptr as *mut BFResourcePtr) };

            if bfrp.num_refs < 100 {
                bfrp.num_refs = 100;
            }

            let ptr = Box::into_raw(bfrp) as u32;

            save_to_memory(this_ptr, ptr);
            true
        } else {
            if !file_name_string.starts_with("ztat") {
                info!("Missing file: {}", file_name_string);
            }
            false
        }
    }

    fn parse_openzt_resource_string(file_name: String) -> Result<String, &'static str> {
        if file_name.starts_with(OPENZT_DIR0) {
            let split = file_name.split('/').collect::<Vec<&str>>();
            if split.len() == 2 || split.len() == 3 {
                return Ok(split[1].to_owned());
            }
        }
        Err("Invalid openzt resource string")
    }

    #[hook(unsafe extern "thiscall" BFResourceMgr_constructor, offset = 0x0012903f)]
    fn zoo_bf_resource_mgr_constructor(this_ptr: u32) -> u32 {
        info!("BFResourceMgr::constructor({:X})", this_ptr);

        use std::time::Instant;
        let now = Instant::now();

        let return_value = unsafe { BFResourceMgr_constructor.call(this_ptr) };

        let elapsed = now.elapsed();
        info!("Vanilla loading took {:.2?}", elapsed);

        let ini_path = get_ini_path();
        let mut zoo_ini = Ini::new();
        zoo_ini.set_comment_symbols(&['#']);
        if let Err(e) = zoo_ini.load(ini_path) {
            info!("Failed to load zoo.ini: {}", e);
            return return_value;
        };
        if let Some(paths) = zoo_ini.get("resource", "path") {
            info!("Loading resources from: {}", paths);
            load_resources(paths.split(';').map(|s| s.to_owned()).collect());
            info!("Resources loaded");
        }
        return_value
    }

    #[hook(unsafe extern "cdecl" ZTUI_general_getInfoImageName, offset = 0x000f85d2)]
    fn zoo_ui_general_get_info_image_name(id: u32) -> u32 {
        match get_location_or_habitat_by_id(id) {
            Some(resource_ptr) => resource_ptr,
            None => unsafe { ZTUI_general_getInfoImageName.call(id) },
        }
    }
}
