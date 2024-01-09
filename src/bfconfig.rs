use crate::debug_dll::{get_from_memory, get_string_from_memory, patch_calls};
use crate::add_to_command_register;

// use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::fmt;
use std::io::BufWriter;

use tracing::info;

use retour_utils::hook_module;

use ptree::{TreeBuilder, write_tree};

const BF_CONFIG_STRING_TABLE_START: u32 = 0x6380e4;

#[repr(C)]
struct BFConfigStringTable {
    tree_ptr: u32,
    size: u32,
}

#[repr(C)]
struct Tree {
    colour: u8,
    root_ptr: u32,
    leftmost_ptr: u32,
    rightmost_ptr: u32,
}

#[repr(C)]
struct RedBlackNode {
    colour: u8,
    parent_ptr: u32,
    left_ptr: u32,
    right_ptr: u32,
    compact_string: CompactString,
    not_null: u8,
}

pub struct CompactString {
    start_ptr: u32,
    end_ptr: u32,
    end_buffer_ptr: u32,
}

impl fmt::Display for BFConfigStringTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BFConfigStringTable {{ tree_ptr: {:#08x}, size: {} }}\n{}", self.tree_ptr, self.size, get_from_memory::<Tree>(self.tree_ptr).to_string())
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tree {{ colour: {}, root_ptr: {:#08x}, leftmost_ptr: {:#08x}, rightmost_ptr: {:#08x} }}\n", self.colour, self.root_ptr, self.leftmost_ptr, self.rightmost_ptr)
    }
}

impl fmt::Display for CompactString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X} {:X} {:X} \"{}\"", self.start_ptr, self.end_ptr, self.end_buffer_ptr, get_string_from_memory(self.start_ptr))
    }
}

fn read_bf_config_string_table() {
    let bf_config_string_table = get_from_memory::<BFConfigStringTable>(get_from_memory::<u32>(BF_CONFIG_STRING_TABLE_START));
    info!("{}", bf_config_string_table.to_string());
}

fn log_bfconfig_string_table() {
    let bf_config_string_table = get_from_memory::<BFConfigStringTable>(get_from_memory::<u32>(BF_CONFIG_STRING_TABLE_START));
    let tree = get_from_memory::<Tree>(bf_config_string_table.tree_ptr);
    let root_node = get_from_memory::<RedBlackNode>(tree.root_ptr);
    log_node(root_node)
}

fn log_node(node: RedBlackNode) {
    if node.left_ptr != 0 {
        log_node(get_from_memory::<RedBlackNode>(node.left_ptr));
    }
    info!("{}\n", node.compact_string.to_string());
    if node.right_ptr != 0 {
        log_node(get_from_memory::<RedBlackNode>(node.right_ptr));
    }
}

pub fn create_ptree() {
    let bf_config_string_table = get_from_memory::<BFConfigStringTable>(get_from_memory::<u32>(BF_CONFIG_STRING_TABLE_START));
    let tree = get_from_memory::<Tree>(bf_config_string_table.tree_ptr);
    let root_node = get_from_memory::<RedBlackNode>(tree.root_ptr);
    let mut tb = TreeBuilder::new("root".to_string());
    let tree = add_node_to_ptree(&mut tb, root_node).build();
    // print_tree(&tree);
    let mut buf = BufWriter::new(Vec::new());
    write_tree(&tree, &mut buf).unwrap();
    let bytes = buf.into_inner().unwrap();
    let string = String::from_utf8(bytes).unwrap();
    info!("{}", string);
}

fn add_node_to_ptree(tb: &mut TreeBuilder, node: RedBlackNode) -> &mut TreeBuilder {
    tb.begin_child(node.compact_string.to_string());
    if node.left_ptr != 0 {
        add_node_to_ptree(tb, get_from_memory::<RedBlackNode>(node.left_ptr));
    }
    if node.right_ptr != 0 {
        add_node_to_ptree(tb, get_from_memory::<RedBlackNode>(node.right_ptr));
    }
    tb.end_child()
}



#[hook_module("zoo.exe")]
mod zoo_bfconfig {
    use std::cmp;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    use tracing::info;

    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    // use super::read_bf_config_string_table;
    // use super::log_bfconfig_string_table;
    // use super::create_ptree;
    use super::CompactString;

    // static LOGGING_FLAG: AtomicBool = AtomicBool::new(true);

    static LOGGING_COUNTER: AtomicU32 = AtomicU32::new(0);

    static LOGGING_MAX: u32 = 20;

    fn should_log() -> bool {
        LOGGING_COUNTER.fetch_add(1, Ordering::Relaxed) < LOGGING_MAX
    }

