use crate::add_to_command_register;
use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::ztui::get_selected_entity_type;

// ------------ BFEntityType, Implementation, and Related Functions ------------ //

use tracing::info;

#[derive(Debug)]
#[repr(C)]
struct BFEntityType {
    pad1: [u8; 0x038], // -- padding: 56 bytes
    ncolors: u32, // 0x038
    pad2: [u8; 0x050 - 0x03C], // -- padding: 20 bytes
    icon_zoom: bool, // 0x050
    pad3: [u8; 0x054 - 0x051], // -- padding: 3 bytes
    expansion_id: bool, // 0x054
    movable: bool, // 0x055
    walkable: bool, // 0x056
    walkable_by_tall: bool, // 0x057
    pad4: [u8; 0x059 - 0x058], // -- padding: 1 byte
    rubbleable: bool, // 0x059
    pad5: [u8; 0x05B - 0x05A], // -- padding: 1 byte
    use_numbers_in_name: bool, // 0x05B
    uses_real_shadows: bool, // 0x05C
    has_shadow_images: bool, // 0x05D
    force_shadow_black: bool, // 0x05E
    pad6: [u8; 0x060 - 0x05F], // -- padding: 1 byte
    draws_late: bool, // 0x060
    pad7: [u8; 0x064 - 0x061], // -- padding: 3 bytes
    height: u32, // 0x064
    depth: u32, // 0x068
    has_underwater_section: bool, // 0x06C
    is_transient: bool, // 0x06D
    uses_placement_cube: bool, // 0x06E
    show: bool, // 0x06F
    hit_threshold: u32, // 0x070
    avoid_edges: bool, // 0x074
    pad10: [u8; 0x0B4 - 0x075], // -- padding: 47 bytes
    footprintx: i32, // 0x0B4
    footprinty: i32, // 0x0B8
    footprintz: i32, // 0x0BC
    placement_footprintx: i32, // 0x0C0
    placement_footprinty: i32, // 0x0C4
    placement_footprintz: i32, // 0x0C8
    available_at_startup: bool // 0x0CC
}

impl BFEntityType {
    // returns the instance of the BFEntityType struct
    fn new(address: u32) -> Option<&'static mut BFEntityType> {
        unsafe {
            // get the pointer to the BFEntityType instance    
            let ptr = get_from_memory::<*mut BFEntityType>(address);
    
            // is pointer not null
            if !ptr.is_null() {
                Some(&mut *ptr)
            } 
            else {
                // pointer is null
                None
            }
        }
    }

    // returns the codename of the entity type
    fn get_codename(&self) -> String {
        let obj_ptr = self as *const BFEntityType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x098))
    }

    // returns the type name of the entity type
    fn get_type_name(&self) -> String {
        let obj_ptr = self as *const BFEntityType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x0A4))
    }
}

