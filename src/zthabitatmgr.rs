use nt_time::{FileTime, time::OffsetDateTime};
use tracing::info;
use std::fmt;

use getset::{Getters, Setters};

use crate::{
    command_console::{add_to_command_register, CommandError},
    util::{get_from_memory, ZTArray, ZTBoundedString, ZTString},
    ztworldmgr::{read_zt_world_mgr_from_global, ZTWorldMgr, Direction},
    ztmapview::BFTile,
};

pub const GLOBAL_ZTHABITATMGR_ADDRESS: u32 = 0x0063805c;

/// ZTHabitatMgr struct
#[derive(Debug)]
#[repr(C)]
pub struct ZTHabitatMgr {
    vtable: u32,                     // 0x000
    pad1: [u8; 0x04],                // ----------------------- padding: 4 bytes
    map_size_x: u32,                 // 0x008
    map_size_y: u32,                 // 0x00c
    zoo_entrance_x: u32,             // 0x010
    zoo_entrance_y: u32,             // 0x014
    pad2: [u8; 0x04],                // ----------------------- padding: 4 bytes
    exhibit_array: ZTArray<ZTHabitat>, // 0x01c (0xc bytes)
    other_array_start: u32,        // 0x028 //TODO: Use ZTArray; Seems to be some kind of mapping from BFTile to ZTHabitat or a ZTHabitat index
    other_array_end: u32,          // 0x02c
    other_array_buffer_end: u32,       // 0x030
    pad3: [u8; 0x24],                // ----------------------- padding: 36 bytes
    popularity_scale_factor: f32
}

impl ZTHabitatMgr {
    // fn get_tank(tile: &BFTile) -> Option<ZTHabitat> {
        
    // }

    pub fn get_habitat_by_tile(&self, tile: &BFTile) -> Option<ZTHabitat> {
        self.get_habitat(tile.x, tile.y)
    }

    pub fn get_habitat(&self, pos_x: u32, pos_y: u32) -> Option<ZTHabitat> {
        let base_ptr = self.other_array_start;
        let offset_1 = pos_x * 0xc;
        let intermediate_ptr = get_from_memory::<u32>(base_ptr + offset_1);


        let offset_2 = pos_y * 0x28;
        let ptr = get_from_memory::<u32>(intermediate_ptr + offset_2);

        info!("Habitat ptr: {:#x}", ptr);

        // TODO: Check vtable ptr and return ZTHabitat or ZTTankExhibit?

        if ptr != 0 {
            return Some(get_from_memory::<ZTHabitat>(ptr));
        }

        None
        
    }
}

// int index1 = temp_entity->field_0x34;
// int offset1 = index1 * 0xC;  // 12 bytes per entry
// void* basePointer = GLOBAL_ZTHabitatMgr->mbr_0x28;
// int* intermediatePtr = (int*)(basePointer + offset1);


// int index2 = temp_entity->field_0x38;
// int offset2 = index2 * 0x28;  // 40 bytes per entry
// ZTHabitat** habitatPtrPtr = (ZTHabitat**)(*intermediatePtr + offset2);
// ZTHabitat* this_01 = *habitatPtrPtr;

impl fmt::Display for ZTHabitatMgr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ZTHabitatMgr ({:#x}) {{\n", self.vtable)?;
        write!(f, "  map_size_x: {},\n", self.map_size_x)?;
        write!(f, "  map_size_y: {},\n", self.map_size_y)?;
        write!(f, "  zoo_entrance_x: {},\n", self.zoo_entrance_x)?;
        write!(f, "  zoo_entrance_y: {},\n", self.zoo_entrance_y)?;
        write!(f, "  exhibit_array length: {},\n", self.exhibit_array.len())?;
        write!(f, "  other_array_start: {:#x},\n", self.other_array_start)?;
        write!(f, "  other_array_end: {:#x} ({}),\n", self.other_array_end, (self.other_array_end - self.other_array_start) / 12)?;
        write!(f, "  other_array_buffer_end: {:#x} ({}),\n", self.other_array_buffer_end, (self.other_array_buffer_end - self.other_array_start) /12)?;
        write!(f, "  popularity_scale_factor: {},\n", self.popularity_scale_factor)?;
        write!(f, "}}")
    }
}

