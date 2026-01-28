//! Roof tag extension for scenery entities

use std::collections::HashSet;
use tracing::info;

use crate::resource_manager::openzt_mods::extensions::{register_tag, EntityScope, list_extensions_with_tag, get_extension};
use crate::runtime_state;
use crate::resource_manager::openzt_mods::legacy_attributes::LegacyEntityType;
use crate::shortcuts::{Ctrl, R};
use crate::util::get_from_memory;
use crate::ztworldmgr::read_zt_world_mgr_from_global;

/// Hide all entities tagged with "roof"
///
/// This function iterates through all in-game entities and hides those
/// that have been tagged with the "roof" tag by setting their visible flag to 0.
///
/// The process:
/// 1. Gets all extensions with the "roof" tag
/// 2. Builds a set of base strings for roof-tagged entities
/// 3. Iterates through all in-game entities
/// 4. For each entity, checks if its base matches a roof entity
/// 5. If matched, sets the visible flag (offset 0x13f) to 0
pub fn hide_roofs() {
    info!("hide_roofs() called - hiding all roof-tagged entities");

    // Get all extensions with the "roof" tag
    let roof_extensions = list_extensions_with_tag("roof");
    if roof_extensions.is_empty() {
        info!("No extensions with 'roof' tag found");
        return;
    }

    info!("Found {} extensions with 'roof' tag: {:?}", roof_extensions.len(), roof_extensions);

    // Build a set of base strings that have the roof tag
    let mut roof_bases = HashSet::new();
    for ext_key in &roof_extensions {
        if let Some(record) = get_extension(ext_key) {
            info!("  Roof extension '{}' has base '{}'", ext_key, record.base);
            roof_bases.insert(record.base);
        } else {
            info!("  WARNING: Could not get extension record for key '{}'", ext_key);
        }
    }

    info!("Looking for {} roof-tagged base strings: {:?}", roof_bases.len(), roof_bases);

    // Get all in-game entities
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entity_array_start = zt_world_mgr.entity_array_start();
    let entity_array_end = zt_world_mgr.entity_array_end();

    let mut hidden_count = 0;
    let mut checked_count = 0;
    let mut no_base_count = 0;

    let mut i = entity_array_start;
    while i < entity_array_end {
        let entity_ptr = get_from_memory::<u32>(i);

        // Skip null pointers
        if entity_ptr != 0 {
            checked_count += 1;

            // Get the base string for this entity
            match crate::resource_manager::openzt_mods::extensions::get_entity_base(entity_ptr) {
                None => {
                    no_base_count += 1;
                }
                Some(base) => {
                    // Check if this entity type has the roof tag
                    if roof_bases.contains(&base) {
                        // Set visible flag to 0 (hidden) at offset 0x13f
                        unsafe {
                            let visible_ptr = (entity_ptr + 0x13f) as *mut u8;
                            *visible_ptr = 0;
                        }
                        hidden_count += 1;
                        info!("Hid roof entity: {} (ptr: 0x{:x})", base, entity_ptr);
                    }
                }
            }
        }

        i += 0x4;
    }

    info!("hide_roofs() complete: checked {} entities, {} had no base, hid {} roof entities",
        checked_count, no_base_count, hidden_count);
}

/// Show all entities tagged with "roof"
///
/// This function iterates through all in-game entities and shows those
/// that have been tagged with the "roof" tag by setting their visible flag to 1.
///
/// The process:
/// 1. Gets all extensions with the "roof" tag
/// 2. Builds a set of base strings for roof-tagged entities
/// 3. Iterates through all in-game entities
/// 4. For each entity, checks if its base matches a roof entity
/// 5. If matched, sets the visible flag (offset 0x13f) to 1
pub fn show_roofs() {
    info!("show_roofs() called - showing all roof-tagged entities");

    // Get all extensions with the "roof" tag
    let roof_extensions = list_extensions_with_tag("roof");
    if roof_extensions.is_empty() {
        info!("No extensions with 'roof' tag found");
        return;
    }

    // Build set of roof bases
    let mut roof_bases = HashSet::new();
    for ext_key in &roof_extensions {
        if let Some(record) = get_extension(ext_key) {
            roof_bases.insert(record.base);
        }
    }

    // Get all entities and show roof-tagged ones
    let zt_world_mgr = read_zt_world_mgr_from_global();
    let entity_array_start = zt_world_mgr.entity_array_start();
    let entity_array_end = zt_world_mgr.entity_array_end();

    let mut shown_count = 0;

    let mut i = entity_array_start;
    while i < entity_array_end {
        let entity_ptr = get_from_memory::<u32>(i);

        if entity_ptr != 0 {
            if let Some(base) = crate::resource_manager::openzt_mods::extensions::get_entity_base(entity_ptr) {
                if roof_bases.contains(&base) {
                    unsafe {
                        let visible_ptr = (entity_ptr + 0x13f) as *mut u8;
                        *visible_ptr = 1;  // Set visible
                    }
                    shown_count += 1;
                }
            }
        }

        i += 0x4;
    }

    info!("show_roofs() complete: showed {} roof entities", shown_count);
}

/// Toggle roof visibility
///
/// This function toggles the visibility of all roof-tagged entities.
/// It uses the runtime state store to track the current state and calls
/// the appropriate hide/show function.
pub fn toggle_roofs() {
    let is_hidden = runtime_state::toggle_bool("roofs_hidden");

    if is_hidden {
        info!("Toggling roofs: HIDING");
        hide_roofs();
    } else {
        info!("Toggling roofs: SHOWING");
        show_roofs();
    }
}

pub fn init() {
    info!("Initializing roofs module");

    // Register the "roof" tag - only applies to scenery entities
    match register_tag(
        "roofs",
        "roof",
        "Marks this scenery object as a roof that can be placed on buildings",
        EntityScope::single(LegacyEntityType::Scenery),
    ) {
        Ok(_) => info!("Registered 'roof' tag for scenery entities"),
        Err(e) => tracing::error!("Failed to register roof tag: {}", e),
    }

    // Register Ctrl+R shortcut to toggle roof visibility
    crate::shortcut!(
        "roofs",
        "Toggle roof visibility",
        Ctrl + R,
        false,  // override
        || {
            toggle_roofs();
        }
    );
}