// returns the selected entity type
// usage: selected_type [-v] returns printed configuration of the selected entity type and other details
// usage: selected_type [-<config> <value>] sets the configuration of the selected entity type
pub fn command_selected_type(_args: Vec<&str>) -> Result<String, &'static str> {
    
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
    if entity_type_address == 0 {
        return Err("No entity selected");
    }
    let entity_type = BFEntityType::new(entity_type_address).unwrap(); // create a copied instance of the entity type

    // if no selected entity type, return error
    if _args.len() == 0 {
        return Ok(format!("No entity type selected."));
    }

    // if -v flag is used, print the entity type configuration and other details
    if _args[0] == "-v" {

        info!("Printing configuration for entity type at address {:#x}", entity_type_print);

        // NOTE: ncolors is part of a separate structure in memory withn BFEntityType, so we need to grab the pointer to it first
        // this is temporary until the struct can be fully implemented
        let ncolors_ptr = get_from_memory::<u32>(entity_type_print + 0x038);
        let ncolors = get_from_memory::<u32>(ncolors_ptr);
    
        Ok(format!("\n\n[Details]\n\nEntity Type Address: {:#x}\nType Name: {}\nCodename: {}\n\n[Configuration]\n\nncolors: {}\ncIconZoom: {}\ncExpansionID: {}\ncMovable: {}\ncWalkable: {}\ncWalkableByTall: {}\ncRubbleable: {}\ncUseNumbersInName: {}\ncUsesRealShadows: {}\ncHasShadowImages: {}\ncForceShadowBlack: {}\ncDrawsLate: {}\ncHeight: {}\ncDepth: {}\ncHasUnderwaterSection: {}\ncIsTransient: {}\ncUsesPlacementCube: {}\ncShow: {}\ncHitThreshold: {}\ncAvoidEdges: {}\ncFootprintX: {}\ncFootprintY: {}\ncFootprintZ: {}\ncPlacementFootprintX: {}\ncPlacementFootprintY: {}\ncPlacementFootprintZ: {}\ncAvailableAtStartup: {}\n\n",
        entity_type_print,
        entity_type.get_type_name(),
        entity_type.get_codename(),
        ncolors,
        entity_type.icon_zoom as u32,
        entity_type.expansion_id as u32,
        entity_type.movable as u32,
        entity_type.walkable as u32,
        entity_type.walkable_by_tall as u32,
        entity_type.rubbleable as u32,
        entity_type.use_numbers_in_name as u32,
        entity_type.uses_real_shadows as u32,
        entity_type.has_shadow_images as u32,
        entity_type.force_shadow_black as u32,
        entity_type.draws_late as u32,
        entity_type.height,
        entity_type.depth,
        entity_type.has_underwater_section as u32,
        entity_type.is_transient as u32,
        entity_type.uses_placement_cube as u32,
        entity_type.show as u32,
        entity_type.hit_threshold,
        entity_type.avoid_edges as u32,
        entity_type.footprintx,
        entity_type.footprinty,
        entity_type.footprintz,
        entity_type.placement_footprintx,
        entity_type.placement_footprinty,
        entity_type.placement_footprintz,
        entity_type.available_at_startup as u32       
        ))
    }
    else if _args.len() == 2 {
        if _args[0] == "-cIconZoom" {
            entity_type.icon_zoom = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cIconZoom set to {}", _args[1]));
        }
        else if _args[0] == "-cExpansionID" {
            entity_type.expansion_id = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cExpansionID set to {}", _args[1]));
        }
        else if _args[0] == "-cMovable" {
            entity_type.movable = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cMovable set to {}", _args[1]));
        }
        else if _args[0] == "-cWalkable" {
            entity_type.walkable = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cWalkable set to {}", _args[1]));
        }
        else if _args[0] == "-cWalkableByTall" {
            entity_type.walkable_by_tall = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cWalkableByTall set to {}", _args[1]));
        }
        else if _args[0] == "-cRubbleable" {
            entity_type.rubbleable = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cRubbleable set to {}", _args[1]));
        }
        else if _args[0] == "-cUseNumbersInName" {
            entity_type.use_numbers_in_name = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cUseNumbersInName set to {}", _args[1]));
        }
        else if _args[0] == "-cUsesRealShadows" {
            entity_type.uses_real_shadows = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cUsesRealShadows set to {}", _args[1]));
        }
        else if _args[0] == "-cHasShadowImages" {
            entity_type.has_shadow_images = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cHasShadowImages set to {}", _args[1]));
        }
        else if _args[0] == "-cForceShadowBlack" {
            entity_type.force_shadow_black = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cForceShadowBlack set to {}", _args[1]));
        }
        else if _args[0] == "-cDrawsLate" {
            entity_type.draws_late = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cDrawsLate set to {}", _args[1]));
        }
        else if _args[0] == "-cHeight" {
            entity_type.height = _args[1].parse::<u32>().unwrap();
            return Ok(format!("cHeight set to {}", _args[1]));
        }
        else if _args[0] == "-cDepth" {
            entity_type.depth = _args[1].parse::<u32>().unwrap();
            return Ok(format!("cDepth set to {}", _args[1]));
        }
        else if _args[0] == "-cHasUnderwaterSection" {
            entity_type.has_underwater_section = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cHasUnderwaterSection set to {}", _args[1]));
        }
        else if _args[0] == "-cIsTransient" {
            entity_type.is_transient = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cIsTransient set to {}", _args[1]));
        }
        else if _args[0] == "-cUsesPlacementCube" {
            entity_type.uses_placement_cube = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cUsesPlacementCube set to {}", _args[1]));
        }
        else if _args[0] == "-cShow" {
            entity_type.show = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cShow set to {}", _args[1]));
        }
        else if _args[0] == "-cHitThreshold" {
            entity_type.hit_threshold = _args[1].parse::<u32>().unwrap();
            return Ok(format!("cHitThreshold set to {}", _args[1]));
        }
        else if _args[0] == "-cAvoidEdges" {
            entity_type.avoid_edges = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cAvoidEdges set to {}", _args[1]));
        }
        else if _args[0] == "-cFootprintX" {
            entity_type.footprintx = _args[1].parse::<i32>().unwrap();
            return Ok(format!("cFootprintX set to {}", _args[1]));
        }
        else if _args[0] == "-cFootprintY" {
            entity_type.footprinty = _args[1].parse::<i32>().unwrap();
            return Ok(format!("cFootprintY set to {}", _args[1]));
        }
        else if _args[0] == "-cFootprintZ" {
            entity_type.footprintz = _args[1].parse::<i32>().unwrap();
            return Ok(format!("cFootprintZ set to {}", _args[1]));
        }
        else if _args[0] == "-cPlacementFootprintX" {
            entity_type.placement_footprintx = _args[1].parse::<i32>().unwrap();
            return Ok(format!("cPlacementFootprintX set to {}", _args[1]));
        }
        else if _args[0] == "-cPlacementFootprintY" {
            entity_type.placement_footprinty = _args[1].parse::<i32>().unwrap();
            return Ok(format!("cPlacementFootprintY set to {}", _args[1]));
        }
        else if _args[0] == "-cPlacementFootprintZ" {
            entity_type.placement_footprintz = _args[1].parse::<i32>().unwrap();
            return Ok(format!("cPlacementFootprintZ set to {}", _args[1]));
        }
        else if _args[0] == "-availableAtStartup" {
            entity_type.available_at_startup = _args[1].parse::<bool>().unwrap();
            return Ok(format!("cAvailableAtStartup set to {}", _args[1]));
        }
        else {
            return Ok("Invalid argument".to_string());
        }
        
    }
    else {
        Ok("Invalid argument".to_string())
    }
}

