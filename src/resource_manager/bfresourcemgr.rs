use std::fmt::{Display, Formatter, Result};

use public::public;
use tracing::{error, info};

use crate::util::{get_from_memory, get_string_from_memory};

const GLOBAL_BFRESOURCEMGR_ADDRESS: u32 = 0x006380C0;

#[public]
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

#[public]
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

#[public]
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

#[public]
#[derive(Debug)]
#[repr(C)]
struct BFResourceDirContents {
    dir: BFResourceDir,
    zips: Vec<BFResourceZip>,
}

#[public]
#[derive(Debug)]
#[repr(C)]
struct BFResource {
    bf_resource_ptr_ptr: u32,
}

#[public]
#[derive(Debug, Clone)]
#[repr(C)]
struct BFResourcePtr {
    num_refs: u32,
    bf_zip_name_ptr: u32,
    bf_resource_name_ptr: u32,
    data_ptr: u32,
    content_size: u32,
}

#[derive(Debug)]
#[repr(C)]
struct GXLLEAnim {
    padding: [u8; 5],
    bfresource_maybe: u32,
}

impl Display for BFResourcePtr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "BFResourcePtr {{ num_refs: {:#x}, bf_zip_name: {}, bf_resource_name: {}, data_ptr: {:#x}, content_size: {:#x} }}",
            self.num_refs,
            get_string_from_memory(self.bf_zip_name_ptr),
            get_string_from_memory(self.bf_resource_name_ptr),
            self.data_ptr,
            self.content_size
        )
    }
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

pub fn read_bf_resource_mgr_from_memory() -> BFResourceMgr {
    get_from_memory::<BFResourceMgr>(GLOBAL_BFRESOURCEMGR_ADDRESS)
}

pub fn read_bf_resource_dir_contents_from_memory() -> Vec<BFResourceDirContents> {
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
                bf_resource_dir_contents.push(BFResourceDirContents {
                    dir: current_bf_resource_dir,
                    zips: bf_resource_zips,
                });
                current_bf_resource_dir = get_from_memory::<BFResourceDir>(get_from_memory::<u32>(bf_resource_dir_ptr));
                bf_resource_zips = Vec::new();
                bf_resource_dir_ptr += 4;
            }
            0x630b0c => {
                bf_resource_zips.push(get_from_memory::<BFResourceZip>(get_from_memory::<u32>(bf_resource_dir_ptr)));
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

impl Display for BFResourceMgr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "BFResourceMgr {{ resource_array_start: 0x{:X}, resource_array_end: 0x{:X}, resource_array_buffer_end: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u8_1: 0x{:X} }}",
            self.resource_array_start, self.resource_array_end, self.resource_array_buffer_end, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u8_1
        )
    }
}

impl Display for BFResourceDir {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let dir_name_string = get_string_from_memory(self.dir_name_string_start);
        write!(
            f,
            "BFResourceDir {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, dir_name: {}, num_bfr_zip: 0x{:X}, unknown_u32_2: 0x{:X} }}",
            self.class, self.unknown_u32_1, dir_name_string, self.num_child_files, self.unknown_u32_2
        )
    }
}

impl Display for BFResourceZip {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let zip_name_string = get_string_from_memory(self.zip_name_string_start);
        write!(
            f,
            "BFResourceZip {{ class: 0x{:X}, unknown_u32_1: 0x{:X}, unknown_u32_2: 0x{:X}, unknown_u32_3: 0x{:X}, zip_name: {}, contents_tree: 0x{:X} }}",
            self.class, self.unknown_u32_1, self.unknown_u32_2, self.unknown_u32_3, zip_name_string, self.contents_tree
        )
    }
}
