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


