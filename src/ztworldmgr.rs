use std::cmp::max;
use std::{collections::HashMap, fmt};
use std::str::FromStr;
use getset::Getters;
use itertools::Itertools;
use num_enum::FromPrimitive;
use tracing::{error, info};
use retour_utils::hook_module;

use crate::{
    bfentitytype::{read_zt_entity_type_from_memory, BFEntityType, ZTEntityType, ZTSceneryType},
    command_console::{add_to_command_register, CommandError},
    util::{get_from_memory, get_string_from_memory, map_from_memory},
};
use crate::util::{ZTArray, ZTBoundedString, ZTString, ZTStringPtr, ZTBufferString};
use crate::ztmapview::BFTile;
use crate::bfentitytype::ZTEntityTypeClass;

const GLOBAL_ZTWORLDMGR_ADDRESS: u32 = 0x00638040;

#[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
#[repr(u32)]
pub enum ZTEntityClass {
    Food = 0x62dd08,
    Path = 0x62da88,
    Fences = 0x62d808,
    Building = 0x62e0b0,
    Animal = 0x62ff54,
    Guest = 0x630f88,
    Scenery = 0x62d950,
    Keeper = 0x62f3e4,
    MaintenanceWorker = 0x62ea54,
    TourGuide = 0x62f714,
    Drt = 0x62f0b4,
    Ambient = 0x62d6ec,
    Rubble = 0x62df78,
    TankWall = 0x62dbc0,
    TankFilter = 0x62de40,
    #[num_enum(default)]
    Unknown = 0x0,
}

// TODO: Make this look like other structs with proper offsets and padding -> 
#[derive(Debug, Getters)]
#[get = "pub"]
#[repr(C)]
pub struct ZTEntity {
    vtable: u32,
    // Technically, the first 0x154 bytes are BFEntity, should grab what Eric started doing in his PR and embed BFEntity here
    class: ZTEntityClass,
    type_class: ZTEntityType, // TODO: Change to &ZTEntityType at some point?
    name: String,
    pos1: u32,
    pos2: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Footprint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl fmt::Display for Footprint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Footprint {{ x: {}, y: {}, z: {} }}", self.x, self.y, self.z)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rectangle {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

// zt_type: get_string_from_memory(get_from_memory::<u32>(zt_entity_type_ptr + 0x98)),
// zt_sub_type: get_string_from_memory(get_from_memory::<u32>(zt_entity_type_ptr + 0xa4)),
// bf_config_file_ptr: get_from_memory::<u32>(zt_entity_type_ptr + 0x80),

#[derive(Debug, Getters)]
#[get = "pub"]
#[repr(C)]
pub struct BFEntity {
    vtable: u32,
    padding: [u8; 0x104],
    name: ZTBufferString,   // 0x108
    x_coord: u32,           // 0x114
    y_coord: u32,           // 0x118   
    z_coord: u32,           // 0x11c
    height_above_terrain: u32, // 0x120
    padding4: [u8; 0x4],    // ----- padding: 4 bytes
    inner_class_ptr: u32,   // 0x128
    rotation: i32,          // 0x12c
    padding5: [u8; 0x14],    // ----- padding: 28 bytes
    unknown_flag1: u8,    // 0x13c // isRemoved
    unknown_flag2: u8,    // 0x13d // isRemovedUndo
    unknown_flag3: u8,    // 0x13e
    visible: u8,    // 0x13f 
    snap_to_ground: u8, // 0x140
    selected: u8, // 0x141
    unknown_flag4: u8, // 0x142 // Moving? Programmatically?
    unknown_flag5: u8, // 0x143 // Picked up?
    draw_dithered: u8, // 0x144
    unknown_flag6: u8, // 0x145 // If != 0; Draw selection graphic
    stop_at_end: u8, // 0x146
}

impl fmt::Display for BFEntity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "BFEntity {{ name: {}, x_coord: {}, y_coord: {}, z_coord: {}, height_above_terrain: {}, rotation: {}, inner_class_ptr: {:#x}, visible: {}, snap_to_ground: {}, selected: {}, draw_dithered: {} }}",
            self.name, self.x_coord, self.y_coord, self.z_coord, self.height_above_terrain, self.rotation, self.inner_class_ptr, self.visible, self.snap_to_ground, self.selected, self.draw_dithered
        )
    }
}

