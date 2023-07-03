
use crate::debug_dll::{get_from_memory, get_string_from_memory, get_zt_string_array_from_memory};
use tracing::info;
use std::fmt;


const GLOBAL_ZTWORLDMGR_ADDRESS: u32 = 0x00638040;

#[derive(Debug)]
#[repr(C)]
struct zt_entity {
    class: u32,
    secondary_class_ptr: u32,
    secondary_class: u32,
    zt_class: String,
    zt_type: String,
    zt_sub_type: String,
    name: String,
}

#[derive(Debug)]
#[repr(C)]
struct zt_world_mgr {
    entity_array_start: u32,
    entity_array_end: u32,
}


fn read_zt_entity_from_memory(zt_entity_ptr: u32) -> zt_entity {
    let inner_class_ptr = get_from_memory::<u32>(zt_entity_ptr + 0x128);
    let secondary_class = get_from_memory(inner_class_ptr);
    let class_name_getter = get_from_memory::<u32>(secondary_class + 0x14);

    // info!("inner_class_ptr: {:#x}", inner_class_ptr);
    // info!("secondary_class: {:#x}", secondary_class);
    info!("zt_entity_ptr: {:#x}", zt_entity_ptr);
    // info!("inner_class_ptr: {:#x}", inner_class_ptr);
    // info!("class_name_getter: {:#x}", class_name_getter);

    let ptr = get_from_memory::<u32>(secondary_class + 0x14) as *const ();
    let code: extern "thiscall" fn(u32) -> u32 = unsafe { std::mem::transmute(ptr) };
    let result = (code)(inner_class_ptr);
    
    // info!("result: {:#x}", result);

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

fn read_zt_world_mgr_from_global() -> zt_world_mgr {
    let zt_world_mgr_ptr = get_from_memory::<u32>(GLOBAL_ZTWORLDMGR_ADDRESS);
    read_zt_world_mgr_from_memory(zt_world_mgr_ptr)
}

fn read_zt_world_mgr_from_memory(zt_world_mgr_ptr: u32) -> zt_world_mgr {
    zt_world_mgr{
        entity_array_start: get_from_memory::<u32>(zt_world_mgr_ptr + 0x80),
        entity_array_end: get_from_memory::<u32>(zt_world_mgr_ptr + 0x84),
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

pub fn command_get_zt_world_mgr_entities(args: Vec<&str>) -> Result<String, &'static str> {
    // log_zt_world_mgr_entities(zt_world_mgr_from_global());
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

impl fmt::Display for zt_entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "Entity Type: {:#x}, Secondary Type: {:#x}", self.class, self.secondary_class)
        write!(f, "Entity Type: {:#x},Secondary Class Ptr: {:#x}, Secondary Type: {:#x}, ZT Class: {}, ZT Type: {}, ZT Sub Type: {}, Name: {}", self.class, self.secondary_class_ptr, self.secondary_class, self.zt_class, self.zt_type, self.zt_sub_type, self.name)
    }
}

fn get_zt_world_mgr_entities(zt_world_mgr: &zt_world_mgr) -> Vec<zt_entity> {
    let entity_array_start = zt_world_mgr.entity_array_start;
    let entity_array_end = zt_world_mgr.entity_array_end;

    let mut entities: Vec<zt_entity> = Vec::new();
    let mut i = entity_array_start;
    while i < entity_array_end {
        // info!("Reading entity at {:#x}; end {:#x}", i, entity_array_end);
        let zt_entity = read_zt_entity_from_memory(get_from_memory::<u32>(i));
        entities.push(zt_entity);
        i += 0x4;
    }
    return entities;
}