use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::add_to_command_register;

use tracing::info;
use std::collections::HashMap;
use std::fmt;
use num_enum::FromPrimitive;

trait BFEntityType {
    fn get_ncolors() -> u32; // 0x038
    fn set_ncolors(ncolors: u32); // 0x038
    fn get_icon_zoom() -> bool; // 0x050
    fn set_icon_zoom(icon_zoom: bool); // 0x050
    fn get_expansion_id() -> bool; // 0x054
    fn set_expansion_id(expansion_id: bool); // 0x054
    fn get_movable() -> bool; // 0x055
    fn set_movable(movable: bool); // 0x055
    fn get_walkable() -> bool; // 0x056
    fn set_walkable(walkable: bool); // 0x056
    fn get_walkable_by_tall() -> bool; // 0x057
    fn set_walkable_by_tall(walkable_by_tall: bool); // 0x057
    fn get_rubbleable() -> bool; // 0x059
    fn set_rubbleable(rubbleable: bool); // 0x059
    fn get_use_numbers_in_name() -> bool; // 0x05B
    fn set_use_numbers_in_name(use_numbers_in_name: bool); // 0x05B
    fn get_uses_real_shadows() -> bool; // 0x05C
    fn set_uses_real_shadows(uses_real_shadows: bool); // 0x05C
    fn get_has_shadow_images() -> bool; // 0x05D
    fn set_has_shadow_images(has_shadow_images: bool); // 0x05D
    fn get_force_shadow_black() -> bool; // 0x05E
    fn set_force_shadow_black(force_shadow_black: bool); // 0x05E
    fn get_draws_late() -> bool; // 0x060 <---- Might need double checking, not available in viewing canopies
    fn set_draws_late(draws_late: bool); // 0x060
    fn get_height() -> u32; // 0x064
    fn set_height(height: u32); // 0x064
    fn get_depth() -> u32; // 0x068
    fn set_depth(depth: u32); // 0x068
    fn get_has_underwater_section() -> bool; // 0x06C
    fn set_has_underwater_section(has_underwater_section: bool); // 0x06C
    fn get_is_transient() -> bool; // 0x06D
    fn set_is_transient(is_transient: bool); // 0x06D
    fn get_uses_placement_cube() -> bool; // 0x06E
    fn set_uses_placement_cube(uses_placement_cube: bool); // 0x06E
    fn get_show() -> bool; // 0x06F
    fn set_show(show: bool); // 0x06F
    fn get_hit_threshold() -> u32; // 0x070
    fn set_hit_threshold(hit_threshold: u32); // 0x070
    fn get_avoid_edges() -> bool; // 0x074
    fn set_avoid_edges(avoid_edges: bool); // 0x074
    fn get_type_name() -> String; // 0x0A4, 0x0A8
    fn set_type_name(type_name: String); // 0x0A4, 
    fn get_codename() -> String; // 0x098, 0x09C
    fn set_codename(codename: String); // 0x098, 0x09C
    fn get_footprintx() -> u32; // 0x0B4
    fn set_footprintx(footprintx: u32); // 0x0B4
    fn get_footprinty() -> u32; // 0x0B8
    fn set_footprinty(footprinty: u32); // 0x0B8
    fn get_footprintz() -> u32; // 0x0BC
    fn set_footprintz(footprintz: u32); // 0x0BC
    fn get_placement_footprintx() -> u32; // 0x0C0
    fn set_placement_footprintx(placement_footprintx: u32); // 0x0C0
    fn get_placement_footprinty() -> u32; // 0x0C4
    fn set_placement_footprinty(placement_footprinty: u32); // 0x0C4
    fn get_placement_footprintz() -> u32; // 0x0C8
    fn set_placement_footprintz(placement_footprintz: u32); // 0x0C8
    fn get_available_at_startup() -> bool; // 0x0CC
    fn set_available_at_startup(available_at_startup: bool); // 0x0CC
}

#[derive(Debug)]
#[repr(C)]
pub struct EntityType {
    this: u32,
}

impl BFEntityType for EntityType {
    fn this(ptr : u32) -> u32 {
        EntityType {
            this: ptr
        }
    }

    fn get_ncolors() -> u32 {
        get_from_memory::<u32>(this + 0x038)
    }

    fn set_ncolors(ncolors: u32) {
        get_from_memory::<u32>(this + 0x038) = ncolors;
    }

    fn get_icon_zoom() -> bool {
        get_from_memory::<bool>(this + 0x050)
    }

    fn set_icon_zoom(icon_zoom: bool) {
        get_from_memory::<bool>(this + 0x050) = icon_zoom;
    }

    fn get_expansion_id() -> bool {
        get_from_memory::<bool>(this + 0x054)
    }

    fn set_expansion_id(expansion_id: bool) {
        get_from_memory::<bool>(this + 0x054) = expansion_id;
    }

    fn get_movable() -> bool {
        get_from_memory::<bool>(this + 0x055)
    }

    fn set_movable(movable: bool) {
        get_from_memory::<bool>(this + 0x055) = movable;
    }

    fn get_walkable() -> bool {
        get_from_memory::<bool>(this + 0x056)
    }

    fn set_walkable(walkable: bool) {
        get_from_memory::<bool>(this + 0x056) = walkable;
    }

    fn get_walkable_by_tall() -> bool {
        get_from_memory::<bool>(this + 0x057)
    }

    fn set_walkable_by_tall(walkable_by_tall: bool) {
        get_from_memory::<bool>(this + 0x057) = walkable_by_tall;
    }