impl BFEntity {
    pub fn entity_type_class(&self) -> ZTEntityTypeClass {
        ZTEntityTypeClass::from(self.inner_class_ptr)
    }

    pub fn entity_type(&self) -> BFEntityType {
        get_from_memory(self.inner_class_ptr)
    }

    pub fn is_on_tile(&self, tile: &BFTile) -> bool {
        // BFEntity::getBlockRect
        // BFEntity::getFootprint
        false
    }


    pub fn get_blocking_rect(&self) -> Rectangle {
        // Transient entities don't block anything
        if self.inner_class_ptr == 0 || self.entity_type().is_transient {
            return Rectangle::default(); // Zero rectangle
        }
        
        let mut footprint = self.vtable_get_footprint();
        
        if self.rotation % 2 != 0 {
            info!("SWAP");
            // let temp = footprint.x;
            // footprint.x = footprint.y;
            // footprint.y = temp;
            // Try this?
            let max = max(footprint.x, footprint.y);
            footprint.x = max;
            footprint.y = max;
        }
        // if self.rotation % 180 != 0 {
        //     // Swap width and height for 90° or 270° rotations
        //     std::mem::swap(&mut footprint.width, &mut footprint.height);
        // }
        
        // Calculate half-dimensions for easier rectangle construction
        let half_width = (footprint.x * 32) / 2;  // Scaling factor preserved from original
        let half_height = (footprint.y * 32) / 2;
        
        // Construct and return the rectangle
        Rectangle {
            min_x: self.x_coord as i32 - half_width,
            min_y: self.y_coord as i32 - half_height,
            max_x: self.x_coord as i32 + half_width,
            max_y: self.y_coord as i32 + half_height,
        }
    }

    fn vtable_get_footprint(&self) -> Footprint {
        info!("Vtable: {:#x} -> {:p}", get_from_memory::<u32>(self.vtable + 0x94), self);
        // let unsafe_copy = get_from_memory::<BFEntity>(self as *const BFEntity as u32);
        // info!("Unsafe copy: {} vs {}", unsafe_copy.inner_class_ptr, self.inner_class_ptr);
        // return Footprint::default()
        let function_address = get_from_memory::<u32>(self.vtable + 0x94);
        info!("Get Footprint Function address: {:#x}", function_address);
        let get_footprint_fn = unsafe { std::mem::transmute::<u32, extern "thiscall" fn(this: &BFEntity, param_1: &Footprint, param_2: u32) -> u32>(function_address) };
        let mut result_footprint = Footprint::default();
        // let mut result_footprint = [0_u32; 3];
        // info!("Calling get_footprint_fn with self: {:p}, get_footprint_fn: {:?}", get_footprint_fn, result_footprint);
        let footprint_ptr = get_footprint_fn(self, &result_footprint, 0_u32);
        info!("Footprint pointer: {:#x}", footprint_ptr);
        // let footprint_ptr = get_footprint_fn(self as *const BFEntity as u32, &result_footprint, 0);
        // let footprint_ptr = get_footprint_fn(helper);
        get_from_memory::<Footprint>(footprint_ptr)
        // return Footprint::default()
    }

    pub fn get_footprint(&self) -> Footprint {
        let entity_type = self.entity_type();
        if self.rotation % 4 == 0 {
            return Footprint {
                x: entity_type.footprintx,
                y: entity_type.footprinty,
                z: entity_type.footprintz,
            };
        } else {
            return Footprint {
                x: entity_type.footprinty,
                y: entity_type.footprintx,
                z: entity_type.footprintz,
            };
        }
    }
}

// ZTAnimal -> 0x3a6 = animalDying

// let inner_class_ptr = get_from_memory::<u32>(zt_entity_ptr + 0x128);
// 
// ZTEntity {
// class: ZTEntityClass::from(get_from_memory::<u32>(zt_entity_ptr)),
// type_class: read_zt_entity_type_from_memory(inner_class_ptr),
// name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
// pos1: get_from_memory::<u32>(zt_entity_ptr + 0x114),
// pos2: get_from_memory::<u32>(zt_entity_ptr + 0x118),
// }


