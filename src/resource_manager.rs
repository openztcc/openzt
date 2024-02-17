use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use walkdir::{DirEntry, WalkDir};

use retour_utils::hook_module;
use tracing::{error, info};

use crate::console::add_to_command_register;
use crate::debug_dll::{get_from_memory, get_string_from_memory};

const GLOBAL_BFRESOURCEMGR_ADDRESS: u32 = 0x006380C0;

#[derive(Debug)]
#[repr(C)]
struct BFResourceMgr {
    resource_array_start: u32,
    resource_array_end: u32,
    resource_array_buffer_end: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_u8_1: u8,
}

#[derive(Debug)]
#[repr(C)]
struct BFResourceDir {
    class: u32,
    unknown_u32_1: u32,
    dir_name_string_start: u32,
    dir_name_string_end: u32,
    unknown_u32_2: u32,
    num_child_files: u32,
    unknown_u32_3: u32,
}

#[derive(Debug)]
#[repr(C)]
struct BFResourceZip {
    class: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_u32_3: u32,
    zip_name_string_start: u32,
    contents_tree: u32, //? contents end?
}

struct BFResourceDirContents {
    dir: BFResourceDir,
    zips: Vec<BFResourceZip>,
}

trait Name {
    fn name(&self) -> String;
}

impl Name for BFResourceDir {
    fn name(&self) -> String {
        get_string_from_memory(self.dir_name_string_start)
    }
}

impl Name for BFResourceZip {
    fn name(&self) -> String {
        get_string_from_memory(self.zip_name_string_start)
    }
}

fn read_bf_resource_mgr_from_memory() -> BFResourceMgr {
    get_from_memory::<BFResourceMgr>(GLOBAL_BFRESOURCEMGR_ADDRESS)
}

fn read_bf_resource_dir_contents_from_memory() -> Vec<BFResourceDirContents> {
    info!("Reading BFResourceDir from memory");
    let bf_resource_mgr = read_bf_resource_mgr_from_memory();
    let mut bf_resource_dir_contents: Vec<BFResourceDirContents> = Vec::new();
    let mut bf_resource_dir_ptr = bf_resource_mgr.resource_array_start;
    let mut bf_resource_zips: Vec<BFResourceZip> = Vec::new();
    let mut current_bf_resource_dir =
        get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
    bf_resource_dir_ptr += 4;

    while bf_resource_dir_ptr < bf_resource_mgr.resource_array_end {
        let class = get_from_memory::<u32>(get_from_memory::<u32>(bf_resource_dir_ptr));
        match class {
            0x630aec => {
                bf_resource_dir_contents.push(BFResourceDirContents {
                    dir: current_bf_resource_dir,
                    zips: bf_resource_zips,
                });
                current_bf_resource_dir =
                    get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
                bf_resource_zips = Vec::new();
                bf_resource_dir_ptr += 4;
            }
            0x630b0c => {
                bf_resource_zips.push(get_from_memory::<BFResourceZip>(get_from_memory::<u32>(
                    bf_resource_dir_ptr,
                )));
                bf_resource_dir_ptr += 4;
            }
            _ => {
                error!("Unknown class: 0x{:X}", class);
                bf_resource_dir_ptr += 4;
            }
        }
    }
    bf_resource_dir_contents.push(BFResourceDirContents {
        dir: current_bf_resource_dir,
        zips: bf_resource_zips,
    });
    bf_resource_dir_contents
}

impl fmt::Display for BFResourceMgr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BFResourceMgr {{ resource_array_start: 0x{:X}, resource_array_end: 0x{:X}, resource_array_buffer_end: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u8_1: 0x{:X} }}", self.resource_array_start, self.resource_array_end, self.resource_array_buffer_end, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u8_1)
    }
}

impl fmt::Display for BFResourceDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dir_name_string = get_string_from_memory(self.dir_name_string_start);
        write!(f, "BFResourceDir {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, dir_name: {}, num_bfr_zip: 0x{:X}, unknown_u32_2: 0x{:X} }}", self.class, self.unknown_u32_1, dir_name_string, self.num_child_files, self.unknown_u32_2)
    }
}

impl fmt::Display for BFResourceZip {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let zip_name_string = get_string_from_memory(self.zip_name_string_start);
        write!(f, "BFResourceZip {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u32_3: 0x{:X}, zip_name: {}, contents_tree: 0x{:X} }}", self.class, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u32_3, zip_name_string, self.contents_tree)
    }
}

