
use std::fmt;

use retour_utils::hook_module;

use tracing::{info, error};

use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::console::add_to_command_register;

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

struct BFResourceDirContents {
    dir: BFResourceDir,
    zips: Vec<BFResourceZip>,
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
    let mut current_bf_resource_dir = get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
    bf_resource_dir_ptr += 4;

    while bf_resource_dir_ptr < bf_resource_mgr.resource_array_end {
        let class = get_from_memory::<u32>(get_from_memory::<u32>(bf_resource_dir_ptr));
        match class {
            0x630aec => {
                bf_resource_dir_contents.push(BFResourceDirContents { dir: current_bf_resource_dir, zips: bf_resource_zips });
                current_bf_resource_dir = get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
                bf_resource_zips = Vec::new();
                bf_resource_dir_ptr += 4;
            }
            0x630b0c => {
                let zip = get_from_memory::<BFResourceZip>(get_from_memory::<u32>(bf_resource_dir_ptr));
                info!("Found zip: {}, at {:X}", zip.name(), bf_resource_dir_ptr);
                bf_resource_zips.push(zip);
                bf_resource_dir_ptr += 4;
            }
            _ => {
                error!("Unknown class: 0x{:X}", class);
                bf_resource_dir_ptr += 4;
            }
        }
    }
    bf_resource_dir_contents.push(BFResourceDirContents { dir: current_bf_resource_dir, zips: bf_resource_zips });
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
        result_string.push_str(&format!("{} ({})\n", get_string_from_memory(bf_resource_dir.dir_name_string_start), bf_resource_dir.num_child_files));
        let bf_resource_zips = bf_resource_dir_content.zips;
        for bf_resource_zip in bf_resource_zips {
            result_string.push_str(&format!("{}\n", get_string_from_memory(bf_resource_zip.zip_name_string_start)));
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
}

#[hook_module("zoo.exe")]
pub mod zoo_resource_mgr {
    use tracing::info;

    use super::{BFResourceZip, Name};

    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    #[hook(unsafe extern "thiscall" BFResourceMgr_find, offset = 0x000b9a40)]
    fn zoo_bf_resource_mgr_find(this_ptr: u32, buffer_ptr: u32, file_name: u32, file_extension: u32) -> u32 {
        info!("BFResourceMgr::find({:X}, {:X}, {}, {})", this_ptr, buffer_ptr, get_string_from_memory(file_name), get_string_from_memory(file_extension));
        let return_value = unsafe { BFResourceMgr_find.call(this_ptr, buffer_ptr, file_name, file_extension) };
        info!("BFResourceMgr::find({:X}, {:X}, {}, {}) -> {:X} -> {:X}", this_ptr, buffer_ptr, get_string_from_memory(file_name), get_string_from_memory(file_extension), return_value, get_from_memory::<u32>(return_value));
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceMgr_findall, offset = 0x000bf92b)]
    fn zoo_bf_resource_mgr_findall(this_ptr: u32, buffer_ptr: u32, file_extension: u32) -> u32 {
        info!("BFResourceMgr::findall({:X}, {:X}, {})", this_ptr, buffer_ptr, get_string_from_memory(file_extension));
        let return_value = unsafe { BFResourceMgr_findall.call(this_ptr, buffer_ptr, file_extension) };
        info!("BFResourceMgr::findall({:X}, {:X}, {}) -> {:X} -> {:X}", this_ptr, buffer_ptr, get_string_from_memory(file_extension), return_value, get_from_memory::<u32>(return_value));
        info!("{:X}", get_from_memory::<u32>(buffer_ptr));
        return_value
    }

    #[hook(unsafe extern "thiscall" ZTFoodType_ZTFoodType, offset = 0x00065634)]
    fn zt_food_type_zt_food_type(this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) -> u32 {
        let return_value = unsafe { ZTFoodType_ZTFoodType.call(this_ptr, param_1, param_2, param_3) };
        info!("ZTFoodType::ZTFoodType({:X}, {}, {}, {}) -> {:X}", this_ptr, get_string_from_memory(get_from_memory::<u32>(param_1)), get_string_from_memory(get_from_memory::<u32>(param_2)), get_string_from_memory(param_3), return_value);
        return_value
    }

