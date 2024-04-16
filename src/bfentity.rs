use std::ops::Deref;

use getset::{Getters, Setters};
use tracing::info;
use num_enum::FromPrimitive;
use std::fmt;

use crate::{bfentitytype, console::{add_to_command_register, CommandError}, debug_dll::{get_from_memory, get_string_from_memory, map_from_memory}, ztui::get_selected_entity_type_address, ztworldmgr};
use crate::expansions::is_member;

// --------------- Helper Structs and Enums --------------- //

struct Position {
    x: i32,
    y: i32,
}

// --------------- BFEntity, Implementation, and Related Functions --------------- //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct BFEntity {
    pad00: [u8; 0x0B4 - 0x000], // ----------------------- padding: 180 bytes
    position: Position, // 0x0B4 and 0x0B8
    pad01: [u8; 0x0C - 0x0BC], // ------------------------ padding: 4 bytes
    view: i32, // 0x0C0
    pad02: [u8; 0x108 - 0x0C4], // ----------------------- padding: 76 bytes
    name: String, // 0x108
    pad03: [u8; 0x114 - 0x10C], // ----------------------- padding: 8 bytes
    camera_position: Position, // 0x114 and 0x118
    pad04: [u8; 0x128 - 0x11C], // ----------------------- padding: 12 bytes
    entity_type: u32, // 0x128
    pad04: [u8; 0x13F - 0x12C], // ----------------------- padding: 20 bytes
    is_visible: bool, // 0x13F
    pad05: [u8; 0x143 - 0x140], // ----------------------- padding: 4 bytes
    is_grabbed: bool, // 0x143
    pad06: [u8; 0x144 - 0x144], // ----------------------- padding: 1 byte
    is_selected: bool, // 0x145
}

// --------------- ZTBuildingType, Implementation, and Related Functions --------------- //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTBuildingType {
    bfentity: BFEntity, // 0x145 - 0x0 = 0x145  = 325 bytes
    pad00: [u8; 0x1A0 - 0x146], // ----------------------- padding: 90 bytes
    total_income: i32, // 0x1A0
    pad01: [u8; 0x1AC - 0x1A4], // ----------------------- padding: 8 bytes
    item_sale_price: i32, // 0x1AC
    pad02: [u8; 0x1B0 - 0x1A8], // ----------------------- padding: 4 bytes
    total_upkeep: i32, // 0x1B0
}