fn command_list_resources(_args: Vec<&str>) -> Result<String, &'static str> {
    let mut result_string = String::new();
    let bf_resource_dir_contents = read_bf_resource_dir_contents_from_memory();
    for bf_resource_dir_content in bf_resource_dir_contents {
        let bf_resource_dir = bf_resource_dir_content.dir;
        result_string.push_str(&format!(
            "{} ({})\n",
            get_string_from_memory(bf_resource_dir.dir_name_string_start),
            bf_resource_dir.num_child_files
        ));
        let bf_resource_zips = bf_resource_dir_content.zips;
        for bf_resource_zip in bf_resource_zips {
            result_string.push_str(&format!(
                "{}\n",
                get_string_from_memory(bf_resource_zip.zip_name_string_start)
            ));
        }
    }
    Ok(result_string)
}

fn command_get_bf_resource_mgr(_args: Vec<&str>) -> Result<String, &'static str> {
    let bf_resource_mgr = read_bf_resource_mgr_from_memory();
    Ok(format!("{}", bf_resource_mgr))
}

pub fn init() {
    add_to_command_register("list_resources".to_owned(), command_list_resources);
    add_to_command_register("get_bfresourcemgr".to_owned(), command_get_bf_resource_mgr);
    unsafe { zoo_resource_mgr::init_detours().unwrap() };
    //TODO: Load resources from ZTD files
}

#[hook_module("zoo.exe")]
pub mod zoo_resource_mgr {
    use std::ffi::CString;

    use tracing::info;

    use super::{BFResourceZip, Name};

    use configparser::ini::Ini; //TODO: Replace with custom ini parser

    use crate::debug_dll::{get_from_memory, get_ini_path, get_string_from_memory};

    #[hook(unsafe extern "thiscall" BFResourceMgr_find, offset = 0x000b9a40)]
    fn zoo_bf_resource_mgr_find(
        this_ptr: u32,
        buffer_ptr: u32,
        file_name: u32,
        file_extension: u32,
    ) -> u32 {
        info!(
            "BFResourceMgr::find({:X}, {:X}, {}, {})",
            this_ptr,
            buffer_ptr,
            get_string_from_memory(file_name),
            get_string_from_memory(file_extension)
        );
        let return_value =
            unsafe { BFResourceMgr_find.call(this_ptr, buffer_ptr, file_name, file_extension) };
        info!(
            "BFResourceMgr::find({:X}, {:X}, {}, {}) -> {:X} -> {:X}",
            this_ptr,
            buffer_ptr,
            get_string_from_memory(file_name),
            get_string_from_memory(file_extension),
            return_value,
            get_from_memory::<u32>(return_value)
        );
        info!(
            "BFConfigFile {}",
            get_string_from_memory(get_from_memory::<u32>(return_value) + 0x10)
        );
        return_value
    }

    #[hook(unsafe extern "thiscall" ZTAdvTerrainMgr_loadTextures, offset = 0x001224b9)]
    fn zoo_zt_adv_terrain_mgr_load_textures(this_ptr: u32) -> u32 {
        info!("ZTAdvTerrainMgr::loadTextures({:X})", this_ptr);
        let return_value = unsafe { ZTAdvTerrainMgr_loadTextures.call(this_ptr) };
        info!(
            "ZTAdvTerrainMgr::loadTextures({:X}) -> {:X}",
            this_ptr, return_value
        );
        return_value
    }

    #[hook(unsafe extern "thiscall" BFTerrainTypeInfo_initialize, offset = 0x00123c58)]
    fn zoo_bf_terrain_type_info_initialize(this_ptr: u32, config_ptr: u32, name: u32) -> u32 {
        info!(
            "BFTerrainTypeInfo::initialize({:X}, {:X}, {})",
            this_ptr,
            config_ptr,
            get_string_from_memory(name)
        );
        let return_value = unsafe { BFTerrainTypeInfo_initialize.call(this_ptr, config_ptr, name) };
        info!(
            "BFTerrainTypeInfo::initialize({:X}, {:X}, {}) -> {:X}",
            this_ptr,
            config_ptr,
            get_string_from_memory(name),
            return_value
        );
        return_value
    }

    #[hook(unsafe extern "thiscall" BFMap_paintCell, offset = 0x000f8fd8)]
    fn zoo_bf_map_paint_cell(this_ptr: u32, bf_terrain_type_info_ptr: u32, param: bool) -> u32 {
        info!(
            "BFMap::paintCell({:X}, {:X}, {} -> {:X})",
            this_ptr,
            bf_terrain_type_info_ptr,
            param,
            get_from_memory::<u32>(bf_terrain_type_info_ptr)
        );
        let return_value =
            unsafe { BFMap_paintCell.call(this_ptr, bf_terrain_type_info_ptr, param) };
        info!(
            "BFMap::paintCell({:X}, {:X}, {}) -> {:X}",
            this_ptr, bf_terrain_type_info_ptr, param, return_value
        );
        return_value
    }

