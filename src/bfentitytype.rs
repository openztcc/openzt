use crate::add_to_command_register;
use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::ztui::get_selected_entity_type;

use tracing::info;

trait BFEntityType {
    fn get_ncolors(&self) -> u32; // 0x038
    fn set_ncolors(&self, ncolors: u32); // 0x038
    fn get_icon_zoom(&self) -> bool; // 0x050
    fn set_icon_zoom(&self, icon_zoom: bool); // 0x050
    fn get_expansion_id(&self) -> bool; // 0x054
    fn set_expansion_id(&self, expansion_id: bool); // 0x054
    fn get_movable(&self) -> bool; // 0x055
    fn set_movable(&self, movable: bool); // 0x055
    fn get_walkable(&self) -> bool; // 0x056
    fn set_walkable(&self, walkable: bool); // 0x056
    fn get_walkable_by_tall(&self) -> bool; // 0x057
    fn set_walkable_by_tall(&self, walkable_by_tall: bool); // 0x057
    fn get_rubbleable(&self) -> bool; // 0x059
    fn set_rubbleable(&self, rubbleable: bool); // 0x059
    fn get_use_numbers_in_name(&self) -> bool; // 0x05B
    fn set_use_numbers_in_name(&self, use_numbers_in_name: bool); // 0x05B
    fn get_uses_real_shadows(&self) -> bool; // 0x05C
    fn set_uses_real_shadows(&self, uses_real_shadows: bool); // 0x05C
    fn get_has_shadow_images(&self) -> bool; // 0x05D
    fn set_has_shadow_images(&self, has_shadow_images: bool); // 0x05D
    fn get_force_shadow_black(&self) -> bool; // 0x05E
    fn set_force_shadow_black(&self, force_shadow_black: bool); // 0x05E
    fn get_draws_late(&self) -> bool; // 0x060 <---- Might need double checking, not available in viewing canopies
    fn set_draws_late(&self, draws_late: bool); // 0x060
    fn get_height(&self) -> u32; // 0x064
    fn set_height(&self, height: u32); // 0x064
    fn get_depth(&self) -> u32; // 0x068
    fn set_depth(&self, depth: u32); // 0x068
    fn get_has_underwater_section(&self) -> bool; // 0x06C
    fn set_has_underwater_section(&self, has_underwater_section: bool); // 0x06C
    fn get_is_transient(&self) -> bool; // 0x06D
    fn set_is_transient(&self, is_transient: bool); // 0x06D
    fn get_uses_placement_cube(&self) -> bool; // 0x06E
    fn set_uses_placement_cube(&self, uses_placement_cube: bool); // 0x06E
    fn get_show(&self) -> bool; // 0x06F
    fn set_show(&self, show: bool); // 0x06F
    fn get_hit_threshold(&self) -> u32; // 0x070
    fn set_hit_threshold(&self, hit_threshold: u32); // 0x070
    fn get_avoid_edges(&self) -> bool; // 0x074
    fn set_avoid_edges(&self, avoid_edges: bool); // 0x074
    fn get_type_name(&self) -> String; // 0x0A4, 0x0A8
    fn set_type_name(&self, type_name: String); // 0x0A4, 
    fn get_codename(&self) -> String; // 0x098, 0x09C
    fn set_codename(&self, codename: String); // 0x098, 0x09C
    fn get_footprintx(&self) -> u32; // 0x0B4
    fn set_footprintx(&self, footprintx: u32); // 0x0B4
    fn get_footprinty(&self) -> u32; // 0x0B8
    fn set_footprinty(&self, footprinty: u32); // 0x0B8
    fn get_footprintz(&self) -> u32; // 0x0BC
    fn set_footprintz(&self, footprintz: u32); // 0x0BC
    fn get_placement_footprintx(&self) -> u32; // 0x0C0
    fn set_placement_footprintx(&self, placement_footprintx: u32); // 0x0C0
    fn get_placement_footprinty(&self) -> u32; // 0x0C4
    fn set_placement_footprinty(&self, placement_footprinty: u32); // 0x0C4
    fn get_placement_footprintz(&self) -> i32; // 0x0C8
    fn set_placement_footprintz(&self, placement_footprintz: i32); // 0x0C8
    fn get_available_at_startup(&self) -> bool; // 0x0CC
    fn set_available_at_startup(&self, available_at_startup: bool); // 0x0CC
    fn new(ptr: u32) -> Self;
}

