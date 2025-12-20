use openzt_detour_macro::detour_mod;
use tracing::error;

pub fn init_hooks() {
    if unsafe { zoo_resource_mgr::init_detours() }.is_err() {
        error!("Error initialising custom expansion detours");
    };
}

#[detour_mod]
mod zoo_resource_mgr {
    use std::ffi::CString;
    use std::time::Instant;

    use tracing::info;

    use openzt_configparser::ini::Ini;
    use openzt_detour::gen::bfresourcemgr::{CONSTRUCTOR, ADD_PATH};
    use openzt_detour::gen::bfresource::{ATTEMPT, PREPARE};
    use openzt_detour::gen::ztui_general::GET_INFO_IMAGE_NAME;

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
    #[detour(ATTEMPT)]
    unsafe extern "thiscall" fn zoo_bf_resource_attempt(this_ptr: u32, file_name: u32) -> bool {
        if bf_resource_inner(this_ptr, file_name) {
            return true;
        }
        unsafe { ATTEMPT_DETOUR.call(this_ptr, file_name) }
    }

    ///When Zoo Tycoon tries to load a resource we check if it's a resource we've already loaded and return that instead
    #[detour(PREPARE)]
    unsafe extern "thiscall" fn zoo_bf_resource_prepare(this_ptr: u32, file_name: u32) -> bool {
        if bf_resource_inner(this_ptr, file_name) {
            return true;
        }

        unsafe { PREPARE_DETOUR.call(this_ptr, file_name) }
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

    #[detour(CONSTRUCTOR)]
    unsafe extern "thiscall" fn zoo_bf_resource_mgr_constructor(this_ptr: u32) -> u32 {
        info!("BFResourceMgr::constructor({:X})", this_ptr);

        let now = Instant::now();

        let return_value = unsafe { CONSTRUCTOR_DETOUR.call(this_ptr) };

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

            let mut paths: Vec<String> = paths.split(';').map(|s| s.to_owned()).collect();

            if !paths.iter().any(|s| s.trim() == "./mods") {

                info!("Adding mods directory to BFResourceMgr");

                if let Ok(mods_path) = CString::new("./mods") {
                    ADD_PATH.original()(this_ptr, mods_path.as_ptr() as u32);
                }

                paths.insert(0, "./mods".to_owned());
            }

            info!("Loading resources from: {:?}", paths);
            load_resources(paths);
            info!("Resources loaded");
        }
        return_value
    }

    #[detour(GET_INFO_IMAGE_NAME)]
    unsafe extern "cdecl" fn zoo_ui_general_get_info_image_name(id: i32) -> u32 {
        match get_location_or_habitat_by_id(id as u32) {
            Some(resource_ptr) => resource_ptr,
            None => unsafe { GET_INFO_IMAGE_NAME_DETOUR.call(id) },
        }
    }
}