pub fn init() {
    add_to_command_register("selected_type".to_string(), command_selected_type);
    add_to_command_register("selected_scenery".to_string(), command_selected_scenery);
}


// ------------ ZTSceneryType, Implementation, and Related Functions ------------ //

#[derive(Debug)]
#[repr(C)]
struct ZTSceneryType {
    bfentitytype: BFEntityType, // 0x000
    pad1: [u8; 0xFC - 0x0CD], // -- padding: 35 bytes
    purchase_cost: f32, // 0x100
    name_id: u32, // 0x104
    help_id: u32, // 0x108
    habitat: u32, // 0x10C
    location: u32, // 0x110
    era: u32, // 0x114
    max_food_units: u32, // 0x118
    deletable: bool, // 0x119
    pad2: [u8; 0x11C - 0x11A], // -- padding: 2 bytes
    stink: bool, // 0x11C
    pad3: [u8; 0x120 - 0x11D], // -- padding: 3 bytes
    esthetic_weight: u32, // 0x120
    pad4: [u8; 0x128 - 0x124], // -- padding: 4 bytes
    selectable: bool, // 0x128
    pad5: [u8; 0x12A - 0x129], // -- padding: 1 byte
    foliage: bool, // 0x12A
    pad6: [u8; 0x12D - 0x12B], // -- padding: 2 bytes
    auto_rotate: bool, // 0x12D
    land: bool, // 0x12E
    swims: bool, // 0x12F
    underwater: bool, // 0x130
    surface: bool, // 0x131
    submerge: bool, // 0x132
    only_swims: bool, // 0x133
    needs_confirm: bool, // 0x134
    gawk_only_from_front: bool, // 0x135
    dead_on_land: bool, // 0x136
    dead_on_flat_water: bool, // 0x137
    dead_underwater: bool, // 0x138
    uses_tree_rubble: bool, // 0x139
    forces_scenery_rubble: bool, // 0x13A
    blocks_los: bool, // 0x13B
}

impl ZTSceneryType {
    fn new (address: u32) -> Option<&'static mut ZTSceneryType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTSceneryType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            }
            else {
                None
            }
        }
    }

    fn get_info_image_name(&self) -> String {
        let obj_ptr = self as *const ZTSceneryType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x14C))
    }
} 