    #[hook(unsafe extern "thiscall" basic_string_con1, offset = 0x00001cab)]
    fn zoo_basic_string_con1(_this_ptr: u32, param_1: u32, param_2: u32, param_3: u8) {
        if should_log() {
            let string_1 = get_string_from_memory(param_1);
            let string_2 = get_string_from_memory(param_2);
            info!("basic_string_con1({:X}->{:X}, {:X}->{}, {:X}->{}, {:X})", _this_ptr, get_from_memory::<u32>(_this_ptr), param_1, string_1, param_2, string_2, param_3);
        }
        unsafe { basic_string_con1.call(_this_ptr, param_1, param_2, param_3)};
        let compact_string = get_from_memory::<CompactString>(_this_ptr);
        if should_log() {
            info!("basic_string_con1 {}", compact_string.to_string());
        }
    }

    #[hook(unsafe extern "stdcall" basic_string_des1, offset = 0x00001a2f)]
    fn zoo_basic_string_des1(param_1: u32, param_2: u32) {
        if should_log() {
        //     let compact_string = get_from_memory::<CompactString>(_this_ptr);
            info!("basic_string_des1({:X}; {:X})", param_1, param_2);
        }
        unsafe { basic_string_des1.call(param_1, param_2)};
    }


    // #[hook(unsafe extern "cdecl" ZTBFConfig_addString, offset = 0x0000ae55)]
    // fn zoo_zt_bfconfig_add_string(param_1: u32, param_2: u32) -> u32 {
    //     let param_1_string = get_string_from_memory(param_1);
    //     let snippet = &param_1_string[..cmp::min(param_1_string.len(), 10)];
    //     // let param_2_ptr = get_from_memory::<u32>(param_2);
    //     // info!("ZTBFConfig_add_string: {:X}->{:?} {:X}->{}", param_1, param_1_string, param_2, param_2_ptr);
    //     info!("ZTBFConfig_add_string: {:X}->{} {:X}", param_1, snippet, param_2);
    //     unsafe { ZTBFConfig_addString.call(param_1, param_2)}
    // }

    // #[hook(unsafe extern "cdecl" ZTBFConfig_delString, offset = 0x0000a6ad)]
    // fn zoo_zt_bfconfig_del_string(param_1: u32) {
    //     let param_1_string = &get_string_from_memory(param_1);
    //     let snippet = &param_1_string[..cmp::min(param_1_string.len(), 10)];
    //     // let param_2_ptr = get_from_memory::<u32>(param_2);
    //     // info!("ZTBFConfig_add_string: {:X}->{:?} {:X}->{}", param_1, param_1_string, param_2, param_2_ptr);
    //     info!("ZTBFConfig_del_string: {:X}->{}", param_1, snippet);
    //     unsafe { ZTBFConfig_delString.call(param_1)};
    // }

    // fn logif(message: String) {
    //     if LOGGING_FLAG.load(Ordering::Relaxed) {
    //         // info!("{}", message);
    //     }
    // }

    // #[hook(unsafe extern "thiscall" ZTBFConfig_parse, offset = 0x0000ade7)]
    // fn zoo_zt_config_parse(_this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
    //     logif(format!("ZTBFConfig_parse({:X}, {:X}, {})", _this_ptr, param_1, param_2));
    //     let result = unsafe { ZTBFConfig_parse.call(_this_ptr, param_1, param_2)};
    //     logif(format!("Return from ZTBFConfig_parse {}", result));
    //     // if LOGGING_FLAG.load(Ordering::Relaxed) {
    //     if should_log() {
    //         // read_bf_config_string_table();
    //         create_ptree();
    //         log_bfconfig_string_table();
    //     }
    //     // create_ptree();
    //     LOGGING_FLAG.store(false, Ordering::Relaxed);
    //     result
    // }

    // #[hook(unsafe extern "thiscall" ZTBFConfig_addBlock, offset = 0x0000b540)]
    // fn zoo_zt_config_add_block(_this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) -> u32 {
    //     logif(format!("ZTBFConfig_addBlock({:X}, {:X}, {}, {})", _this_ptr, param_1, param_2, param_3));
    //     let result = unsafe { ZTBFConfig_addBlock.call(_this_ptr, param_1, param_2, param_3)};
    //     logif(format!("Return from ZTBFConfig_addBlock {}", result));
    //     result
    // }

    // #[hook(unsafe extern "thiscall" ZTBFConfig_addKeyVal, offset = 0x0000af4f)]
    // fn zoo_zt_config_add_key_val(_this_ptr: u32, param_1: u32, param_2: u32, param_3: u32, param_4: u32) {
    //     logif(format!("ZTBFConfig_addKeyVal({:X}, {:X}, {:X}, {:X}, {:X})", _this_ptr, param_1, param_2, param_3, param_4));
    //     unsafe { ZTBFConfig_addKeyVal.call(_this_ptr, param_1, param_2, param_3, param_4)};
    //     logif(format!("Return from ZTBFConfig_addKeyVal"));
    // }
}

pub fn init() {
    unsafe { zoo_bfconfig::init_detours().unwrap() };
}