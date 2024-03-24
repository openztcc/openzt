use tracing::info;

use crate::console::add_to_command_register;

use crate::ztworldmgr::read_zt_entity_from_memory;

pub fn init() {
    add_to_command_register("get_selected_entity".to_owned(), command_get_selected_entity);
}

fn command_get_selected_entity(_args: Vec<&str>) -> Result<String, &'static str> {
    let get_selected_entity_fn = unsafe { std::mem::transmute::<u32, fn() -> u32>(0x00410f84) }; //TODO: Move type to variable declaration rather than turbofish
    let entity_address = get_selected_entity_fn();
    if entity_address == 0 {
        return Ok("No entity selected".to_string());
    }
    let entity = read_zt_entity_from_memory(entity_address);
    Ok(format!("{:#?}", entity))
}

// returns the address of the selected entity
pub fn get_selected_entity() -> u32 {
    let get_selected_entity_fn = unsafe { std::mem::transmute::<u32, fn() -> u32>(0x00410f84) };
    get_selected_entity_fn()
}

// returns the address of the selected entity type
pub fn get_selected_entity_type() -> u32 {
    let selected_entity = get_selected_entity();
    if selected_entity == 0 {
        return 0;
    }

    let entity_type_address = selected_entity + 0x128;
    entity_type_address
}