impl ZTEntity {
    pub fn is_member(&self, member: String) -> bool {
        self.type_class.is_member(member)
    }
}

impl fmt::Display for ZTEntity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Entity Type: {:?}, Name: {}, EntityType {} ({},{}) ({},{})",
            self.class, self.name, self.type_class, self.pos1, self.pos2, self.pos1 >> 6, self.pos2 >> 6
        )
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ZTWorldMgr {
    padding_1: [u8; 0x34],
    map_x_size: u32,
    map_y_size: u32,
    padding_2: [u8; 0x4],
    tile_array: u32,
    padding_3: [u8; 0x3c],
    entity_array_start: u32,
    entity_array_end: u32,
    entity_array_buffer_end: u32,
    padding_4: [u8; 0xc],
    entity_type_array_start: u32,
    entity_type_array_end: u32,
    entity_type_array_buffer_end: u32,
}

impl fmt::Display for ZTWorldMgr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ZTWorldMgr {{ map_x_size: {}, map_y_size: {}, tile_array: {:#x}, entity_array_start: {:#x}, entity_array_end: {:#x}, entity_type_array_start: {:#x}, entity_type_array_end: {:#x} }}",
            self.map_x_size,
            self.map_y_size,
            self.tile_array,
            self.entity_array_start,
            self.entity_array_end,
            self.entity_type_array_start,
            self.entity_type_array_end,
        )
    }
}

// TODO: Move to util or better named crate
#[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
#[repr(u32)]
pub enum Direction {
    #[default]
    West = 0,
    NorthWest = 1,
    North = 2,
    NorthEast = 3,
    East = 4,
    SouthEast = 5,
    South = 6,
    SouthWest = 7,
}

impl ZTWorldMgr {
    pub fn get_neighbour(&self, bftile: &BFTile, direction: Direction) -> Option<BFTile> {
        let x_offset: i32 = match direction {
            Direction::West => 0,
            Direction::NorthWest => 1,
            Direction::North => 1,
            Direction::NorthEast => 1,
            Direction::East => 0,
            Direction::SouthEast => -1,
            Direction::South => -1,
            Direction::SouthWest => -1,
        };
        let y_offset: i32 = match direction {
            Direction::West => -1,
            Direction::NorthWest => -1,
            Direction::North => 0,
            Direction::NorthEast => 1,
            Direction::East => 1,
            Direction::SouthEast => 1,
            Direction::South => 0,
            Direction::SouthWest => -1,
        };

        let x: i32 = bftile.x as i32 + x_offset;
        let y: i32 = bftile.y as i32 + y_offset;

        if x < 0 || x >= self.map_x_size as i32 || y < 0 || y >= self.map_y_size as i32 {
            return None;
        }

        Some(get_from_memory::<BFTile>(self.tile_array + (((y as u32 * self.map_x_size) + x as u32) * 0x8c_u32)))
    }

    pub fn get_ptr_from_bftile(&self, bftile: &BFTile) -> u32 {
        let x = bftile.x;
        let y = bftile.y;
        self.tile_array + ((y * self.map_x_size + x) * 0x8c)
    }
}

#[hook_module("zoo.exe")]
pub mod hooks_ztworldmgr {
    use crate::util::save_to_memory;

    use super::*;

    #[hook(unsafe extern "thiscall" BFMap_get_neighbour, offset = 0x00032236)]
    fn bfmap_get_neighbour(_this: u32, bftile: u32, direction: u32) -> u32 {
        // info!("BFMap::getNeighbor called with params: {:#x}, {:#x}, {:?}", _this, bftile, direction);
        // let result = unsafe { BFMap_get_neighbour.call(_this, bftile, direction) };
        let ztwm = read_zt_world_mgr_from_global();
        let bftile = get_from_memory::<BFTile>(bftile);
        let direction = Direction::from(direction);
        match ztwm.get_neighbour(&bftile, direction) {
            Some(neighbour) => {
                ztwm.get_ptr_from_bftile(&neighbour)
            }
            None => {
                0
            }
        }
    }    