    // #[hook(unsafe extern "thiscall" BFMap_paintCell2, offset = 0x000f17e0)]
    // fn zoo_bf_tile_set_terrain_type(this_ptr: u32, bf_terrain_type_info_ptr: u32) -> u32 {
    //     info!("BFTile::setTerrainType({:X}, {:X} -> {:X})", this_ptr, bf_terrain_type_info_ptr, get_from_memory(bf_terrain_type_info_ptr));
    //     let return_value = unsafe { BFMap_paintCell.call(this_ptr, bf_terrain_type_info_ptr) };
    //     info!("BFTile::setTerrainType({:X}, {:X}) -> {:X}", this_ptr, bf_terrain_type_info_ptr, return_value);
    //     return_value
    // }

    #[hook(unsafe extern "thiscall" GXImageTGA_attempt, offset = 0x000b32a7)]
    fn zoo_gx_image_tga_attempt(this_ptr: u32, file_name: u32) -> u32 {
        info!(
            "GXImageTGA::attempt({:X}, {})",
            this_ptr,
            get_string_from_memory(file_name)
        );
        let return_value = unsafe { GXImageTGA_attempt.call(this_ptr, file_name) };
        // info!("GXImageTGA::attempt({:X}, {:X}) -> {:X}", this_ptr, file_name, return_value);
        info!(
            "GXImageTGA::attempt({:X}, {}) -> {:X}",
            this_ptr,
            get_string_from_memory(file_name),
            return_value
        );
        return_value
    }

    #[hook(unsafe extern "thiscall" BFUIMgr_configureAndShowDialog, offset = 0x001a1b15)]
    fn zoo_bf_ui_mgr_configure_and_show_dialog(
        this_ptr: u32,
        bf_config_file_ptr: u32,
        param_2: u32,
        param_3: bool,
    ) -> u32 {
        info!(
            "BFUIMgr::configureAndShowDialog({:X}, {:X}, {}, {})",
            this_ptr,
            bf_config_file_ptr,
            get_string_from_memory(param_2),
            param_3
        );
        let return_value = unsafe {
            BFUIMgr_configureAndShowDialog.call(this_ptr, bf_config_file_ptr, param_2, param_3)
        };
        info!(
            "BFUIMgr::configureAndShowDialog({:X}, {:X}, {}, {}) -> {:X}",
            this_ptr,
            bf_config_file_ptr,
            get_string_from_memory(param_2),
            param_3,
            return_value
        );
        return_value
    }

    #[hook(unsafe extern "thiscall" BFApp_getInstalledExpansion, offset = 0x000ab32c)]
    fn zoo_bf_app_get_installed_expansion(this_ptr: u32, param_1: u32) -> u32 {
        info!(
            "BFApp::getInstalledExpansion({:X}, {:X})",
            this_ptr,
            param_1
        );
        let return_value = unsafe { BFApp_getInstalledExpansion.call(this_ptr, param_1) };
        info!(
            "BFApp::getInstalledExpansion({:X}, {:X}) -> {:X}",
            this_ptr,
            param_1,
            return_value
        );
        return_value
    }
    
    #[hook(unsafe extern "thiscall" BFResourceMgr_constructor, offset = 0x0012903f)]
    fn zoo_bf_resource_mgr_constructor(this_ptr: u32) -> u32 {
        let return_value = unsafe { BFResourceMgr_constructor.call(this_ptr) };
        let ini_path = get_ini_path();
        let mut zoo_ini = Ini::new();                       //TODO: Load this once on startup; fix up load_ini to actually contain all the ini related functions
        zoo_ini.load(ini_path).unwrap();
        if let Some(paths) = zoo_ini.get("resource", "path") {
            if !paths.split(';').any(|s| s.trim() == "./mods") {
                info!("Adding mods directory to BFResourceMgr");
                let add_path: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(0x0052870b) };
                if let Ok(mods_path) = CString::new("./mods") {
                    add_path(this_ptr, mods_path.as_ptr() as u32);
                }
            }
        }
        return_value
    }
}

// for entry in WalkDir::new("/Users/finnhartshorn/Projects/zootycoon/vanilla_install/Zoo Tycoon")

fn get_ztd_resources(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut resources = Vec::new();
    let walker = WalkDir::new(dir).follow_links(true).max_depth(if recursive { 0 } else { 1 });
    for entry in walker {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_str()
            .map(|s| s.ends_with(".ztd"))
            .unwrap_or(false)
        {
            println!("{}", &entry.path().display());
            resources.push(entry.path().to_path_buf());
        }
    }
    resources
}

// fn get_default_ztd_resources() -> Vec<PathBuf> {
