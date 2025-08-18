use openzt_detour_macro::detour_mod;
use tracing::error;

pub fn init_hooks() {
    if unsafe { zoo_resource_mgr::init_detours() }.is_err() {
        error!("Error initialising custom expansion detours");
    };
}

#[detour_mod]
mod zoo_resource_mgr {
    use openzt_configparser::ini::Ini;
    use tracing::info;
    use openzt_detour::{BFRESOURCE_ATTEMPT, BFRESOURCE_PREPARE, BFRESOURCEMGR_CONSTRUCTOR, ZTUI_GENERAL_GET_INFO_IMAGE_NAME};
    use openzt_detour::gen::bfresourcemgr::{ATTEMPT, PREPARE, CONSTRUCTOR};
    use openzt_detour::gen::ztuigeneral::{GET_INFO_IMAGE_NAME};

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
    // TODO: Generated signature returns bool instead of u8 - verify if this matters
    #[detour(BFRESOURCE_ATTEMPT)]
    unsafe extern "thiscall" fn zoo_bf_resource_attempt(this_ptr: u32, file_name: u32) -> bool {
        if bf_resource_inner(this_ptr, file_name) {
            return true;
        }
        unsafe { BFRESOURCE_ATTEMPT_DETOUR.call(this_ptr, file_name) }
    }

    ///When Zoo Tycoon tries to load a resource we check if it's a resource we've already loaded and return that instead
    // TODO: Generated signature returns bool instead of u8 - verify if this matters
    #[detour(BFRESOURCE_PREPARE)]
    unsafe extern "thiscall" fn zoo_bf_resource_prepare(this_ptr: u32, file_name: u32) -> bool {
        if bf_resource_inner(this_ptr, file_name) {
            return true;
        }

        unsafe { BFRESOURCE_PREPARE_DETOUR.call(this_ptr, file_name) }
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
        if !check_file(&file_name_string) {
            return false
        }

        if let Some(ptr) = get_file_ptr(&file_name_string) {
            let mut bfrp = unsafe { Box::from_raw(ptr as *mut BFResourcePtr) };

            if bfrp.num_refs < 100 {
                bfrp.num_refs = 100;
            }

            let ptr = Box::into_raw(bfrp) as u32;

            save_to_memory(this_ptr, ptr);
            true
        } else {
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

    #[detour(BFRESOURCEMGR_CONSTRUCTOR)]
    unsafe extern "thiscall" fn zoo_bf_resource_mgr_constructor(this_ptr: u32) -> u32 {
        info!("BFResourceMgr::constructor({:X})", this_ptr);

        use std::time::Instant;
        let now = Instant::now();

        let return_value = unsafe { BFRESOURCEMGR_CONSTRUCTOR_DETOUR.call(this_ptr) };

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

    // TODO: Generated signature uses i32 instead of u32 for parameter - verify if sign matters
    #[detour(ZTUI_GENERAL_GET_INFO_IMAGE_NAME)]
    unsafe extern "cdecl" fn zoo_ui_general_get_info_image_name(id: i32) -> u32 {
        match get_location_or_habitat_by_id(id as u32) {
            Some(resource_ptr) => resource_ptr,
            None => unsafe { ZTUI_GENERAL_GET_INFO_IMAGE_NAME_DETOUR.call(id) },
        }
    }
}