    // // 0x0040f916 int * __thiscall OOAnalyzer::BFEntity::getFootprint(BFEntity *this,undefined4 *param_1)
    // #[hook(unsafe extern "thiscall" BFEntity_get_footprint, offset = 0x0000f916)]
    // fn bfentity_get_footprint(_this: u32, param_1: u32, _param_2: u32) -> u32 {
    //     let entity = get_from_memory::<BFEntity>(_this);
    //     let footprint = entity.get_footprint();
    //     save_to_memory(param_1, footprint.x);
    //     save_to_memory(param_1 + 0x4, footprint.y);
    //     save_to_memory(param_1 + 0x8, footprint.z);

    //     param_1
    // }

    // 0x0042721a u32 __thiscall OOAnalyzer::BFEntity::getBlockingRect(BFEntity *this,u32 param_1,int param_2)
    #[hook(unsafe extern "thiscall" BFEntity_get_blocking_rect, offset = 0x0002721a)]
    fn bfentity_get_blocking_rect(_this: u32, param_1: u32) -> u32 {
        info!("Actual: {:#x} -> {:#x}", _this, get_from_memory::<u32>(_this));
        let entity_ptr = get_from_memory::<u32>(_this);
        let result = unsafe {
            BFEntity_get_blocking_rect.call(_this, param_1)
        };
        if entity_ptr < 0x400000 {
            if 0 != get_from_memory(result) || 0 != get_from_memory(result + 0x4)
                || 0 != get_from_memory(result + 0x8) || 0 != get_from_memory(result + 0xc) {
                error!("BFEntity::getBlockingRect returned unexpected rectangle: {:#?}", result);
                return result;
            }
            save_to_memory(param_1, 0);
            save_to_memory(param_1 + 0x4, 0);
            save_to_memory(param_1 + 0x8, 0);
            save_to_memory(param_1 + 0xc, 0);
            param_1
        } else {
            let result = unsafe {
                BFEntity_get_blocking_rect.call(_this, param_1)
            };
            let entity = get_from_memory::<BFEntity>(_this);
            // info!("BFEntity {}", entity);
            let rect = entity.get_blocking_rect();
            // info!("Reimplemented {:?}", rect);
            // save_to_memory(param_1, rect.min_x);
            // save_to_memory(param_1 + 0x4, rect.min_y);
            // save_to_memory(param_1 + 0x8, rect.max_x);
            // save_to_memory(param_1 + 0xc, rect.max_y);
            // param
            
            let result_rect = get_from_memory::<Rectangle>(result);
            if result_rect.max_x != rect.max_x
                || result_rect.max_y != rect.max_y
                || result_rect.min_x != rect.min_x
                || result_rect.min_y != rect.min_y {
                error!("BFEntity::getBlockingRect returned unexpected rectangle: {:#?} instead of {:#?}", result_rect, rect);
            }
            result
            // get_from_memory::<u32>(_this)
        }

        // }
        // let rect = entity.get_blocking_rect();
        // info!("Reimplemented {:?}", rect);
        
        // let result_rect = get_from_memory::<Rectangle>(result);
        // if result_rect.max_x != rect.max_x
        //     || result_rect.max_y != rect.max_y
        //     || result_rect.min_x != rect.min_x
        //     || result_rect.min_y != rect.min_y {
        //     error!("BFEntity::getBlockingRect returned unexpected rectangle: {:#?} instead of {:#?}", result_rect, rect);
        // }
        // save_to_memory(param_1, rect.min_x);
        // save_to_memory(param_1 + 0x4, rect.min_y);
        // save_to_memory(param_1 + 0x8, rect.max_x);
        // save_to_memory(param_1 + 0xc, rect.max_y);

        // param_1
        // result
    }
}

