
use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::add_to_command_register;
use crate::expansions::is_member;

use getset::Getters;

use tracing::info;
use std::collections::HashMap;
use std::fmt;
use num_enum::FromPrimitive;


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
    DRT = 0x62e980,
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

#[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
#[repr(u32)]
pub enum ZTEntityTypeClass {
    Animal = 0x630268,
    Ambient = 0x62e1e8,
    Guest = 0x62e330,
    Fences = 0x63034c,
    TourGuide = 0x62e8ac,
    Building = 0x6307e4,
    Scenery = 0x6303f4,
    Food = 0x630544,
    TankFilter = 0x630694,
    Path = 0x63049c,
    Rubble = 0x63073c,
    TankWall = 0x6305ec,
    Keeper = 0x62e7d8,
    MaintenanceWorker = 0x62e704,
    DRT = 0x62e980,
    #[num_enum(default)]
    Unknown = 0x0,
}


#[derive(Debug, Getters)]
#[get = "pub"]
pub struct ZTEntityType {
    ptr: u32,
    class_string: u32,
    class: ZTEntityTypeClass,
    zt_type: String,
    zt_sub_type: String,
    bf_config_file_ptr: u32,
}

impl ZTEntityType {
    pub fn is_member(&self, member: String) -> bool {
        match self.class {
            ZTEntityTypeClass::Animal |
            ZTEntityTypeClass::Guest |
            ZTEntityTypeClass::Fences |
            ZTEntityTypeClass::TourGuide |
            ZTEntityTypeClass::TankFilter |
            ZTEntityTypeClass::TankWall |
            ZTEntityTypeClass::Keeper |
            ZTEntityTypeClass::MaintenanceWorker |
            ZTEntityTypeClass::DRT => {
                is_member(&self.zt_type, &member)
            }
            ZTEntityTypeClass::Building |
            ZTEntityTypeClass::Scenery |
            ZTEntityTypeClass::Food |
            ZTEntityTypeClass::Path |
            ZTEntityTypeClass::Rubble |
            ZTEntityTypeClass::Ambient => {
                is_member(&self.zt_sub_type, &member)
            }

            ZTEntityTypeClass::Unknown => false,
        }
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
    // let secondary_class = get_from_memory(inner_class_ptr);
    // info!("zt_entity_ptr: {:#x}", zt_entity_ptr);
    // let ptr = get_from_memory::<u32>(secondary_class + 0x14) as *const ();
    // let code: extern "thiscall" fn(u32) -> u32 = unsafe { std::mem::transmute(ptr) };
    // let result = (code)(inner_class_ptr);
    
    ZTEntity{
        class: ZTEntityClass::from(get_from_memory::<u32>(zt_entity_ptr + 0x0)),
        // secondary_class: get_from_memory::<u32>(get_from_memory::<u32>(zt_entity_ptr + 0x128)),
        // secondary_class: secondary_class,
        // secondary_class_ptr: inner_class_ptr,
        type_class: read_zt_entity_type_from_memory(get_from_memory::<u32>(inner_class_ptr)),
        // zt_class: "not implemented".to_string(),
        // zt_class: get_string_from_memory(get_from_memory::<u32>(result)),
        // zt_type: get_string_from_memory(get_from_memory::<u32>(inner_class_ptr + 0x98)),
        // zt_sub_type: get_string_from_memory(get_from_memory::<u32>(inner_class_ptr + 0xa4)),
        name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
    }
}

pub fn read_zt_entity_type_from_memory(zt_entity_type_ptr: u32) -> ZTEntityType {
    let class_string = get_from_memory::<u32>(zt_entity_type_ptr + 0x0);
    let class = ZTEntityTypeClass::from(class_string);

    ZTEntityType{
        ptr: zt_entity_type_ptr,
        class_string: class_string,
        class: class,
        zt_type: get_string_from_memory(get_from_memory::<u32>(zt_entity_type_ptr + 0x98)),
        zt_sub_type: get_string_from_memory(get_from_memory::<u32>(zt_entity_type_ptr + 0xa4)),
        bf_config_file_ptr: get_from_memory::<u32>(zt_entity_type_ptr + 0x80),
    }
}



fn read_zt_world_mgr_from_global() -> ZTWorldMgr {
    let zt_world_mgr_ptr = get_from_memory::<u32>(GLOBAL_ZTWORLDMGR_ADDRESS);
    read_zt_world_mgr_from_memory(zt_world_mgr_ptr)
}

fn read_zt_world_mgr_from_memory(zt_world_mgr_ptr: u32) -> ZTWorldMgr {
    ZTWorldMgr{
        entity_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x80),
        entity_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x84),
        entity_type_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x98),
        entity_type_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x9c),
    }
}

fn log_zt_world_mgr(zt_world_mgr: &ZTWorldMgr) {
    info!("zt_world_mgr: {:#?}", zt_world_mgr);
}

fn command_get_zt_world_mgr_entities(_args: Vec<&str>) -> Result<String, &'static str> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entities = get_zt_world_mgr_entities(&zt_world_mgr);
    info!("Found {} entities", entities.len());
    if entities.len() == 0 {
        return Ok("No entities found".to_string());
    }
    let mut string_array = Vec::new();
    for entity in entities {
        string_array.push(entity.to_string());
    }
    Ok(string_array.join("\n"))
}

fn command_get_zt_world_mgr_types(_args: Vec<&str>) -> Result<String, &'static str> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let types = get_zt_world_mgr_types(&zt_world_mgr);
    info!("Found {} types", types.len());
    if types.len() == 0 {
        return Ok("No types found".to_string());
    }
    let mut string_array = Vec::new();
    for zt_type in types {
        string_array.push(zt_type.to_string());
    }
    Ok(string_array.join("\n"))
}

fn command_get_zt_world_mgr(_args: Vec<&str>) -> Result<String, &'static str> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    Ok(zt_world_mgr.to_string())
}

fn command_zt_world_mgr_types_summary(_args: Vec<&str>) -> Result<String, &'static str> {
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let types = get_zt_world_mgr_types(&zt_world_mgr);
    let mut summary = "\n".to_string();
    let mut subtype: HashMap<String, u32> = HashMap::new();
    if types.len() == 0 {
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

impl fmt::Display for ZTEntityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Class String: {:#x}, Class: {:?}, ZT Type: {}, ZT Sub Type: {}, ptr {:#x}, config_file_ptr {:#x}", self.class_string, self.class, self.zt_type, self.zt_sub_type, self.ptr, self.bf_config_file_ptr)
    }
}

impl fmt::Display for ZTWorldMgr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let num_entities = (self.entity_array_end - self.entity_array_start) / 0x4;
        let num_entity_types = (self.entity_type_array_end - self.entity_type_array_start) / 0x4;
        write!(f, "Entity Array Start: {:#x}, Entity Array End: {:#x}, ({}), Entity Type Array Start: {:#x}, Entity Type Array End: {:#x}, ({})", self.entity_array_start, self.entity_array_end, num_entities, self.entity_type_array_start, self.entity_type_array_end, num_entity_types)
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
    return entities;
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
    return entity_types;
}