pub fn read_zt_habitat_mgr_from_memory() -> ZTHabitatMgr {
    get_from_memory::<ZTHabitatMgr>(get_from_memory(GLOBAL_ZTHABITATMGR_ADDRESS))
}

#[derive(Debug, Getters)]
#[repr(C)]
#[get = "pub"]
pub struct ZTHabitat{
    vtable: u32,                     // 0x000
    zt_show_info_ptr: u32,            // 0x004
    pad1: [u8; 0x38],                // ----------------------- padding: 60 bytes
    exhibit_tile_ptr: u32,          // 0x040 // Seems incorrect?
    pad2: [u8; 0x48],                // ----------------------- padding: 72 bytes
    entrance_tile_ptr: u32,          // 0x08c
    entrance_rotation: u32,          // 0x090
    pad3: [u8; 0x58],                // ----------------------- padding: 88 bytes
    unknown_u32: u32,                // 0x0ec
    pad4: [u8; 0xc],                // ----------------------- padding: 12 bytes
    current_donactions: f32,       // 0xfc
    last_donactions: f32,          // 0x100
    total_donactions: f32,         // 0x104
    current_upkeep: f32,          // 0x108
    last_upkeep: f32,             // 0x10c
    total_upkeep: f32,            // 0x110
    unknown_u32_2: u32,            // 0x114
    unknown_u32_3: u32,            // 0x118
    unknown_u32_4: u32,            // 0x11c
    created_timestamp: FileTime,        // 0x120
    unknown_nt_time: FileTime,          // 0x128
    pad5: [u8; 0x24],                // ----------------------- padding: 36 bytes
    exhibit_name: ZTBoundedString
}

impl ZTHabitat {
    pub fn get_gate_tile_in(&self) -> Option<BFTile> {
        if self.entrance_tile_ptr == 0 {
            return None;
        }
        // info!("ZTHabitat: {}", self);
        // info!("Entrance tile ptr: {:#x}", self.entrance_tile_ptr);
        let tile = get_from_memory::<BFTile>(self.entrance_tile_ptr);
        // info!("Entrance tile: {}", tile);

        let zthm = read_zt_habitat_mgr_from_memory();
        if let Some(gate_habitat) = zthm.get_habitat_by_tile(&tile) && gate_habitat == *self {
            return Some(tile);
        }
        let ztwm = read_zt_world_mgr_from_global();
        ztwm.get_neighbour(&tile, Direction::from(self.entrance_rotation))
    }
}

impl PartialEq for ZTHabitat {
    fn eq(&self, other: &Self) -> bool {
        self.exhibit_tile_ptr == other.exhibit_tile_ptr &&
        self.entrance_rotation == other.entrance_rotation &&
        self.entrance_tile_ptr == other.entrance_tile_ptr &&
        self.exhibit_name.copy_to_string() == other.exhibit_name.copy_to_string()
    }
}

impl fmt::Display for ZTHabitat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ZTHabitat {{\n",)?;
        write!(f, "  vtable: {:#x},\n", self.vtable)?;
        write!(f, "  zt_show_info_ptr: {:#x},\n", self.zt_show_info_ptr)?;
        write!(f, "  exhibit_tile_ptr: {:#x},\n", self.exhibit_tile_ptr)?;
        write!(f, "  entrance_tile_ptr: {:#x},\n", self.entrance_tile_ptr)?;
        write!(f, "  entrance_rotation: {:#x},\n", self.entrance_rotation)?;
        write!(f, "  unknown_u32: {:#x},\n", self.unknown_u32)?;
        write!(f, "  current_donactions: {},\n", self.current_donactions)?;
        write!(f, "  last_donactions: {},\n", self.last_donactions)?;
        write!(f, "  total_donactions: {},\n", self.total_donactions)?;
        write!(f, "  current_upkeep: {},\n", self.current_upkeep)?;
        write!(f, "  last_upkeep: {},\n", self.last_upkeep)?;
        write!(f, "  total_upkeep: {},\n", self.total_upkeep)?;
        write!(f, "  unknown_u32_2: {:#x},\n", self.unknown_u32_2)?;
        write!(f, "  unknown_u32_3: {:#x},\n", self.unknown_u32_3)?;
        write!(f, "  unknown_u32_4: {:#x},\n", self.unknown_u32_4)?;
        write!(f, "  created_timestamp: {},\n", OffsetDateTime::try_from(self.created_timestamp).unwrap())?;
        write!(f, "  unknown_nt_time: {} ({}, {}, {}),\n", OffsetDateTime::try_from(self.unknown_nt_time).unwrap(), self.unknown_nt_time.to_raw() as f64, self.unknown_nt_time.to_raw() as u32, (self.unknown_nt_time.to_raw() >> 32) as u32)?;
        write!(f, "  exhibit_name: {},\n", self.exhibit_name.copy_to_string())?;