fn command_selected_scenery(_args: Vec<&str>) -> Result<String, &'static str> {
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
    if entity_type_address == 0 {
        return Err("No entity selected");
    }
    let entity_type = ZTSceneryType::new(entity_type_address).unwrap(); // create a copied instance of the entity type

    // if no selected entity type, return error
    if _args.len() == 0 {
        return Ok(format!("No entity type selected."));
    }

    // if -v flag is used, print the entity type configuration and other details
    if _args[0] == "-v" {

        info!("Printing configuration for entity type at address {:#x}", entity_type_print);

        // NOTE: ncolors is part of a separate structure in memory withn BFEntityType, so we need to grab the pointer to it first
        // this is temporary until the struct can be fully implemented
        let ncolors_ptr = get_from_memory::<u32>(entity_type_print + 0x038);
        let ncolors = get_from_memory::<u32>(ncolors_ptr);
    
        Ok(format!("\n\n[Details]\n\nEntity Type Address: {:#x}\nType Name: {}\nCodename: {}\n\n[Configuration]\n\nncolors: {}\ncIconZoom: {}\ncExpansionID: {}\ncMovable: {}\ncWalkable: {}\ncWalkableByTall: {}\ncRubbleable: {}\ncUseNumbersInName: {}\ncUsesRealShadows: {}\ncHasShadowImages: {}\ncForceShadowBlack: {}\ncDrawsLate: {}\ncHeight: {}\ncDepth: {}\ncHasUnderwaterSection: {}\ncIsTransient: {}\ncUsesPlacementCube: {}\ncShow: {}\ncHitThreshold: {}\ncAvoidEdges: {}\ncFootprintX: {}\ncFootprintY: {}\ncFootprintZ: {}\ncPlacementFootprintX: {}\ncPlacementFootprintY: {}\ncPlacementFootprintZ: {}\ncAvailableAtStartup: {}\ncPurchaseCost: {:.2}\ncNameID: {}\ncHelpID: {}\ncHabitat: {}\ncLocation: {}\ncEra: {}\ncMaxFoodUnits: {}\ncDeletable: {}\ncStink: {}\ncEstheticWeight: {}\ncSelectable: {}\ncFoliage: {}\ncAutoRotate: {}\ncLand: {}\ncSwims: {}\ncUnderwater: {}\ncSurface: {}\ncSubmerge: {}\ncOnlySwims: {}\ncNeedsConfirm: {}\ncGawkOnlyFromFront: {}\ncDeadOnLand: {}\ncDeadOnFlatWater: {}\ncDeadUnderwater: {}\ncUsesTreeRubble: {}\ncForcesSceneryRubble: {}\ncBlocksLOS: {}\n\n",
        entity_type_print,
        entity_type.bfentitytype.get_type_name(),
        entity_type.bfentitytype.get_codename(),
        ncolors,
        entity_type.bfentitytype.icon_zoom as u32,
        entity_type.bfentitytype.expansion_id as u32,
        entity_type.bfentitytype.movable as u32,
        entity_type.bfentitytype.walkable as u32,
        entity_type.bfentitytype.walkable_by_tall as u32,
        entity_type.bfentitytype.rubbleable as u32,
        entity_type.bfentitytype.use_numbers_in_name as u32,
        entity_type.bfentitytype.uses_real_shadows as u32,
        entity_type.bfentitytype.has_shadow_images as u32,
        entity_type.bfentitytype.force_shadow_black as u32,
        entity_type.bfentitytype.draws_late as u32,
        entity_type.bfentitytype.height,
        entity_type.bfentitytype.depth,
        entity_type.bfentitytype.has_underwater_section as u32,
        entity_type.bfentitytype.is_transient as u32,
        entity_type.bfentitytype.uses_placement_cube as u32,
        entity_type.bfentitytype.show as u32,
        entity_type.bfentitytype.hit_threshold,
        entity_type.bfentitytype.avoid_edges as u32,
        entity_type.bfentitytype.footprintx,
        entity_type.bfentitytype.footprinty,
        entity_type.bfentitytype.footprintz,
        entity_type.bfentitytype.placement_footprintx,
        entity_type.bfentitytype.placement_footprinty,
        entity_type.bfentitytype.placement_footprintz,
        entity_type.bfentitytype.available_at_startup as u32,  
        entity_type.purchase_cost,
        entity_type.name_id,
        entity_type.help_id,
        entity_type.habitat,
        entity_type.location,
        entity_type.era,
        entity_type.max_food_units,
        entity_type.deletable as u32,
        entity_type.stink as u32,
        entity_type.esthetic_weight,
        entity_type.selectable as u32,
        entity_type.foliage as u32,
        entity_type.auto_rotate as u32,
        entity_type.land as u32,
        entity_type.swims as u32,
        entity_type.underwater as u32,
        entity_type.surface as u32,
        entity_type.submerge as u32,
        entity_type.only_swims as u32,
        entity_type.needs_confirm as u32,
        entity_type.gawk_only_from_front as u32,
        entity_type.dead_on_land as u32,
        entity_type.dead_on_flat_water as u32,
        entity_type.dead_underwater as u32,
        entity_type.uses_tree_rubble as u32,
        entity_type.forces_scenery_rubble as u32,
        entity_type.blocks_los as u32,

        ))
    }
    else {
        Ok("Invalid argument".to_string())
    }
}