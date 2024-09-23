use std::{collections::HashMap, fmt};

use getset::Getters;
use num_enum::FromPrimitive;
use tracing::info;

use crate::{
    bfentitytype::{read_zt_entity_type_from_memory, ZTEntityType, ZTSceneryType},
    command_console::{add_to_command_register, CommandError},
    util::{get_from_memory, get_string_from_memory, map_from_memory},
};

const GLOBAL_ZTWORLDMGR_ADDRESS: u32 = 0x00638040;

#[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
#[repr(u32)]
pub enum ZTEntityClass {
    Food = 0x630544,
    Path = 0x63049c,
    Fences = 0x63034c,
    Building = 0x6307e4,
    Animal = 0x630268,
    Guest = 0x62e330,
    Scenery = 0x6303f4,
    Keeper = 0x62e7d8,
    MaintenanceWorker = 0x62e704,
    TourGuide = 0x62e8ac,
    Drt = 0x62e980,
    Ambient = 0x62e1e8,
    Rubble = 0x63073c,
    TankWall = 0x6305ec,
    TankFilter = 0x630694,
    #[num_enum(default)]
    Unknown = 0x0,
}

#[derive(Debug, Getters)]
#[get = "pub"]
#[repr(C)]
pub struct ZTEntity {
    class: ZTEntityClass,
    type_class: ZTEntityType, // TODO: Change to &ZTEntityType at some point?
    name: String,
}

impl ZTEntity {
    pub fn is_member(&self, member: String) -> bool {
        self.type_class.is_member(member)
    }
}

#[derive(Debug)]
#[repr(C)]
struct ZTWorldMgr {
    entity_array_start: u32,
    entity_array_end: u32,
    entity_type_array_start: u32,
    entity_type_array_end: u32,
}

pub fn init() {
    add_to_command_register("list_entities".to_owned(), command_get_zt_world_mgr_entities);
    add_to_command_register("list_types".to_owned(), command_get_zt_world_mgr_types);
    add_to_command_register("get_zt_world_mgr".to_owned(), command_get_zt_world_mgr);
    add_to_command_register("get_types_summary".to_owned(), command_zt_world_mgr_types_summary);
}

pub fn read_zt_entity_from_memory(zt_entity_ptr: u32) -> ZTEntity {
    let inner_class_ptr = get_from_memory::<u32>(zt_entity_ptr + 0x128);

    ZTEntity {
        class: ZTEntityClass::from(get_from_memory::<u32>(zt_entity_ptr)),
        type_class: read_zt_entity_type_from_memory(get_from_memory::<u32>(inner_class_ptr)),
        name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
    }
}

fn read_zt_world_mgr_from_global() -> ZTWorldMgr {
    let zt_world_mgr_ptr = get_from_memory::<u32>(GLOBAL_ZTWORLDMGR_ADDRESS);
    read_zt_world_mgr_from_memory(zt_world_mgr_ptr)
}

fn read_zt_world_mgr_from_memory(zt_world_mgr_ptr: u32) -> ZTWorldMgr {
    ZTWorldMgr {
        entity_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x80),
        entity_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x84),
        entity_type_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x98),
        entity_type_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x9c),
    }
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

impl fmt::Display for ZTEntity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Entity Type: {:?}, Name: {}, EntityType {}", self.class, self.name, self.type_class)
    }
}

impl fmt::Display for ZTWorldMgr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let num_entities = (self.entity_array_end - self.entity_array_start) / 0x4;
        let num_entity_types = (self.entity_type_array_end - self.entity_type_array_start) / 0x4;
        write!(
            f,
            "Entity Array Start: {:#x}, Entity Array End: {:#x}, ({}), Entity Type Array Start: {:#x}, Entity Type Array End: {:#x}, ({})",
            self.entity_array_start, self.entity_array_end, num_entities, self.entity_type_array_start, self.entity_type_array_end, num_entity_types
        )
    }
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

fn get_zt_world_mgr_types(zt_world_mgr: &ZTWorldMgr) -> Vec<ZTEntityType> {
    let entity_type_array_start = zt_world_mgr.entity_type_array_start;
    let entity_type_array_end = zt_world_mgr.entity_type_array_end;

    let mut entity_types: Vec<ZTEntityType> = Vec::new();
    let mut i = entity_type_array_start;
    while i < entity_type_array_end {
        info!("Reading entity at {:#x}; end {:#x}", i, entity_type_array_end);
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