    // #[hook(unsafe extern "thiscall" unknown_func_1, offset = 0x000b3805)]
    // fn unknown_1(this_ptr: u32) -> u32 {
    //     let return_value = unsafe { unknown_func_1.call(this_ptr) };
    //     info!("unknown_1({:X}->{:X}) -> {:X} -> {:X}", this_ptr, get_from_memory::<u32>(this_ptr), return_value, get_from_memory::<u32>(return_value));
    //     return_value
    // }

    // #[hook(unsafe extern "thiscall" unknown_func_2, offset = 0x0000a5bc)]
    // fn unknown_2(this_ptr: u32) {
    //     info!("unknown_2({:X} -> {:X})", this_ptr, get_from_memory::<u32>(this_ptr));
    //     unsafe { unknown_func_2.call(this_ptr) };
    // }

    #[hook(unsafe extern "thiscall" BFResourceZip_BFResourceZip, offset = 0x000128ac1)]
    fn bf_resource_zip_bf_resource_zip(this_ptr: u32, param_1: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_BFResourceZip.call(this_ptr, param_1) };
        info!("BFResourceZip::BFResourceZip({:X}, {}) -> {:X}", this_ptr, get_string_from_memory(param_1), return_value);
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_1, offset = 0x0003a1c)] //load, getResourcePtr?
    fn unknown_3(this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_unknown_1.call(this_ptr, param_1, param_2) };
        if return_value != 0 {
            let name = get_from_memory::<BFResourceZip>(this_ptr).name();
            info!("unknown_3({:X} ({}), {:X}, {:X}) -> {:X}", this_ptr, name, param_1, param_2, return_value);
        }
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_2, offset = 0x000b9b73)]   //find("filename_prefix", "extension")
    fn bf_resource_zip_load(this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_unknown_2.call(this_ptr, param_1, param_2, param_3) };
        let name = get_from_memory::<BFResourceZip>(this_ptr).name();
        info!("BFResourceZip::find({:X} ({}), {:X}, {}, {}) -> {:X} -> {}", this_ptr, name, param_1, get_string_from_memory(param_2), get_string_from_memory(param_3), return_value, get_string_from_memory(get_from_memory::<u32>(return_value)));
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_3, offset = 0x000bf9fb)]  //find("extenion") -> pointer to list?
    fn unknown_5(this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_unknown_3.call(this_ptr, param_1, param_2) };
        let name = get_from_memory::<BFResourceZip>(this_ptr).name();
        info!("BFResourceZip::findall({:X} ({}), {:X}, {}) -> {:X}", this_ptr, name, param_1, get_string_from_memory(param_2), return_value);
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_4, offset = 0x00017e351)]
    fn unknown_6(this_ptr: u32, param_1: u32) -> u32 {
        let name = get_from_memory::<BFResourceZip>(this_ptr).name();
        let return_value = unsafe { BFResourceZip_unknown_4.call(this_ptr, param_1) };
        info!("unknown_6({:X} ({}), {:X}) -> {:X}", this_ptr, name, param_1, return_value);
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_5, offset = 0x000202f65)]
    fn unknown_7(this_ptr: u32, param_1: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_unknown_5.call(this_ptr, param_1) };
        info!("unknown_7({:X}, {:X}) -> {:X}", this_ptr, param_1, return_value);
        return_value
    }

    // thiscall 2 params offset 3b43
    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_6, offset = 0x0003b43)]
    fn unknown_8(this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_unknown_6.call(this_ptr, param_1, param_2) };
        if return_value != 0 {
            let name = get_from_memory::<BFResourceZip>(this_ptr).name();
            info!("unknown_8({:X} ({}), {:X}, {:X}) -> {:X}", this_ptr, name, param_1, param_2, return_value);
        }
        return_value
    }

    #[hook(unsafe extern "thiscall" BFResourceZip_unknown_7, offset = 0x00048da)]
    fn unknown_9(this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) -> u32 {
        let return_value = unsafe { BFResourceZip_unknown_7.call(this_ptr, param_1, param_2, param_3) };
        if return_value != 0 {
            let name = get_from_memory::<BFResourceZip>(this_ptr).name();
            info!("unknown_9({:X} ({}), {:X}, {:X}, {:X}) -> {:X}", this_ptr, name, param_1, param_2, param_3, return_value);
        }
        return_value
    }


}