    fn get_rubbleable() -> bool {
        get_from_memory::<bool>(this + 0x059)
    }

    fn set_rubbleable(rubbleable: bool) {
        get_from_memory::<bool>(this + 0x059) = rubbleable;
    }

    fn get_use_numbers_in_name() -> bool {
        get_from_memory::<bool>(this + 0x05B)
    }

    fn set_use_numbers_in_name(use_numbers_in_name: bool) {
        get_from_memory::<bool>(this + 0x05B) = use_numbers_in_name;
    }

    fn get_uses_real_shadows() -> bool {
        get_from_memory::<bool>(this + 0x05C)
    }

    fn set_uses_real_shadows(uses_real_shadows: bool) {
        get_from_memory::<bool>(this + 0x05C) = uses_real_shadows;
    }

    fn get_has_shadow_images() -> bool {
        get_from_memory::<bool>(this + 0x05D)
    }

    fn set_has_shadow_images(has_shadow_images: bool) {
        get_from_memory::<bool>(this + 0x05D) = has_shadow_images;
    }

    fn get_force_shadow_black() -> bool {
        get_from_memory::<bool>(this + 0x05E)
    }

    fn set_force_shadow_black(force_shadow_black: bool) {
        get_from_memory::<bool>(this + 0x05E) = force_shadow_black;
    }

    fn get_draws_late() -> bool {
        get_from_memory::<bool>(this + 0x060)
    }

    fn set_draws_late(draws_late: bool) {
        get_from_memory::<bool>(this + 0x060) = draws_late;
    }

    fn get_height() -> u32 {
        get_from_memory::<u32>(this + 0x064)
    }

    fn set_height(height: u32) {
        get_from_memory::<u32>(this + 0x064) = height;
    }

    fn get_depth() -> u32 {
        get_from_memory::<u32>(this + 0x068)
    }

    fn set_depth(depth: u32) {
        get_from_memory::<u32>(this + 0x068) = depth;
    }

    fn get_has_underwater_section() -> bool {
        get_from_memory::<bool>(this + 0x06C)
    }

    fn set_has_underwater_section(has_underwater_section: bool) {
        get_from_memory::<bool>(this + 0x06C) = has_underwater_section;
    }

    fn get_is_transient() -> bool {
        get_from_memory::<bool>(this + 0x06D)
    }

    fn set_is_transient(is_transient: bool) {
        get_from_memory::<bool>(this + 0x06D) = is_transient;
    }

    fn get_uses_placement_cube() -> bool {
        get_from_memory::<bool>(this + 0x06E)
    }

    fn set_uses_placement_cube(uses_placement_cube: bool) {
        get_from_memory::<bool>(this + 0x06E) = uses_placement_cube;
    }

    fn get_show() -> bool {
        get_from_memory::<bool>(this + 0x06F)
    }

    fn set_show(show: bool) {
        get_from_memory::<bool>(this + 0x06F) = show;
    }

    fn get_hit_threshold() -> u32 {
        get_from_memory::<u32>(this + 0x070)
    }

    fn set_hit_threshold(hit_threshold: u32) {
        get_from_memory::<u32>(this + 0x070) = hit_threshold;
    }

    fn get_avoid_edges() -> bool {
        get_from_memory::<bool>(this + 0x074)
    }

    fn set_avoid_edges(avoid_edges: bool) {
        get_from_memory::<bool>(this + 0x074) = avoid_edges;
    }

    fn get_type_name() -> String {
        get_string_from_memory(this + 0x0A4)
    }

    fn set_type_name(type_name: String) {
        get_from_memory::<String>(this + 0x0A4) = type_name;
    }

    fn get_codename() -> String {
        get_string_from_memory(this + 0x098)
    }

    fn set_codename(codename: String) {
        get_from_memory::<String>(this + 0x098) = codename;
    }

    fn get_footprintx() -> u32 {
        get_from_memory::<u32>(this + 0x0B4)
    }

    fn set_footprintx(footprintx: u32) {
        get_from_memory::<u32>(this + 0x0B4) = footprintx;
    }

    fn get_footprinty() -> u32 {
        get_from_memory::<u32>(this + 0x0B8)
    }

    fn set_footprinty(footprinty: u32) {
        get_from_memory::<u32>(this + 0x0B8) = footprinty;
    }

    fn get_footprintz() -> u32 {
        get_from_memory::<u32>(this + 0x0BC)
    }

    fn set_footprintz(footprintz: u32) {
        get_from_memory::<u32>(this + 0x0BC) = footprintz;
    }

    fn get_placement_footprintx() -> u32 {
        get_from_memory::<u32>(this + 0x0C0)
    }

    fn set_placement_footprintx(placement_footprintx: u32) {
        get_from_memory::<u32>(this + 0x0C0) = placement_footprintx;
    }

    fn get_placement_footprinty() -> u32 {
        get_from_memory::<u32>(this + 0x0C4)
    }

    fn set_placement_footprinty(placement_footprinty: u32) {
        get_from_memory::<u32>(this + 0x0C4) = placement_footprinty;
    }

    fn get_placement_footprintz() -> u32 {
        get_from_memory::<u32>(this + 0x0C8)
    }

    fn set_placement_footprintz(placement_footprintz: u32) {
        get_from_memory::<u32>(this + 0x0C8) = placement_footprintz;
    }

    fn get_available_at_startup() -> bool {
        get_from_memory::<bool>(this + 0x0CC)
    }

    fn set_available_at_startup(available_at_startup: bool) {
        get_from_memory::<bool>(this + 0x0CC) = available_at_startup;
    }
}