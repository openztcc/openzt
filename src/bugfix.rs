
use tracing::info;

use crate::debug_dll::{get_from_memory, save_to_protected_memory};

const ZOOWALL_MAP_EDGE_CRASH_ADDRESS: u32 = 0x050b260;

pub fn init() {
    fix_zoowall_map_edge_crash();
}


fn fix_zoowall_map_edge_crash() {
    // We change a jump address to fix a bug trying access a null pointer
    save_to_protected_memory(ZOOWALL_MAP_EDGE_CRASH_ADDRESS, 0xfffffcfeu32 as i32);
}