#[derive(Debug)]
#[repr(C)]
pub struct EntityType {
    this: u32,
}

impl BFEntityType for EntityType {
    fn new(ptr: u32) -> EntityType {
        EntityType {
            this : get_from_memory::<u32>(ptr),
        }
    }

    fn get_ncolors(&self) -> u32 {
        let ncolors_ptr = get_from_memory::<u32>(self.this + 0x038);
        let ncolors = get_from_memory::<u32>(ncolors_ptr);
        ncolors
    }

    // TODO: this should be a double pointer to set, like above
    fn set_ncolors(&self, ncolors: u32) {
        unsafe {
            let ncolors_ptr: *mut u32 = (self.this + 0x038) as *mut u32;
            *ncolors_ptr = ncolors;
        }
    }

    fn get_icon_zoom(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x050)
    }

    fn set_icon_zoom(&self, icon_zoom: bool) {
        unsafe {
            let icon_zoom_ptr: *mut bool = (self.this + 0x050) as *mut bool;
            *icon_zoom_ptr = icon_zoom;
        }
    }

    fn get_expansion_id(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x054)
    }

    fn set_expansion_id(&self, expansion_id: bool) {
        unsafe {
            let expansion_id_ptr: *mut bool = (self.this + 0x054) as *mut bool;
            *expansion_id_ptr = expansion_id;
        }
    }

    fn get_movable(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x055)
    }

    fn set_movable(&self, movable: bool) {
        unsafe {
            let movable_ptr: *mut bool = (self.this + 0x055) as *mut bool;
            *movable_ptr = movable;
        }
    }

    fn get_walkable(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x056)
    }

    fn set_walkable(&self, walkable: bool) {
        unsafe {
            let walkable_ptr: *mut bool = (self.this + 0x056) as *mut bool;
            *walkable_ptr = walkable;
        }
    }

    fn get_walkable_by_tall(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x057)
    }

    fn set_walkable_by_tall(&self, walkable_by_tall: bool) {
        unsafe {
            let walkable_by_tall_ptr: *mut bool = (self.this + 0x057) as *mut bool;
            *walkable_by_tall_ptr = walkable_by_tall;
        }
    }

    fn get_rubbleable(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x059)
    }

    fn set_rubbleable(&self, rubbleable: bool) {
        unsafe {
            let rubbleable_ptr: *mut bool = (self.this + 0x059) as *mut bool;
            *rubbleable_ptr = rubbleable;
        }
    }

    fn get_use_numbers_in_name(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x05B)
    }

    fn set_use_numbers_in_name(&self, use_numbers_in_name: bool) {
        unsafe {
            let use_numbers_in_name_ptr: *mut bool = (self.this + 0x05B) as *mut bool;
            *use_numbers_in_name_ptr = use_numbers_in_name;
        }
    }

    fn get_uses_real_shadows(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x05C)
    }

    fn set_uses_real_shadows(&self, uses_real_shadows: bool) {
        unsafe {
            let uses_real_shadows_ptr: *mut bool = (self.this + 0x05C) as *mut bool;
            *uses_real_shadows_ptr = uses_real_shadows;
        }
    }

    fn get_has_shadow_images(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x05D)
    }

    fn set_has_shadow_images(&self, has_shadow_images: bool) {
        unsafe {
            let has_shadow_images_ptr: *mut bool = (self.this + 0x05D) as *mut bool;
            *has_shadow_images_ptr = has_shadow_images;
        }
    }

    fn get_force_shadow_black(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x05E)
    }

    fn set_force_shadow_black(&self, force_shadow_black: bool) {
        unsafe {
            let force_shadow_black_ptr: *mut bool = (self.this + 0x05E) as *mut bool;
            *force_shadow_black_ptr = force_shadow_black;
        }
    }

    fn get_draws_late(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x060)
    }

    fn set_draws_late(&self, draws_late: bool) {
        unsafe {
            let draws_late_ptr: *mut bool = (self.this + 0x060) as *mut bool;
            *draws_late_ptr = draws_late;
        }
    }

    fn get_height(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x064)
    }

    fn set_height(&self, height: u32) {
        unsafe {
            let height_ptr: *mut u32 = (self.this + 0x064) as *mut u32;
            *height_ptr = height;
        }
    }

    fn get_depth(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x068)
    }

    fn set_depth(&self, depth: u32) {
        unsafe {
            let depth_ptr: *mut u32 = (self.this + 0x068) as *mut u32;
            *depth_ptr = depth;
        }
    }

    fn get_has_underwater_section(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x06C)
    }

    fn set_has_underwater_section(&self, has_underwater_section: bool) {
        unsafe {
            let has_underwater_section_ptr: *mut bool = (self.this + 0x06C) as *mut bool;
            *has_underwater_section_ptr = has_underwater_section;
        }
    }

    fn get_is_transient(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x06D)
    }

    fn set_is_transient(&self, is_transient: bool) {
        unsafe {
            let is_transient_ptr: *mut bool = (self.this + 0x06D) as *mut bool;
            *is_transient_ptr = is_transient;
        }
    }

    fn get_uses_placement_cube(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x06E)
    }

    fn set_uses_placement_cube(&self, uses_placement_cube: bool) {
        unsafe {
            let uses_placement_cube_ptr: *mut bool = (self.this + 0x06E) as *mut bool;
            *uses_placement_cube_ptr = uses_placement_cube;
        }
    }

    fn get_show(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x06F)
    }

    fn set_show(&self, show: bool) {
        unsafe {
            let show_ptr: *mut bool = (self.this + 0x06F) as *mut bool;
            *show_ptr = show;
        }
    }

    fn get_hit_threshold(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x070)
    }

    fn set_hit_threshold(&self, hit_threshold: u32) {
        unsafe {
            let hit_threshold_ptr: *mut u32 = (self.this + 0x070) as *mut u32;
            *hit_threshold_ptr = hit_threshold;
        }
    }

    fn get_avoid_edges(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x074)
    }

    fn set_avoid_edges(&self, avoid_edges: bool) {
        unsafe {
            let avoid_edges_ptr: *mut bool = (self.this + 0x074) as *mut bool;
            *avoid_edges_ptr = avoid_edges;
        }
    }

    fn get_type_name(&self) -> String {
        let type_name_ptr = get_from_memory::<u32>(self.this + 0x0A4);
        get_string_from_memory(type_name_ptr)
    }

    fn set_type_name(&self, type_name: String) {
        unsafe {
            let type_name_ptr: *mut String = (self.this + 0x0A4) as *mut String;
            *type_name_ptr = type_name;
        }
    }

    fn get_codename(&self) -> String {
        let codename_ptr = get_from_memory::<u32>(self.this + 0x098);
        get_string_from_memory(codename_ptr)
    }

    fn set_codename(&self, codename: String) {
        unsafe {
            let codename_ptr: *mut String = (self.this + 0x098) as *mut String;
            *codename_ptr = codename;
        }
    }

    fn get_footprintx(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x0B4)
    }

    fn set_footprintx(&self, footprintx: u32) {
        unsafe {
            let footprintx_ptr: *mut u32 = (self.this + 0x0B4) as *mut u32;
            *footprintx_ptr = footprintx;
        }
    }

    fn get_footprinty(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x0B8)
    }

    fn set_footprinty(&self, footprinty: u32) {
        unsafe {
            let footprinty_ptr: *mut u32 = (self.this + 0x0B8) as *mut u32;
            *footprinty_ptr = footprinty;
        }
    }

    fn get_footprintz(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x0BC)
    }

    fn set_footprintz(&self, footprintz: u32) {
        unsafe {
            let footprintz_ptr: *mut u32 = (self.this + 0x0BC) as *mut u32;
            *footprintz_ptr = footprintz;
        }
    }

    fn get_placement_footprintx(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x0C0)
    }

    fn set_placement_footprintx(&self, placement_footprintx: u32) {
        unsafe {
            let placement_footprintx_ptr: *mut u32 = (self.this + 0x0C0) as *mut u32;
            *placement_footprintx_ptr = placement_footprintx;
        }
    }

    fn get_placement_footprinty(&self) -> u32 {
        get_from_memory::<u32>(self.this + 0x0C4)
    }

    fn set_placement_footprinty(&self, placement_footprinty: u32) {
        unsafe {
            let placement_footprinty_ptr: *mut u32 = (self.this + 0x0C4) as *mut u32;
            *placement_footprinty_ptr = placement_footprinty;
        }
    }

    fn get_placement_footprintz(&self) -> i32 {
        get_from_memory::<i32>(self.this + 0x0C8)
    }

    fn set_placement_footprintz(&self, placement_footprintz: i32) {
        unsafe {
            let placement_footprintz_ptr: *mut i32 = (self.this + 0x0C8) as *mut i32;
            *placement_footprintz_ptr = placement_footprintz;
        }
    }

    fn get_available_at_startup(&self) -> bool {
        get_from_memory::<bool>(self.this + 0x0CC)
    }

    fn set_available_at_startup(&self, available_at_startup: bool) {
        unsafe {
            let available_at_startup_ptr: *mut bool = (self.this + 0x0CC) as *mut bool;
            *available_at_startup_ptr = available_at_startup;
        }
    }

}