pub fn init() {
    add_to_command_register("list_entities".to_owned(), command_get_zt_world_mgr_entities);
    add_to_command_register("list_entities_2".to_owned(), command_get_zt_world_mgr_entities_2);
    add_to_command_register("list_types".to_owned(), command_get_zt_world_mgr_types);
    add_to_command_register("get_zt_world_mgr".to_owned(), command_get_zt_world_mgr);
    add_to_command_register("get_types_summary".to_owned(), command_zt_world_mgr_types_summary);
    add_to_command_register("get_entity_vtable_entry".to_owned(), command_get_entity_unique_vtable_entries);
    unsafe { hooks_ztworldmgr::init_detours().unwrap() };
}

pub fn read_zt_entity_from_memory(zt_entity_ptr: u32) -> ZTEntity {
    let inner_class_ptr = get_from_memory::<u32>(zt_entity_ptr + 0x128);

    ZTEntity {
        vtable: get_from_memory::<u32>(zt_entity_ptr),
        class: ZTEntityClass::from(get_from_memory::<u32>(zt_entity_ptr)),
        type_class: read_zt_entity_type_from_memory(inner_class_ptr),
        name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
        pos1: get_from_memory::<u32>(zt_entity_ptr + 0x114),
        pos2: get_from_memory::<u32>(zt_entity_ptr + 0x118),
    }
}

pub fn read_zt_world_mgr_from_global() -> ZTWorldMgr {
    let zt_world_mgr_ptr = get_from_memory::<u32>(GLOBAL_ZTWORLDMGR_ADDRESS);
    get_from_memory::<ZTWorldMgr>(zt_world_mgr_ptr)
}

fn log_zt_world_mgr(zt_world_mgr: &ZTWorldMgr) {
    info!("zt_world_mgr: {:#?}", zt_world_mgr);
}

fn command_get_zt_world_mgr_entities(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entities = get_zt_world_mgr_entities(&zt_world_mgr);
    info!("Found {} entities", entities.len());
    if entities.is_empty() {
        return Ok("No entities found".to_string());
    }
    let mut string_array = Vec::new();
    for entity in entities {
        string_array.push(entity.to_string());
    }
    Ok(string_array.join("\n"))
}

fn command_get_zt_world_mgr_entities_2(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entities = get_zt_world_mgr_entities_2(&zt_world_mgr);
    info!("Found {} entities", entities.len());
    if entities.is_empty() {
        return Ok("No entities found".to_string());
    }
    let mut string_array = Vec::new();
    for entity in entities {
        string_array.push(entity.to_string());
    }
    Ok(string_array.join("\n"))
}

fn command_get_entity_unique_vtable_entries(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() != 1 {
        return Err(CommandError::new("Vtable offset required".to_string()));
    }
    
    let vtable_offset = match args[0].strip_prefix("0x") {
        Some(hex_str) => {
            u32::from_str_radix(hex_str, 16)?
        }
        None => {
            u32::from_str(args[0])?
        }
    };
    
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entities = get_zt_world_mgr_entities(&zt_world_mgr);

    let mut result = String::new();
    
    entities
        .iter()
        .map(|entity| (entity.type_class.class.clone(), entity.vtable + vtable_offset))
        .unique_by(|t| t.1)
        .for_each(|(type_name, vfunc_ptr)| {
            result.push_str(&format!("{:?} -> {:#x}\n", type_name, get_from_memory::<u32>(vfunc_ptr)));
        });

    Ok(result)
}

fn command_get_zt_world_mgr_types(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let types = get_zt_world_mgr_types(&zt_world_mgr);
    info!("Found {} types", types.len());
    if types.is_empty() {
        return Ok("No types found".to_string());
    }
    let mut string_array = Vec::new();
    for zt_type in types {
        string_array.push(zt_type.to_string());
    }
    Ok(string_array.join("\n"))
}

fn command_get_zt_world_mgr(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    Ok(zt_world_mgr.to_string())
}