        // write!(f, "  entrance_x: {},\n", self.entrance_x)?;
        // write!(f, "  entrance_y: {},\n", self.entrance_y)?;
        // write!(f, "  entrance_rotation: {},\n", self.entrance_rotation)?;
        // write!(f, "  unknown_ptr: {:#x},\n", self.unknown_ptr)?;
        // write!(f, "  unknown_ptr2: {:#x},\n", self.unknown_ptr2)?;
        // write!(f, "  unknown_ptr3: {:#x},\n", self.unknown_ptr3)?;
        // write!(f, "  current_donations: {},\n", self.current_donations)?;
        // write!(f, "  last_donations: {},\n", self.last_donations)?;
        // write!(f, "  total_donations: {},\n", self.total_donations)?;
        // write!(f, "  current_upkeep: {},\n", self.current_upkeep)?;
        // write!(f, "  last_upkeep: {},\n", self.last_upkeep)?;
        // write!(f, "  total_upkeep: {},\n", self.total_upkeep)?;
        // write!(f, " unknown_ptr4: {:#x},\n", self.unknown_ptr4)?;
        // write!(f, " unknown_ptr5: {:#x},\n", self.unknown_ptr5)?;
        // write!(f, " unknown_ptr6: {:#x},\n", self.unknown_ptr6)?;
        // write!(f, " created_timestamp: {:#x},\n", self.created_timestamp)?;
        write!(f, "}}")
    }
}

fn command_get_zt_habitat_mgr(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_habitat_mgr = read_zt_habitat_mgr_from_memory();
    Ok(format!("{}", zt_habitat_mgr))
}

fn command_get_zt_habitats(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_habitat_mgr = read_zt_habitat_mgr_from_memory();
    let mut result_string = String::new();
    for i in 0..zt_habitat_mgr.exhibit_array.len() {
        let habitat = zt_habitat_mgr.exhibit_array.get(i);
        let habitat_location = zt_habitat_mgr.exhibit_array.get_ptr(i);
        let popularity_scale_factor = zt_habitat_mgr.popularity_scale_factor;
        result_string.push_str(&format!("Habitat {} ({:#x}): ", i, habitat_location));
        result_string.push_str(&format!("  exhibit_popularity?: {}, {}, {}),\n", (habitat.unknown_nt_time.to_raw() as f64)/popularity_scale_factor as f64, (habitat.unknown_nt_time.to_raw() as f32) / popularity_scale_factor, ((habitat.unknown_nt_time.to_raw() >> 32) as f32)/popularity_scale_factor));
        result_string.push_str(&format!("{}\n", habitat));
    }
    // zt_habitat_mgr.exhibit_array.get_vec().iter().enumerate().for_each(|(i, habitat)| {
    //     result_string.push_str(&format!("Habitat {}: ", i));
    //     result_string.push_str(&format!("{}\n", habitat));
    // });
    Ok(result_string)
}

#[hook_module("zoo.exe")]
pub mod hooks_zthabitatmgr {
    use super::*;

    // 00410349 BFTile * __thiscall OOAnalyzer::ZTHabitat::getGateTileIn(ZTHabitat *this)
    #[hook(unsafe extern "thiscall" ZTHabitat_get_gate_tile_in, offset = 0x00010349)]
    fn get_gate_tile_in(_this: u32) -> u32 {
        let habitat = get_from_memory::<ZTHabitat>(_this);
        match habitat.get_gate_tile_in() {
            Some(tile) => {
                read_zt_world_mgr_from_global().get_ptr_from_bftile(&tile)
            }
            None => {
                0
            }
        }
    }

    
}

pub fn init() {
    add_to_command_register("get_zthabitatmgr".to_owned(), command_get_zt_habitat_mgr);
    add_to_command_register("list_exhibits".to_string(), command_get_zt_habitats);
}