pub fn command_print_configuration(_args: Vec<&str>) -> Result<String, &'static str> {
    
    let entity_type_address = get_selected_entity_type();
    let entity_type_ptr = get_from_memory::<u32>(entity_type_address);
    if entity_type_ptr == 0 {
        return Err("No entity selected");
    }
    let entity_type = EntityType::new(entity_type_address);
    info!("Printing configuration for entity type at address {:#x}", entity_type_ptr);
  
    Ok(format!("\n\n[Details]\nEntityType: {:#x}\nType Name: {}\nCodename: {}\n\n[Printed configuration]\nncolors: {}\ncIconZoom: {}\ncExpansionID: {}\ncMovable: {}\ncWalkable: {}\ncWalkableByTall: {}\ncRubbleable: {}\ncUseNumbersInName: {}\ncUsesRealShadows: {}\ncHasShadowImages: {}\ncForceShadowBlack: {}\ncDrawsLate: {}\ncHeight: {}\ncDepth: {}\ncHasUnderwaterSection: {}\ncIsTransient: {}\ncUsesPlacementCube: {}\ncShow: {}\ncHitThreshold: {}\ncAvoidEdges: {}\ncFootprintX: {}\ncFootprintY: {}\ncFootprintZ: {}\ncPlacementFootprintX: {}\ncPlacementFootprintY: {}\ncPlacementFootprintZ: {}\ncAvailableAtStartup: {}\n",
        entity_type.this,
        entity_type.get_type_name(),
        entity_type.get_codename(),
        entity_type.get_ncolors(),
        entity_type.get_icon_zoom() as u32,
        entity_type.get_expansion_id() as u32,
        entity_type.get_movable() as u32,
        entity_type.get_walkable() as u32,
        entity_type.get_walkable_by_tall() as u32,
        entity_type.get_rubbleable() as u32,
        entity_type.get_use_numbers_in_name() as u32,
        entity_type.get_uses_real_shadows() as u32,
        entity_type.get_has_shadow_images() as u32,
        entity_type.get_force_shadow_black() as u32,
        entity_type.get_draws_late() as u32,
        entity_type.get_height(),
        entity_type.get_depth(),
        entity_type.get_has_underwater_section() as u32,
        entity_type.get_is_transient() as u32,
        entity_type.get_uses_placement_cube() as u32,
        entity_type.get_show() as u32,
        entity_type.get_hit_threshold(),
        entity_type.get_avoid_edges() as u32,
        entity_type.get_footprintx(),
        entity_type.get_footprinty(),
        entity_type.get_footprintz(),
        entity_type.get_placement_footprintx(),
        entity_type.get_placement_footprinty(),
        entity_type.get_placement_footprintz(),
        entity_type.get_available_at_startup() as u32
     ))
}

pub fn init() {
    add_to_command_register("print_selected_configuration".to_string(), command_print_configuration);
}