fn command_zt_world_mgr_types_summary(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let types = get_zt_world_mgr_types(&zt_world_mgr);
    let mut summary = "\n".to_string();
    let mut subtype: HashMap<String, u32> = HashMap::new();
    if types.is_empty() {
        return Ok("No types found".to_string());
    }
    let mut current_class = types[0].class.clone();
    for zt_type in types {
        if current_class != zt_type.class {
            let mut string_array = Vec::new();
            let mut total = 0;
            for (class, count) in subtype {
                string_array.push(format!("\t{:?}: {}", class, count));
                total += count;
            }
            summary.push_str(&format!("{:?}: ({})\n{}\n", current_class, total, string_array.join("\n")));
            info!("{:?}: ({})\n{}", current_class, total, string_array.join("\n"));
            subtype = HashMap::new();
            current_class = zt_type.class.clone();
        }
        info!("{:?}, {}", current_class, zt_type.zt_type);
        let count = subtype.entry(zt_type.zt_type).or_insert(0);
        *count += 1;
    }
    Ok(summary)
}

fn get_zt_world_mgr_entities(zt_world_mgr: &ZTWorldMgr) -> Vec<ZTEntity> {
    let entity_array_start = zt_world_mgr.entity_array_start;
    let entity_array_end = zt_world_mgr.entity_array_end;

    let mut entities: Vec<ZTEntity> = Vec::new();
    let mut i = entity_array_start;
    while i < entity_array_end {
        let zt_entity = read_zt_entity_from_memory(get_from_memory::<u32>(i));
        entities.push(zt_entity);
        i += 0x4;
    }
    entities
}

fn get_zt_world_mgr_entities_2(zt_world_mgr: &ZTWorldMgr) -> Vec<BFEntity> {
    let entity_array_start = zt_world_mgr.entity_array_start;
    let entity_array_end = zt_world_mgr.entity_array_end;

    let mut entities: Vec<BFEntity> = Vec::new();
    let mut i = entity_array_start;
    while i < entity_array_end {
        let bf_entity = get_from_memory(get_from_memory::<u32>(i));
        entities.push(bf_entity);
        i += 0x4;
    }
    entities
}

fn get_zt_world_mgr_types(zt_world_mgr: &ZTWorldMgr) -> Vec<ZTEntityType> {
    let entity_type_array_start = zt_world_mgr.entity_type_array_start;
    let entity_type_array_end = zt_world_mgr.entity_type_array_end;

    let mut entity_types: Vec<ZTEntityType> = Vec::new();
    let mut i = entity_type_array_start;
    while i < entity_type_array_end {
        info!("Reading entity at {:#x} -> {:#x}", i, get_from_memory::<u32>(i));
        let zt_entity_type = read_zt_entity_type_from_memory(get_from_memory::<u32>(i));
        entity_types.push(zt_entity_type);
        i += 0x4;
    }
    entity_types
}

pub fn get_entity_type_by_id(id: u32) -> u32 {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entity_type_array_start = zt_world_mgr.entity_type_array_start;
    let entity_type_array_end = zt_world_mgr.entity_type_array_end;

    let mut i = (entity_type_array_end - entity_type_array_start) / 0x4;

    info!("Searching {} entity types for id {}", i, id);

    i -= 1;

    // TODO: Currently this function only works with Scenery types. We need to generalize it to work with all entity types.
    // This section defines three sets of entity types each with distinct cName ID offsets.
    // let scenery_types: HashSet<&str> = ["Fences", "Path", "Rubble", "TankWall", "TankFilter", "Scenery", "Building"].iter().cloned().collect();
    // let unit_types: HashSet<&str> = ["Animal", "Guest", "Keeper", "MaintenanceWorker", "DRT", "TourGuide"].iter().cloned().collect();
    // let overlay_types: HashSet<&str> = ["Ambient"].iter().cloned().collect();

    while i > 0 {
        let entity_type_ptr = entity_type_array_start + i * 0x4;
        info!("Checking entity type at {:#x}", entity_type_ptr);
        let entity_type = map_from_memory::<ZTSceneryType>(entity_type_ptr);
        info!("Entity type name id: {}", entity_type.name_id);
        if entity_type.name_id == id {
            info!("Found entity type {}", entity_type.bfentitytype.get_type_name());
            return entity_type_ptr;
        } else {
            info!("Entity type {} does not match", entity_type.bfentitytype.get_type_name());
            i -= 1;
        }
    }
    0
}

// struct BFMap {
//     padding: [u8; 0x5c],
// }