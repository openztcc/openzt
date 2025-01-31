mod habitats_locations;
mod loading;

pub use crate::resource_manager::openzt_mods::{
    habitats_locations::{get_location_habitat_ids, get_location_or_habitat_by_id},
    loading::{get_mod_ids, get_num_mod_ids, load_open_zt_mod},
};
