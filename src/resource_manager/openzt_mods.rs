mod loading;
mod habitats_locations;

pub use crate::resource_manager::openzt_mods::habitats_locations::{add_location_or_habitat, get_location_habitat_ids, get_location_or_habitat_by_id};
pub use crate::resource_manager::openzt_mods::loading::{get_mod_ids, get_num_mod_ids, load_def, load_open_zt_mod};
