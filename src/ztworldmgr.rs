
use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::add_to_command_register;

use tracing::info;
use std::collections::HashMap;
use std::fmt;
use num_enum::FromPrimitive;


const GLOBAL_ZTWORLDMGR_ADDRESS: u32 = 0x00638040;

#[derive(Debug)]
#[repr(C)]
pub struct zt_entity {
    class: u32,
    secondary_class_ptr: u32,
    secondary_class: u32,
    zt_class: String,
    zt_type: String,
    zt_sub_type: String,
    name: String,
}

#[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
#[repr(u32)]
enum ZtEntityTypeClass {
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

#[derive(Debug)]
struct ZtEntityType {
    ptr: u32,
    class_string: u32,
    class: ZtEntityTypeClass,
    zt_type: String,
    zt_sub_type: String,
}

#[derive(Debug)]
#[repr(C)]
pub struct zt_world_mgr {
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


pub fn read_zt_entity_from_memory(zt_entity_ptr: u32) -> zt_entity {
    let inner_class_ptr = get_from_memory::<u32>(zt_entity_ptr + 0x128);
    let secondary_class = get_from_memory(inner_class_ptr);

    info!("zt_entity_ptr: {:#x}", zt_entity_ptr);

    let ptr = get_from_memory::<u32>(secondary_class + 0x14) as *const ();
    let code: extern "thiscall" fn(u32) -> u32 = unsafe { std::mem::transmute(ptr) };
    let result = (code)(inner_class_ptr);
    
    zt_entity{
        class: get_from_memory::<u32>(zt_entity_ptr + 0x0),
        // secondary_class: get_from_memory::<u32>(get_from_memory::<u32>(zt_entity_ptr + 0x128)),
        secondary_class: secondary_class,
        secondary_class_ptr: inner_class_ptr,
        // zt_class: "not implemented".to_string(),
        zt_class: get_string_from_memory(get_from_memory::<u32>(result)),
        zt_type: get_string_from_memory(get_from_memory::<u32>(inner_class_ptr + 0x98)),
        zt_sub_type: get_string_from_memory(get_from_memory::<u32>(inner_class_ptr + 0xa4)),
        name: get_string_from_memory(get_from_memory::<u32>(zt_entity_ptr + 0x108)),
    }
}

fn log_zt_entity(zt_entity: &zt_entity) {
    info!("class: {:#x}", zt_entity.class);
    info!("secondary_class: {:#x}", zt_entity.secondary_class);
}

fn read_zt_entity_type_from_memory(zt_entity_type_ptr: u32) -> ZtEntityType {
    let class_string = get_from_memory::<u32>(zt_entity_type_ptr + 0x0);
    let class = ZtEntityTypeClass::from(class_string);

    ZtEntityType{
        ptr: zt_entity_type_ptr,
        class_string: class_string,
        class: class,
        zt_type: get_string_from_memory(get_from_memory::<u32>(zt_entity_type_ptr + 0x98)),
        zt_sub_type: get_string_from_memory(get_from_memory::<u32>(zt_entity_type_ptr + 0xa4)),
    }
}



pub fn read_zt_world_mgr_from_global() -> zt_world_mgr {
    read_zt_world_mgr_from_memory(read_zt_world_mgr_ptr())
}

pub fn read_zt_world_mgr_ptr() -> u32 {
    get_from_memory::<u32>(GLOBAL_ZTWORLDMGR_ADDRESS)
}

fn read_zt_world_mgr_from_memory(zt_world_mgr_ptr: u32) -> zt_world_mgr {
    zt_world_mgr{
        entity_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x80),
        entity_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x84),
        entity_type_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x98),
        entity_type_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x9c),
    }
}

fn log_zt_world_mgr(zt_world_mgr: &zt_world_mgr) {
    info!("zt_world_mgr: {:#?}", zt_world_mgr);
}

fn log_zt_world_mgr_entities(zt_world_mgr: &zt_world_mgr) {
    let entity_array_start = zt_world_mgr.entity_array_start;
    let entity_array_end = zt_world_mgr.entity_array_end;

    let mut i = entity_array_start;
    while i < entity_array_end {
        let zt_entity = read_zt_entity_from_memory(i);
        log_zt_entity(&zt_entity);
        i += 0x4;
    }
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

impl fmt::Display for zt_entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Entity Type: {:#x},Secondary Class Ptr: {:#x}, Secondary Type: {:#x}, ZT Class: {}, ZT Type: {}, ZT Sub Type: {}, Name: {}", self.class, self.secondary_class_ptr, self.secondary_class, self.zt_class, self.zt_type, self.zt_sub_type, self.name)
    }
}

impl fmt::Display for ZtEntityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Class String: {:#x}, Class: {:?}, ZT Type: {}, ZT Sub Type: {}", self.class_string, self.class, self.zt_type, self.zt_sub_type)
    }
}

impl fmt::Display for zt_world_mgr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let num_entities = (self.entity_array_end - self.entity_array_start) / 0x4;
        let num_entity_types = (self.entity_type_array_end - self.entity_type_array_start) / 0x4;
        write!(f, "Entity Array Start: {:#x}, Entity Array End: {:#x}, ({}), Entity Type Array Start: {:#x}, Entity Type Array End: {:#x}, ({})", self.entity_array_start, self.entity_array_end, num_entities, self.entity_type_array_start, self.entity_type_array_end, num_entity_types)
    }
}

fn get_zt_world_mgr_entities(zt_world_mgr: &zt_world_mgr) -> Vec<zt_entity> {
    let entity_array_start = zt_world_mgr.entity_array_start;
    let entity_array_end = zt_world_mgr.entity_array_end;

    let mut entities: Vec<zt_entity> = Vec::new();
    let mut i = entity_array_start;
    while i < entity_array_end {
        let zt_entity = read_zt_entity_from_memory(get_from_memory::<u32>(i));
        entities.push(zt_entity);
        i += 0x4;
    }
    return entities;
}

fn get_zt_world_mgr_types(zt_world_mgr: &zt_world_mgr) -> Vec<ZtEntityType> {
    let entity_type_array_start = zt_world_mgr.entity_type_array_start;
    let entity_type_array_end = zt_world_mgr.entity_type_array_end;

    let mut entity_types: Vec<ZtEntityType> = Vec::new();
    let mut i = entity_type_array_start;
    while i < entity_type_array_end {
        info!("Reading entity at {:#x}; end {:#x}", i, entity_type_array_end);
        let zt_entity_type = read_zt_entity_type_from_memory(get_from_memory::<u32>(i));
        entity_types.push(zt_entity_type);
        i += 0x4;
    }
    return entity_types;
}