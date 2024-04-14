// ------------ BFEntityType, Implementation, and Related Functions ------------ //
use std::ops::Deref;

use tracing::info;

use getset::Getters;
use getset::Setters;

use crate::{
    add_to_command_register,
    debug_dll::{get_from_memory, get_string_from_memory},
    ztui::get_selected_entity_type,
    ztworldmgr::determine_entity_type,
};

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct BFEntityType {
    pad1: [u8; 0x038],                // ----------------------- padding: 56 bytes
    pub ncolors: u32,                 // 0x038
    pad2: [u8; 0x050 - 0x03C],        // ----------------------- padding: 20 bytes
    pub icon_zoom: bool,              // 0x050
    pad3: [u8; 0x054 - 0x051],        // ----------------------- padding: 3 bytes
    pub expansion_id: bool,           // 0x054
    pub movable: bool,                // 0x055
    pub walkable: bool,               // 0x056
    pub walkable_by_tall: bool,       // 0x057
    pad4: [u8; 0x059 - 0x058],        // ----------------------- padding: 1 byte
    pub rubbleable: bool,             // 0x059
    pad5: [u8; 0x05B - 0x05A],        // ----------------------- padding: 1 byte
    pub use_numbers_in_name: bool,    // 0x05B
    pub uses_real_shadows: bool,      // 0x05C
    pub has_shadow_images: bool,      // 0x05D
    pub force_shadow_black: bool,     // 0x05E
    pad6: [u8; 0x060 - 0x05F],        // ----------------------- padding: 1 byte
    pub draws_late: bool,             // 0x060
    pad7: [u8; 0x064 - 0x061],        // ----------------------- padding: 3 bytes
    pub height: u32,                  // 0x064
    pub depth: u32,                   // 0x068
    pub has_underwater_section: bool, // 0x06C
    pub is_transient: bool,           // 0x06D
    pub uses_placement_cube: bool,    // 0x06E
    pub show: bool,                   // 0x06F
    pub hit_threshold: u32,           // 0x070
    pub avoid_edges: bool,            // 0x074
    pad10: [u8; 0x0B4 - 0x075],       // ----------------------- padding: 47 bytes
    pub footprintx: i32,              // 0x0B4
    pub footprinty: i32,              // 0x0B8
    pub footprintz: i32,              // 0x0BC
    pub placement_footprintx: i32,    // 0x0C0
    pub placement_footprinty: i32,    // 0x0C4
    pub placement_footprintz: i32,    // 0x0C8
    pub available_at_startup: bool,   // 0x0CC
    pad11: [u8; 0x100 - 0x0CD],       // ----------------------- padding: 35 bytes
}

impl BFEntityType {
    // returns the instance of the BFEntityType struct
    pub fn new(address: u32) -> Option<&'static mut BFEntityType> {
        unsafe {
            // get the pointer to the BFEntityType instance
            let ptr = get_from_memory::<*mut BFEntityType>(address);

            // is pointer not null
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                // pointer is null
                None
            }
        }
    }

    // returns the codename of the entity type
    pub fn get_codename(&self) -> String {
        let obj_ptr = self as *const BFEntityType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x0A4))
    }

    // returns the type name of the entity type
    pub fn get_type_name(&self) -> String {
        let obj_ptr = self as *const BFEntityType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x098))
    }

    pub fn get_info_image_name(&self) -> String {
        let obj_ptr = self as *const BFEntityType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x148))
    }

    // allows setting the configuration of the entity type
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cIconZoom" {
            self.icon_zoom = value.parse::<bool>().unwrap();
            Ok(format!("Set cIconZoom to {}", self.icon_zoom))
        } else if config == "-cExpansionID" {
            self.expansion_id = value.parse::<bool>().unwrap();
            Ok(format!("Set cExpansionID to {}", self.expansion_id))
        } else if config == "-cMovable" {
            self.movable = value.parse::<bool>().unwrap();
            Ok(format!("Set cMovable to {}", self.movable))
        } else if config == "-cWalkable" {
            self.walkable = value.parse::<bool>().unwrap();
            Ok(format!("Set cWalkable to {}", self.walkable))
        } else if config == "-cWalkableByTall" {
            self.walkable_by_tall = value.parse::<bool>().unwrap();
            Ok(format!("Set cWalkableByTall to {}", self.walkable_by_tall))
        } else if config == "-cRubbleable" {
            self.rubbleable = value.parse::<bool>().unwrap();
            Ok(format!("Set cRubbleable to {}", self.rubbleable))
        } else if config == "-cUseNumbersInName" {
            self.use_numbers_in_name = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cUseNumbersInName to {}",
                self.use_numbers_in_name
            ))
        } else if config == "-cUsesRealShadows" {
            self.uses_real_shadows = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cUsesRealShadows to {}",
                self.uses_real_shadows
            ))
        } else if config == "-cHasShadowImages" {
            self.has_shadow_images = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cHasShadowImages to {}",
                self.has_shadow_images
            ))
        } else if config == "-cForceShadowBlack" {
            self.force_shadow_black = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cForceShadowBlack to {}",
                self.force_shadow_black
            ))
        } else if config == "-cDrawsLate" {
            self.draws_late = value.parse::<bool>().unwrap();
            Ok(format!("Set cDrawsLate to {}", self.draws_late))
        } else if config == "-cHeight" {
            self.height = value.parse::<u32>().unwrap();
            Ok(format!("Set cHeight to {}", self.height))
        } else if config == "-cDepth" {
            self.depth = value.parse::<u32>().unwrap();
            Ok(format!("Set cDepth to {}", self.depth))
        } else if config == "-cHasUnderwaterSection" {
            self.has_underwater_section = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cHasUnderwaterSection to {}",
                self.has_underwater_section
            ))
        } else if config == "-cIsTransient" {
            self.is_transient = value.parse::<bool>().unwrap();
            Ok(format!("Set cIsTransient to {}", self.is_transient))
        } else if config == "-cUsesPlacementCube" {
            self.uses_placement_cube = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cUsesPlacementCube to {}",
                self.uses_placement_cube
            ))
        } else if config == "-cShow" {
            self.show = value.parse::<bool>().unwrap();
            Ok(format!("Set cShow to {}", self.show))
        } else if config == "-cHitThreshold" {
            self.hit_threshold = value.parse::<u32>().unwrap();
            Ok(format!("Set cHitThreshold to {}", self.hit_threshold))
        } else if config == "-cAvoidEdges" {
            self.avoid_edges = value.parse::<bool>().unwrap();
            Ok(format!("Set cAvoidEdges to {}", self.avoid_edges))
        } else if config == "-cFootprintX" {
            self.footprintx = value.parse::<i32>().unwrap();
            Ok(format!("Set cFootprintX to {}", self.footprintx))
        } else if config == "-cFootprintY" {
            self.footprinty = value.parse::<i32>().unwrap();
            Ok(format!("Set cFootprintY to {}", self.footprinty))
        } else if config == "-cFootprintZ" {
            self.footprintz = value.parse::<i32>().unwrap();
            Ok(format!("Set cFootprintZ to {}", self.footprintz))
        } else if config == "-cPlacementFootprintX" {
            self.placement_footprintx = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set cPlacementFootprintX to {}",
                self.placement_footprintx
            ))
        } else if config == "-cPlacementFootprintY" {
            self.placement_footprinty = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set cPlacementFootprintY to {}",
                self.placement_footprinty
            ))
        } else if config == "-cPlacementFootprintZ" {
            self.placement_footprintz = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set cPlacementFootprintZ to {}",
                self.placement_footprintz
            ))
        } else if config == "-cAvailableAtStartup" {
            self.available_at_startup = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set cAvailableAtStartup to {}",
                self.available_at_startup
            ))
        } else {
            Err("Invalid configuration option")
        }
    }

    // prints [colorrep] section of the configuration
    fn print_colorrep(&self) -> String {
        // NOTE: ncolors is part of a separate structure in memory withn BFEntityType, so we need to grab the pointer to it first
        // this is temporary until the struct can be fully implemented
        let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
        let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
        let ncolors_ptr = get_from_memory::<u32>(entity_type_print + 0x038);
        let ncolors = get_from_memory::<u32>(ncolors_ptr);

        format!("\n\n[colorrep]\nncolors: {}\n", ncolors)
    }

    //  prints the [Configuration/Integers] section of the configuration
    fn print_config_integers(&self) -> String {
        format!("cIconZoom: {}\ncExpansionID: {}\ncMovable: {}\ncWalkable: {}\ncWalkableByTall: {}\ncRubbleable: {}\ncUseNumbersInName: {}\ncUsesRealShadows: {}\ncHasShadowImages: {}\ncForceShadowBlack: {}\ncDrawsLate: {}\ncHeight: {}\ncDepth: {}\ncHasUnderwaterSection: {}\ncIsTransient: {}\ncUsesPlacementCube: {}\ncShow: {}\ncHitThreshold: {}\ncAvoidEdges: {}\ncFootprintX: {}\ncFootprintY: {}\ncFootprintZ: {}\ncPlacementFootprintX: {}\ncPlacementFootprintY: {}\ncPlacementFootprintZ: {}\ncAvailableAtStartup: {}\n",
        self.icon_zoom as u32,
        self.expansion_id as u32,
        self.movable as u32,
        self.walkable as u32,
        self.walkable_by_tall as u32,
        self.rubbleable as u32,
        self.use_numbers_in_name as u32,
        self.uses_real_shadows as u32,
        self.has_shadow_images as u32,
        self.force_shadow_black as u32,
        self.draws_late as u32,
        self.height,
        self.depth,
        self.has_underwater_section as u32,
        self.is_transient as u32,
        self.uses_placement_cube as u32,
        self.show as u32,
        self.hit_threshold,
        self.avoid_edges as u32,
        self.footprintx,
        self.footprinty,
        self.footprintz,
        self.placement_footprintx,
        self.placement_footprinty,
        self.placement_footprintz,
        self.available_at_startup as u32,
        )
    }

    // prints misc details of the entity type
    fn print_details(&self) -> String {
        format!(
            "\n[Details]\n\nEntity Type Address: {:#x}\nType Name: {}\nCodename: {}\n",
            self as *const BFEntityType as u32,
            self.get_type_name(),
            self.get_codename(),
        )
    }
}

// ------------ ZTSceneryType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTSceneryType {
    pub bfentitytype: BFEntityType, // bytes: 0x100 - 0x000 = 0x100 = 256 bytes
    pub purchase_cost: f32,         // 0x100
    pub name_id: u32,               // 0x104
    pub help_id: u32,               // 0x108
    pub habitat: u32,               // 0x10C
    pub location: u32,              // 0x110
    pub era: u32,                   // 0x114
    pub max_food_units: u32,        // 0x118
    pub stink: bool,                // 0x11C
    pad3: [u8; 0x120 - 0x11D],      // ----------------------- padding: 3 bytes
    pub esthetic_weight: u32,       // 0x120
    pad4: [u8; 0x128 - 0x124],      // ----------------------- padding: 4 bytes
    pub selectable: bool,           // 0x128
    pub deletable: bool,            // 0x129
    pub foliage: bool,              // 0x12A
    pad6: [u8; 0x12D - 0x12B],      // ----------------------- padding: 2 bytes
    pub auto_rotate: bool,          // 0x12D
    pub land: bool,                 // 0x12E
    pub swims: bool,                // 0x12F
    pub underwater: bool,           // 0x130
    pub surface: bool,              // 0x131
    pub submerge: bool,             // 0x132
    pub only_swims: bool,           // 0x133
    pub needs_confirm: bool,        // 0x134
    pub gawk_only_from_front: bool, // 0x135
    pub dead_on_land: bool,         // 0x136
    pub dead_on_flat_water: bool,   // 0x137
    pub dead_underwater: bool,      // 0x138
    pub uses_tree_rubble: bool,     // 0x139
    pub forces_scenery_rubble: bool, // 0x13A
    pub blocks_los: bool,           // 0x13B
    pad7: [u8; 0x168 - 0x13C],      // ----------------------- padding: 51 bytes
}

impl ZTSceneryType {
    pub fn new(address: u32) -> Option<&'static mut ZTSceneryType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTSceneryType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn get_info_image_name(&self) -> String {
        let obj_ptr = self as *const ZTSceneryType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x14C))
    }

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cPurchaseCost" {
            self.purchase_cost = value.parse::<f32>().unwrap();
            Ok(format!("Set Purchase Cost to {}", self.purchase_cost))
        } else if config == "-cNameID" {
            self.name_id = value.parse::<u32>().unwrap();
            Ok(format!("Set Name ID to {}", self.name_id))
        } else if config == "-cHelpID" {
            self.help_id = value.parse::<u32>().unwrap();
            Ok(format!("Set Help ID to {}", self.help_id))
        } else if config == "-cHabitat" {
            self.habitat = value.parse::<u32>().unwrap();
            Ok(format!("Set Habitat to {}", self.habitat))
        } else if config == "-cLocation" {
            self.location = value.parse::<u32>().unwrap();
            Ok(format!("Set Location to {}", self.location))
        } else if config == "-cEra" {
            self.era = value.parse::<u32>().unwrap();
            Ok(format!("Set Era to {}", self.era))
        } else if config == "-cMaxFoodUnits" {
            self.max_food_units = value.parse::<u32>().unwrap();
            Ok(format!("Set Max Food Units to {}", self.max_food_units))
        } else if config == "-cStink" {
            self.stink = value.parse::<bool>().unwrap();
            Ok(format!("Set Stink to {}", self.stink))
        } else if config == "-cEstheticWeight" {
            self.esthetic_weight = value.parse::<u32>().unwrap();
            Ok(format!("Set Esthetic Weight to {}", self.esthetic_weight))
        } else if config == "-cSelectable" {
            self.selectable = value.parse::<bool>().unwrap();
            Ok(format!("Set Selectable to {}", self.selectable))
        } else if config == "-cDeletable" {
            self.deletable = value.parse::<bool>().unwrap();
            Ok(format!("Set Deletable to {}", self.deletable))
        } else if config == "-cFoliage" {
            self.foliage = value.parse::<bool>().unwrap();
            Ok(format!("Set Foliage to {}", self.foliage))
        } else if config == "-cAutoRotate" {
            self.auto_rotate = value.parse::<bool>().unwrap();
            Ok(format!("Set Auto Rotate to {}", self.auto_rotate))
        } else if config == "-cLand" {
            self.land = value.parse::<bool>().unwrap();
            Ok(format!("Set Land to {}", self.land))
        } else if config == "-cSwims" {
            self.swims = value.parse::<bool>().unwrap();
            Ok(format!("Set Swims to {}", self.swims))
        } else if config == "-cUnderwater" {
            self.underwater = value.parse::<bool>().unwrap();
            Ok(format!("Set Underwater to {}", self.underwater))
        } else if config == "-cSurface" {
            self.surface = value.parse::<bool>().unwrap();
            Ok(format!("Set Surface to {}", self.surface))
        } else if config == "-cSubmerge" {
            self.submerge = value.parse::<bool>().unwrap();
            Ok(format!("Set Submerge to {}", self.submerge))
        } else if config == "-cOnlySwims" {
            self.only_swims = value.parse::<bool>().unwrap();
            Ok(format!("Set Only Swims to {}", self.only_swims))
        } else if config == "-cNeedsConfirm" {
            self.needs_confirm = value.parse::<bool>().unwrap();
            Ok(format!("Set Needs Confirm to {}", self.needs_confirm))
        } else if config == "-cGawkOnlyFromFront" {
            self.gawk_only_from_front = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set Gawk Only From Front to {}",
                self.gawk_only_from_front
            ))
        } else if config == "-cDeadOnLand" {
            self.dead_on_land = value.parse::<bool>().unwrap();
            Ok(format!("Set Dead On Land to {}", self.dead_on_land))
        } else if config == "-cDeadOnFlatWater" {
            self.dead_on_flat_water = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set Dead On Flat Water to {}",
                self.dead_on_flat_water
            ))
        } else if config == "-cDeadUnderwater" {
            self.dead_underwater = value.parse::<bool>().unwrap();
            Ok(format!("Set Dead Underwater to {}", self.dead_underwater))
        } else if config == "-cUsesTreeRubble" {
            self.uses_tree_rubble = value.parse::<bool>().unwrap();
            Ok(format!("Set Uses Tree Rubble to {}", self.uses_tree_rubble))
        } else if config == "-cForcesSceneryRubble" {
            self.forces_scenery_rubble = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set Forces Scenery Rubble to {}",
                self.forces_scenery_rubble
            ))
        } else if config == "-cBlocksLOS" {
            self.blocks_los = value.parse::<bool>().unwrap();
            Ok(format!("Set Blocks LOS to {}", self.blocks_los))
        } else {
            Err("Invalid configuration option")
        }
    }

    fn print_config_integers(&self) -> String {
        format!("cPurchaseCost: {}\ncNameID: {}\ncHelpID: {}\ncHabitat: {}\ncLocation: {}\ncEra: {}\ncMaxFoodUnits: {}\ncStink: {}\ncEstheticWeight: {}\ncSelectable: {}\ncDeletable: {}\ncFoliage: {}\ncAutoRotate: {}\ncLand: {}\ncSwims: {}\ncUnderwater: {}\ncSurface: {}\ncSubmerge: {}\ncOnlySwims: {}\ncNeedsConfirm: {}\ncGawkOnlyFromFront: {}\ncDeadOnLand: {}\ncDeadOnFlatWater: {}\ncDeadUnderwater: {}\ncUsesTreeRubble: {}\ncForcesSceneryRubble: {}\ncBlocksLOS: {}\n",
        self.purchase_cost,
        self.name_id,
        self.help_id,
        self.habitat,
        self.location,
        self.era,
        self.max_food_units,
        self.stink as u32,
        self.esthetic_weight,
        self.selectable as u32,
        self.deletable as u32,
        self.foliage as u32,
        self.auto_rotate as u32,
        self.land as u32,
        self.swims as u32,
        self.underwater as u32,
        self.surface as u32,
        self.submerge as u32,
        self.only_swims as u32,
        self.needs_confirm as u32,
        self.gawk_only_from_front as u32,
        self.dead_on_land as u32,
        self.dead_on_flat_water as u32,
        self.dead_underwater as u32,
        self.uses_tree_rubble as u32,
        self.forces_scenery_rubble as u32,
        self.blocks_los as u32,
        )
    }
}

impl Deref for ZTSceneryType {
    type Target = BFEntityType;
    fn deref(&self) -> &Self::Target {
        &self.bfentitytype
    }
}

// ------------ ZTBuildingType, Implementation, and Related Functions ------------ //
#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTBuildingType {
    pub ztscenerytype: ZTSceneryType, // bytes: 0x168 - 0x000 = 0x16C = 364 bytes
    pad0: [u8; 0x16C - 0x168],        // -------------------------- padding: 4 bytes
    pub i_capacity: i32,              // 0x16C
    pub toy_satisfaction: i32,        // 0x170
    pub time_inside: i32,             // 0x174
    pub default_cost: f32,            // 0x178
    pub low_cost: f32,                // 0x17C
    pub med_cost: f32,                // 0x180
    pub high_cost: f32,               // 0x184
    pub price_factor: f32,            // 0x188
    pub upkeep: f32,                  // 0x18C
    pad1: [u8; 0x194 - 0x190],        // -------------------------- padding: 4 bytes
    pub hide_user: bool,              // 0x194
    pub set_letter_facing: bool,      // 0x195
    pub draw_user: bool,              // 0x196
    pub hide_cost_change: bool,       // 0x197
    pub hide_commerce_info: bool,     // 0x198
    pub hide_regular_info: bool,      // 0x199
    pub holds_onto_user: bool,        // 0x19A
    pub user_tracker: bool,           // 0x19B
    pub idler: bool,                  // 0x19C
    pub exhibit_viewer: bool,         // 0x19D
    pad2: [u8; 0x1A0 - 0x19E],        // -------------------------- padding: 2 bytes
    pub alternate_panel_title: u32,   // 0x1A0
    pub direct_entrance: bool,        // 0x1A4
    pub hide_building: bool,          // 0x1A5
    pub user_stays_outside: bool,     // 0x1A6
    pub user_teleports_inside: bool,  // 0x1A7
    pub user_uses_exit: bool,         // 0x1A8
    pub user_uses_entrance_as_emergency_exit: bool, // 0x1A9
    pad3: [u8; 0x1B8 - 0x1AA],        // -------------------------- padding: 9 bytes
    pub adult_change: i32,            // 0x1B8
    pub child_change: i32,            // 0x1BC
    pub hunger_change: i32,           // 0x1C0
    pub thirst_change: i32,           // 0x1C4
    pub bathroom_change: i32,         // 0x1C8
    pub energy_change: i32,           // 0x1CC
}

impl ZTBuildingType {
    fn new(address: u32) -> Option<&'static mut ZTBuildingType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTBuildingType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    // sets the configuration of the building type in the console
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cCapacity" {
            self.i_capacity = value.parse::<i32>().unwrap();
            Ok(format!("Set Capacity to {}", self.i_capacity))
        } else if config == "-cToySatisfaction" {
            self.toy_satisfaction = value.parse::<i32>().unwrap();
            Ok(format!("Set Toy Satisfaction to {}", self.toy_satisfaction))
        } else if config == "-cTimeInside" {
            self.time_inside = value.parse::<i32>().unwrap();
            Ok(format!("Set Time Inside to {}", self.time_inside))
        } else if config == "-cDefaultCost" {
            self.default_cost = value.parse::<f32>().unwrap();
            Ok(format!("Set Default Cost to {}", self.default_cost))
        } else if config == "-cLowCost" {
            self.low_cost = value.parse::<f32>().unwrap();
            Ok(format!("Set Low Cost to {}", self.low_cost))
        } else if config == "-cMedCost" {
            self.med_cost = value.parse::<f32>().unwrap();
            Ok(format!("Set Med Cost to {}", self.med_cost))
        } else if config == "-cHighCost" {
            self.high_cost = value.parse::<f32>().unwrap();
            Ok(format!("Set High Cost to {}", self.high_cost))
        } else if config == "-cPriceFactor" {
            self.price_factor = value.parse::<f32>().unwrap();
            Ok(format!("Set Price Factor to {}", self.price_factor))
        } else if config == "-cUpkeep" {
            self.upkeep = value.parse::<f32>().unwrap();
            Ok(format!("Set Upkeep to {}", self.upkeep))
        } else if config == "-cHideUser" {
            self.hide_user = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide User to {}", self.hide_user))
        } else if config == "-cSetLetterFacing" {
            self.set_letter_facing = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set Set Letter Facing to {}",
                self.set_letter_facing
            ))
        } else if config == "-cDrawUser" {
            self.draw_user = value.parse::<bool>().unwrap();
            Ok(format!("Set Draw User to {}", self.draw_user))
        } else if config == "-cHideCostChange" {
            self.hide_cost_change = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide Cost Change to {}", self.hide_cost_change))
        } else if config == "-cHideCommerceInfo" {
            self.hide_commerce_info = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set Hide Commerce Info to {}",
                self.hide_commerce_info
            ))
        } else if config == "-cHideRegularInfo" {
            self.hide_regular_info = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set Hide Regular Info to {}",
                self.hide_regular_info
            ))
        } else if config == "-cHoldsOntoUser" {
            self.holds_onto_user = value.parse::<bool>().unwrap();
            Ok(format!("Set Holds Onto User to {}", self.holds_onto_user))
        } else if config == "-cUserTracker" {
            self.user_tracker = value.parse::<bool>().unwrap();
            Ok(format!("Set User Tracker to {}", self.user_tracker))
        } else if config == "-cIdler" {
            self.idler = value.parse::<bool>().unwrap();
            Ok(format!("Set Idler to {}", self.idler))
        } else if config == "-cExhibitViewer" {
            self.exhibit_viewer = value.parse::<bool>().unwrap();
            Ok(format!("Set Exhibit Viewer to {}", self.exhibit_viewer))
        } else if config == "-cAlternatePanelTitle" {
            self.alternate_panel_title = value.parse::<u32>().unwrap();
            Ok(format!(
                "Set Alternate Panel Title to {}",
                self.alternate_panel_title
            ))
        } else if config == "-cDirectEntrance" {
            self.direct_entrance = value.parse::<bool>().unwrap();
            Ok(format!("Set Direct Entrance to {}", self.direct_entrance))
        } else if config == "-cHideBuilding" {
            self.hide_building = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide Building to {}", self.hide_building))
        } else if config == "-cUserStaysOutside" {
            self.user_stays_outside = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set User Stays Outside to {}",
                self.user_stays_outside
            ))
        } else if config == "-cUserTeleportsInside" {
            self.user_teleports_inside = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set User Teleports Inside to {}",
                self.user_teleports_inside
            ))
        } else if config == "-cUserUsesExit" {
            self.user_uses_exit = value.parse::<bool>().unwrap();
            Ok(format!("Set User Uses Exit to {}", self.user_uses_exit))
        } else if config == "-cUserUsesEntranceAsEmergencyExit" {
            self.user_uses_entrance_as_emergency_exit = value.parse::<bool>().unwrap();
            Ok(format!(
                "Set User Uses Entrance As Emergency Exit to {}",
                self.user_uses_entrance_as_emergency_exit
            ))
        } else if config == "-cAdultChange" {
            self.adult_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Adult Change to {}", self.adult_change))
        } else if config == "-cChildChange" {
            self.child_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Child Change to {}", self.child_change))
        } else if config == "-cHungerChange" {
            self.hunger_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Hunger Change to {}", self.hunger_change))
        } else if config == "-cThirstChange" {
            self.thirst_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Thirst Change to {}", self.thirst_change))
        } else if config == "-cBathroomChange" {
            self.bathroom_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Bathroom Change to {}", self.bathroom_change))
        } else if config == "-cEnergyChange" {
            self.energy_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Energy Change to {}", self.energy_change))
        } else {
            Err("Invalid configuration option")
        }
    }

    // print [Configuration/Floats] section of the configuration
    fn print_config_floats(&self) -> String {
        format!("\n\n[Configuration/Floats]\n\ncDefaultCost: {:.2}\ncLowCost: {:.2}\ncMedCost: {:.2}\ncHighCost: {:.2}\ncPriceFactor: {:.2}\ncUpkeep: {:.2}\n",
        self.default_cost,
        self.low_cost,
        self.med_cost,
        self.high_cost,
        self.price_factor,
        self.upkeep,
        )
    }

    // prints the [Configuration/Integers] section of the configuration
    fn print_config_integers(&self) -> String {
        format!("cCapacity: {}\ncToySatisfaction: {}\ncTimeInside: {}\ncHideUser: {}\ncSetLetterFacing: {}\ncDrawUser: {}\ncHideCostChange: {}\ncHideCommerceInfo: {}\ncHideRegularInfo: {}\ncHoldsOntoUser: {}\ncUserTracker: {}\ncIdler: {}\ncExhibitViewer: {}\ncAlternatePanelTitle: {}\ncDirectEntrance: {}\ncHideBuilding: {}\ncUserStaysOutside: {}\ncUserTeleportsInside: {}\ncUserUsesExit: {}\ncUserUsesEntranceAsEmergencyExit: {}\ncAdultChange: {}\ncChildChange: {}\ncHungerChange: {}\ncThirstChange: {}\ncBathroomChange: {}\ncEnergyChange: {}\n",
        self.i_capacity,
        self.toy_satisfaction,
        self.time_inside,
        self.hide_user as u32,
        self.set_letter_facing as u32,
        self.draw_user as u32,
        self.hide_cost_change as u32,
        self.hide_commerce_info as u32,
        self.hide_regular_info as u32,
        self.holds_onto_user as u32,
        self.user_tracker as u32,
        self.idler as u32,
        self.exhibit_viewer as u32,
        self.alternate_panel_title,
        self.direct_entrance as u32,
        self.hide_building as u32,
        self.user_stays_outside as u32,
        self.user_teleports_inside as u32,
        self.user_uses_exit as u32,
        self.user_uses_entrance_as_emergency_exit as u32,
        self.adult_change,
        self.child_change,
        self.hunger_change,
        self.thirst_change,
        self.bathroom_change,
        self.energy_change,
        )
    }
}

impl Deref for ZTBuildingType {
    type Target = ZTSceneryType;
    fn deref(&self) -> &Self::Target {
        &self.ztscenerytype
    }
}

// ------------ ZTFenceType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTFenceType {
    pub ztscenerytype: ZTSceneryType, // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub strength: i32,                // 0x168
    pub life: i32,                    // 0x16C
    pub decayed_life: i32,            // 0x170
    pub decayed_delta: i32,           // 0x174
    pub break_sound_atten: i32,       // 0x178
    pub open_sound_atten: i32,        // 0x17C
    // break_sound: String, // 0x184
    // open_sound: String, // 0x188
    pad2: [u8; 0x194 - 0x180], // ----------------------- padding: 20 bytes
    pub see_through: bool,     // 0x194
    pub is_jumpable: bool,     // 0x195
    pub is_climbable: bool,    // 0x196
    pub indestructible: bool,  // 0x197
    pub is_electrified: bool,  // 0x198
    pub no_draw_water: bool,   // 0x199
}

impl ZTFenceType {
    pub fn new(address: u32) -> Option<&'static mut ZTFenceType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTFenceType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    fn get_break_sound(&self) -> String {
        let obj_ptr = self as *const ZTFenceType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x184))
    }

    fn get_open_sound(&self) -> String {
        let obj_ptr = self as *const ZTFenceType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x188))
    }

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cStrength" {
            self.strength = value.parse::<i32>().unwrap();
            Ok(format!("Set Strength to {}", self.strength))
        } else if config == "-cLife" {
            self.life = value.parse::<i32>().unwrap();
            Ok(format!("Set Life to {}", self.life))
        } else if config == "-cDecayedLife" {
            self.decayed_life = value.parse::<i32>().unwrap();
            Ok(format!("Set Decayed Life to {}", self.decayed_life))
        } else if config == "-cDecayedDelta" {
            self.decayed_delta = value.parse::<i32>().unwrap();
            Ok(format!("Set Decayed Delta to {}", self.decayed_delta))
        } else if config == "-cBreakSoundAtten" {
            self.break_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set Break Sound Atten to {}",
                self.break_sound_atten
            ))
        } else if config == "-cOpenSoundAtten" {
            self.open_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Open Sound Atten to {}", self.open_sound_atten))
        } else if config == "-cSeeThrough" {
            self.see_through = value.parse::<bool>().unwrap();
            Ok(format!("Set See Through to {}", self.see_through))
        } else if config == "-cIsJumpable" {
            self.is_jumpable = value.parse::<bool>().unwrap();
            Ok(format!("Set Is Jumpable to {}", self.is_jumpable))
        } else if config == "-cIsClimbable" {
            self.is_climbable = value.parse::<bool>().unwrap();
            Ok(format!("Set Is Climbable to {}", self.is_climbable))
        } else if config == "-cIndestructible" {
            self.indestructible = value.parse::<bool>().unwrap();
            Ok(format!("Set Indestructible to {}", self.indestructible))
        } else if config == "-cIsElectrified" {
            self.is_electrified = value.parse::<bool>().unwrap();
            Ok(format!("Set Is Electrified to {}", self.is_electrified))
        } else if config == "-cNoDrawWater" {
            self.no_draw_water = value.parse::<bool>().unwrap();
            Ok(format!("Set No Draw Water to {}", self.no_draw_water))
        } else {
            Err("Invalid configuration option")
        }
    }

    fn print_config_integers(&self) -> String {
        format!("cStrength: {}\ncLife: {}\ncDecayedLife: {}\ncDecayedDelta: {}\ncBreakSoundAtten: {}\ncOpenSoundAtten: {}\ncSeeThrough: {}\ncIsJumpable: {}\ncIsClimbable: {}\ncIndestructible: {}\ncIsElectrified: {}\ncNoDrawWater: {}\n", // cBreakSound: {}\ncOpenSound: {}\n",
        self.strength,
        self.life,
        self.decayed_life,
        self.decayed_delta,
        self.break_sound_atten,
        self.open_sound_atten,
        self.see_through as u32,
        self.is_jumpable as u32,
        self.is_climbable as u32,
        self.indestructible as u32,
        self.is_electrified as u32,
        self.no_draw_water as u32,
        // self.get_break_sound(),
        // self.get_open_sound(), // TODO: fix this
        )
    }
}

impl Deref for ZTFenceType {
    type Target = ZTSceneryType;
    fn deref(&self) -> &Self::Target {
        &self.ztscenerytype
    }
}

// ------------ ZTTankWallType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTTankWallType {
    pub ztfencetype: ZTFenceType, // bytes: 0x19C - 0x168 = 0x34 = 52 bytes
    // pub portal_open_sound: u32, // 0x19C
    // pub portal_close_sound: u32, // 0x1A0
    pub portal_open_sound_atten: i32,  // 0x1A4
    pub portal_close_sound_atten: i32, // 0x1A8
}

impl ZTTankWallType {
    pub fn new(address: u32) -> Option<&'static mut ZTTankWallType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTTankWallType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    fn get_portal_open_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankWallType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x1A4))
    }

    fn get_portal_close_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankWallType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x1B0))
    }

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        // if config == "-cPortalOpenSound" {
        //     self.portal_open_sound = value.parse::<u32>().unwrap();
        //     Ok(format!("Set Portal Open Sound to {}", self.portal_open_sound))
        // }
        // else if config == "-cPortalCloseSound" {
        //     self.portal_close_sound = value.parse::<u32>().unwrap();
        //     Ok(format!("Set Portal Close Sound to {}", self.portal_close_sound))
        // }
        if config == "-cPortalOpenSoundAtten" {
            self.portal_open_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set Portal Open Sound Atten to {}",
                self.portal_open_sound_atten
            ))
        } else if config == "-cPortalCloseSoundAtten" {
            self.portal_close_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set Portal Close Sound Atten to {}",
                self.portal_close_sound_atten
            ))
        } else {
            Err("Invalid configuration option")
        }
    }

    fn print_portal_sounds(&self) -> String {
        format!("\n\n[PortalSounds]\ncPortalOpenSound: {}\ncPortalCloseSound: {}\ncPortalOpenSoundAtten: {}\ncPortalCloseSoundAtten: {}\n\n",
        self.get_portal_open_sound(),
        self.portal_open_sound_atten,
        self.get_portal_close_sound(),
        self.portal_close_sound_atten,
        )
    }
}

impl Deref for ZTTankWallType {
    type Target = ZTFenceType;
    fn deref(&self) -> &Self::Target {
        &self.ztfencetype
    }
}

// ------------ ZTFoodType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTFoodType {
    pub ztscenerytype: ZTSceneryType, // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub keeper_food_type: u32,        // 0x168
}

impl ZTFoodType {
    pub fn new(address: u32) -> Option<&'static mut ZTFoodType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTFoodType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cKeeperFoodType" {
            self.keeper_food_type = value.parse::<u32>().unwrap();
            Ok(format!("Set Keeper Food Type to {}", self.keeper_food_type))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
        format!("cKeeperFoodType: {}\n", self.keeper_food_type,)
    }
}

impl Deref for ZTFoodType {
    type Target = ZTSceneryType;
    fn deref(&self) -> &Self::Target {
        &self.ztscenerytype
    }
}

// ------------ ZTTankeFilterType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTTankFilterType {
    pub ztscenerytype: ZTSceneryType, // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub starting_health: i32,         // 0x168
    pub decayed_health: i32,          // 0x16C
    pub decay_time: i32,              // 0x170
    pub filter_delay: i32,            // 0x174
    pub filter_upkeep: i32,           // 0x178
    pub filter_clean_amount: i32,     // 0x17C
    pub filter_decayed_clean_amount: i32, // 0x180
    // healthy_sound: String, // 0x184
    // decayed_sound: String, // 0x190
    pad1: [u8; 0x19C - 0x184], // ----------------------- padding: 24 bytes
    pub healthy_atten: i32,    // 0x19C
    pub decayed_atten: i32,    // 0x1A0
}

impl ZTTankFilterType {
    pub fn new(address: u32) -> Option<&'static mut ZTTankFilterType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTTankFilterType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    fn get_healthy_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankFilterType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x184))
    }

    fn get_decayed_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankFilterType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x190))
    }

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cStartingHealth" {
            self.starting_health = value.parse::<i32>().unwrap();
            Ok(format!("Set Starting Health to {}", self.starting_health))
        } else if config == "-cDecayedHealth" {
            self.decayed_health = value.parse::<i32>().unwrap();
            Ok(format!("Set Decayed Health to {}", self.decayed_health))
        } else if config == "-cDecayTime" {
            self.decay_time = value.parse::<i32>().unwrap();
            Ok(format!("Set Decay Time to {}", self.decay_time))
        } else if config == "-cFilterDelay" {
            self.filter_delay = value.parse::<i32>().unwrap();
            Ok(format!("Set Filter Delay to {}", self.filter_delay))
        } else if config == "-cFilterUpkeep" {
            self.filter_upkeep = value.parse::<i32>().unwrap();
            Ok(format!("Set Filter Upkeep to {}", self.filter_upkeep))
        } else if config == "-cFilterCleanAmount" {
            self.filter_clean_amount = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set Filter Clean Amount to {}",
                self.filter_clean_amount
            ))
        } else if config == "-cFilterDecayedCleanAmount" {
            self.filter_decayed_clean_amount = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set Filter Decayed Clean Amount to {}",
                self.filter_decayed_clean_amount
            ))
        } else if config == "-cHealthyAtten" {
            self.healthy_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Healthy Atten to {}", self.healthy_atten))
        } else if config == "-cDecayedAtten" {
            self.decayed_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Decayed Atten to {}", self.decayed_atten))
        } else {
            Err("Invalid configuration option")
        }
    }

    fn print_config_integers(&self) -> String {
        format!("cStartingHealth: {}\ncDecayedHealth: {}\ncDecayTime: {}\ncFilterDelay: {}\ncFilterUpkeep: {}\ncFilterCleanAmount: {}\ncFilterDecayedCleanAmount: {}\ncHealthyAtten: {}\ncDecayedAtten: {}\ncHealthySound: {}\ncDecayedSound: {}\n",
        self.starting_health,
        self.decayed_health,
        self.decay_time,
        self.filter_delay,
        self.filter_upkeep,
        self.filter_clean_amount,
        self.filter_decayed_clean_amount,
        self.healthy_atten,
        self.decayed_atten,
        self.get_healthy_sound(),
        self.get_decayed_sound(), // TODO: fix this
        )
    }

    fn print_filter_sounds(&self) -> String {
        format!("\n\n[FilterSounds]\n\ncHealthySound: {}\ncHealthyAtten: {}\ncDecayedSound: {}\ncDecayedAtten: {}\n\n",
        self.get_healthy_sound(),
        self.healthy_atten,
        self.get_decayed_sound(),
        self.decayed_atten
        )
    }
}

impl Deref for ZTTankFilterType {
    type Target = ZTSceneryType;
    fn deref(&self) -> &Self::Target {
        &self.ztscenerytype
    }
}

// ------------ ZTPathType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTPathType {
    ztscenerytype: ZTSceneryType, // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub material: u32,            // 0x168
                                  // TODO: missing Shapes structure in paths. Could not find.
}

impl ZTPathType {
    pub fn new(address: u32) -> Option<&'static mut ZTPathType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTPathType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cMaterial" {
            self.material = value.parse::<u32>().unwrap();
            Ok(format!("Set Material to {}", self.material))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
        format!("cMaterial: {}\n", self.material,)
    }
}

impl Deref for ZTPathType {
    type Target = ZTSceneryType;
    fn deref(&self) -> &Self::Target {
        &self.ztscenerytype
    }
}

// ------------ ZTRubbleType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct ZTRubbleType {
    ztscenerytype: ZTSceneryType, // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    // explosion_sound: String, // 0x168
    pad0: [u8; 0x16C - 0x168], // ----------------------- padding: 4 bytes
    pub explosion_sound_atten: i32, // 0x16C
}

impl ZTRubbleType {
    pub fn new(address: u32) -> Option<&'static mut ZTRubbleType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTRubbleType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    fn get_explosion_sound(&self) -> String {
        let obj_ptr = self as *const ZTRubbleType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x168))
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        // if config == "-cExplosionSound" {
        //     self.explosion_sound = value.parse::<String>().unwrap();
        //     Ok(format!("Set Explosion Sound to {}", self.explosion_sound))
        // }
        if config == "-cExplosionSoundAtten" {
            self.explosion_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!(
                "Set Explosion Sound Atten to {}",
                self.explosion_sound_atten
            ))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
        format!(
            "cExplosionSound: {}\ncExplosionSoundAtten: {}\n",
            self.get_explosion_sound(),
            self.explosion_sound_atten,
        )
    }
}

impl Deref for ZTRubbleType {
    type Target = ZTSceneryType;
    fn deref(&self) -> &Self::Target {
        &self.ztscenerytype
    }
}

// ------------ BFUnitType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct BFUnitType {
    bfentitytype: BFEntityType, // bytes: 0x100 - 0x000 = 0x100 = 256 bytes
    pub slow_rate: u32,          // 0x100
    pub medium_rate: u32,        // 0x104
    pub fast_rate: u32,          // 0x108
    pub slow_anim_speed: u16,    // 0x10C
    pub medium_anim_speed: u16,  // 0x10E
    pub fast_anim_speed: u16,    // 0x110
    pad0: [u8; 0x114 - 0x112],   // ----------------------- padding: 2 bytes
    pub min_height: u32,         // 0x114 <--- unsure if accurate
    pub max_height: u32,         // 0x118 <--- unsure if accurate
}

impl BFUnitType {
    pub fn new(address: u32) -> Option<&'static mut BFUnitType> {
        unsafe {
            let ptr = get_from_memory::<*mut BFUnitType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cSlowRate" {
            self.slow_rate = value.parse::<u32>().unwrap();
            Ok(format!("Set Slow Rate to {}", self.slow_rate))
        } else if config == "-cMediumRate" {
            self.medium_rate = value.parse::<u32>().unwrap();
            Ok(format!("Set Medium Rate to {}", self.medium_rate))
        } else if config == "-cFastRate" {
            self.fast_rate = value.parse::<u32>().unwrap();
            Ok(format!("Set Fast Rate to {}", self.fast_rate))
        } else if config == "-cSlowAnimSpeed" {
            self.slow_anim_speed = value.parse::<u16>().unwrap();
            Ok(format!("Set Slow Anim Speed to {}", self.slow_anim_speed))
        } else if config == "-cMediumAnimSpeed" {
            self.medium_anim_speed = value.parse::<u16>().unwrap();
            Ok(format!("Set Medium Anim Speed to {}", self.medium_anim_speed))
        } else if config == "-cFastAnimSpeed" {
            self.fast_anim_speed = value.parse::<u16>().unwrap();
            Ok(format!("Set Fast Anim Speed to {}", self.fast_anim_speed))
        } else if config == "-cMinHeight" {
            self.min_height = value.parse::<u32>().unwrap();
            Ok(format!("Set Min Height to {}", self.min_height))
        } else if config == "-cMaxHeight" {
            self.max_height = value.parse::<u32>().unwrap();
            Ok(format!("Set Max Height to {}", self.max_height))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
        format!("cSlowRate: {}\ncMediumRate: {}\ncFastRate: {}\ncSlowAnimSpeed: {}\ncMediumAnimSpeed: {}\ncFastAnimSpeed: {}\ncMinHeight: {}\ncMaxHeight: {}\n",
        self.slow_rate,
        self.medium_rate,
        self.fast_rate,
        self.slow_anim_speed,
        self.medium_anim_speed,
        self.fast_anim_speed,
        self.min_height,
        self.max_height,
        )
    }
}

impl Deref for BFUnitType {
    type Target = BFEntityType;
    fn deref(&self) -> &Self::Target {
        &self.bfentitytype
    }
}

// ------------ ZTUnitType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTUnitType {
    pub bfunit_type: BFUnitType, // bytes: 0x11C - 0x100 = 0x1C = 28 bytes
    pad0: [u8; 0x12C - 0x11C], // ----------------------- padding: 16 bytes
    pub purchase_cost: f32,      // 0x12C
    pub name_id: i32,            // 0x130
    pub help_id: i32,            // 0x134
    pad1: [u8; 0x150 - 0x138],  // ----------------------- padding: 24 bytes
    pub map_footprint: i32,      // 0x150
    pub slow_anim_speed_water: u16, // 0x154
    pub medium_anim_speed_water: u16, // 0x156
    pub fast_anim_speed_water: u16, // 0x158
    pad2: [u8; 0x17C - 0x15C],  // ----------------------- padding: 32 bytes
    // pub list_image_name: String,    // 0x168 TODO: fix offset for string getters in unittype
    pub swims: bool,             // 0x17C
    pub surface: bool,           // 0x17D
    pub underwater: bool,        // 0x17E
    pub only_underwater: bool,   // 0x17F
    pad3: [u8; 0x180 - 0x17F],  // ----------------------- padding: 1 byte
    pub skip_trick_happiness: u32, // 0x180 TODO: potentially not accurate
    pub skip_trick_chance: i32, // 0x184
}

impl ZTUnitType {
    pub fn new(address: u32) -> Option<&'static mut ZTUnitType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTUnitType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn get_list_name(&self) -> String {
        let obj_ptr = self as *const ZTUnitType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x168))
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cPurchaseCost" {
            self.purchase_cost = value.parse::<f32>().unwrap();
            Ok(format!("Set Purchase Cost to {}", self.purchase_cost))
        } else if config == "-cNameID" {
            self.name_id = value.parse::<i32>().unwrap();
            Ok(format!("Set Name ID to {}", self.name_id))
        } else if config == "-cHelpID" {
            self.help_id = value.parse::<i32>().unwrap();
            Ok(format!("Set Help ID to {}", self.help_id))
        } else if config == "-cMapFootprint" {
            self.map_footprint = value.parse::<i32>().unwrap();
            Ok(format!("Set Map Footprint to {}", self.map_footprint))
        } else if config == "-cSlowAnimSpeedWater" {
            self.slow_anim_speed_water = value.parse::<u16>().unwrap();
            Ok(format!("Set Slow Anim Speed Water to {}", self.slow_anim_speed_water))
        } else if config == "-cMediumAnimSpeedWater" {
            self.medium_anim_speed_water = value.parse::<u16>().unwrap();
            Ok(format!("Set Medium Anim Speed Water to {}", self.medium_anim_speed_water))
        } else if config == "-cFastAnimSpeedWater" {
            self.fast_anim_speed_water = value.parse::<u16>().unwrap();
            Ok(format!("Set Fast Anim Speed Water to {}", self.fast_anim_speed_water))
        } else if config == "-cSwims" {
            self.swims = value.parse::<bool>().unwrap();
            Ok(format!("Set Swims to {}", self.swims))
        } else if config == "-cSurface" {
            self.surface = value.parse::<bool>().unwrap();
            Ok(format!("Set Surface to {}", self.surface))
        } else if config == "-cUnderwater" {
            self.underwater = value.parse::<bool>().unwrap();
            Ok(format!("Set Underwater to {}", self.underwater))
        } else if config == "-cOnlyUnderwater" {
            self.only_underwater = value.parse::<bool>().unwrap();
            Ok(format!("Set Only Underwater to {}", self.only_underwater))
        } else if config == "-cSkipTrickHappiness" {
            self.skip_trick_happiness = value.parse::<u32>().unwrap();
            Ok(format!("Set Skip Trick Happiness to {}", self.skip_trick_happiness))
        } else if config == "-cSkipTrickChance" {
            self.skip_trick_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Skip Trick Chance to {}", self.skip_trick_chance))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
        format!("cPurchaseCost: {}\ncNameID: {}\ncHelpID: {}\ncMapFootprint: {}\ncSlowAnimSpeedWater: {}\ncMediumAnimSpeedWater: {}\ncFastAnimSpeedWater: {}\ncSwims: {}\ncSurface: {}\ncUnderwater: {}\ncOnlyUnderwater: {}\ncSkipTrickHappiness: {}\ncSkipTrickChance: {}\n",
        self.purchase_cost,
        self.name_id,
        self.help_id,
        self.map_footprint,
        self.slow_anim_speed_water,
        self.medium_anim_speed_water,
        self.fast_anim_speed_water,
        self.swims as u32,
        self.surface as u32,
        self.underwater as u32,
        self.only_underwater as u32,
        self.skip_trick_happiness as u32,
        self.skip_trick_chance as u32,
        )
    }
}

// ------------ ZTGuestType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTGuestType {
    pub ztunit_type: ZTUnitType, // bytes: 0x188 - 0x100 = 0x88 = 136 bytes
    pad00: [u8; 0x1B4 - 0x188], // ----------------------- padding: 44 bytes
    pub hunger_check: i32,       // 0x1B4
    pub thirsty_check: i32,      // 0x1B8
    pub bathroom_check: i32,     // 0x1BC
    pub leave_zoo_check: i32,    // 0x1C0
    pub buy_souvenir_check: i32, // 0x1C4
    pub energy_check: i32,       // 0x1C8
    pub chase_check: i32,        // 0x1CC
    pub trash_check: i32,        // 0x1D0
    pub like_animals_check: i32, // 0x1D4
    pub viewing_area_check: i32, // 0x1D8
    pub environment_effect_check: i32, // 0x1DC
    pub saw_animal_reset: i32,   // 0x1E0
    pad01: [u8; 0x1E8 - 0x1E4], // ----------------------- padding: 4 bytes
    pub initial_happiness: i32,  // 0x1E8
    pad02: [u8; 0x200 - 0x1EC], // ----------------------- padding: 20 bytes
    pub max_energy: i32,         // 0x200
    pad03: [u8; 0x210 - 0x204], // ----------------------- padding: 12 bytes
    pub energy_increment: i32,   // 0x210
    pub energy_threshold: i32,   // 0x214
    pub angry_energy_change: i32, // 0x218
    pub hunger_increment: i32,   // 0x21C
    pub hunger_threshold: i32,   // 0x220
    pub angry_food_change: i32, // 0x224
    pub preferred_food_change: i32, // 0x228
    pub thirst_increment: i32,   // 0x22C
    pub thirst_threshold: i32,   // 0x230
    pub angry_thirst_change: i32, // 0x234
    pub bathroom_increment: i32, // 0x238
    pub bathroom_threshold: i32, // 0x23C
    pub angry_bathroom_change: i32, // 0x240
    pub price_happy1_change: i32, // 0x244
    pub price_angry1_change: i32, // 0x248
    pub leave_chance_low: i32,   // 0x24C
    pub leave_chance_med: i32,   // 0x250
    pub leave_chance_high: i32,  // 0x254
    pub leave_chance_done: i32,  // 0x258
    pub buy_souvenir_chance_med: i32, // 0x25C
    pub buy_souvenir_chance_high: i32, // 0x260
    pub angry_trash_change: i32, // 0x264
    pub trash_in_tile_threshold: i32, // 0x268
    pub vandalized_objects_in_tile_threshold: i32, // 0x26C
    pub animal_in_row_change: i32, // 0x270
    pub different_species_change: i32, // 0x274
    pub different_species_threshold: i32, // 0x278
    pub sick_animal_change: i32, // 0x27C
    pub crowded_viewing_threshold: i32, // 0x280
    pub crowded_viewing_change: i32, // 0x284
    pub preferred_animal_change: i32, // 0x288
    pub happy_animal_change1: i32, // 0x28C
    pub happy_animal_change2: i32, // 0x290
    pub angry_animal_change1: i32, // 0x294
    pub angry_animal_change2: i32, // 0x298
    pub angry_animal_change3: i32, // 0x29C
    pub escaped_animal_change: i32, // 0x2A0
    pub object_esthetic_threshold: i32, // 0x2A4
    pub happy_esthetic_change: i32, // 0x2A8
    pub stand_and_eat_change: i32, // 0x2AC
    pub stink_threshold: i32,    // 0x2B0
    pub sick_chance: i32,        // 0x2B4
    pub sick_change: i32,        // 0x2B8
    pub mimic_chance: i32,       // 0x2BC
    pub test_fence_chance: i32,  // 0x2C0
    pub zap_happiness_hit: i32,  // 0x2C4
    pub tap_wall_chance: i32,    // 0x2C8
}

impl ZTGuestType {
    pub fn new(address: u32) -> Option<&'static mut ZTGuestType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTGuestType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cHungerCheck" {
            self.hunger_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Hunger Check to {}", self.hunger_check))
        } else if config == "-cThirstyCheck" {
            self.thirsty_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Thirsty Check to {}", self.thirsty_check))
        } else if config == "-cBathroomCheck" {
            self.bathroom_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Bathroom Check to {}", self.bathroom_check))
        } else if config == "-cLeaveZooCheck" {
            self.leave_zoo_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Leave Zoo Check to {}", self.leave_zoo_check))
        } else if config == "-cBuySouvenirCheck" {
            self.buy_souvenir_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Buy Souvenir Check to {}", self.buy_souvenir_check))
        } else if config == "-cEnergyCheck" {
            self.energy_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Energy Check to {}", self.energy_check))
        } else if config == "-cChaseCheck" {
            self.chase_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Chase Check to {}", self.chase_check))
        } else if config == "-cTrashCheck" {
            self.trash_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Trash Check to {}", self.trash_check))
        } else if config == "-cLikeAnimalsCheck" {
            self.like_animals_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Like Animals Check to {}", self.like_animals_check))
        } else if config == "-cViewingAreaCheck" {
            self.viewing_area_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Viewing Area Check to {}", self.viewing_area_check))
        } else if config == "-cEnvironmentEffectCheck" {
            self.environment_effect_check = value.parse::<i32>().unwrap();
            Ok(format!("Set Environment Effect Check to {}", self.environment_effect_check))
        } else if config == "-cSawAnimalReset" {
            self.saw_animal_reset = value.parse::<i32>().unwrap();
            Ok(format!("Set Saw Animal Reset to {}", self.saw_animal_reset))
        } else if config == "-cInitialHappiness" {
            self.initial_happiness = value.parse::<i32>().unwrap();
            Ok(format!("Set Initial Happiness to {}", self.initial_happiness))
        } else if config == "-cMaxEnergy" {
            self.max_energy = value.parse::<i32>().unwrap();
            Ok(format!("Set Max Energy to {}", self.max_energy))
        } else if config == "-cEnergyIncrement" {
            self.energy_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Energy Increment to {}", self.energy_increment))
        } else if config == "-cEnergyThreshold" {
            self.energy_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Energy Threshold to {}", self.energy_threshold))
        } else if config == "-cAngryEnergyChange" {
            self.angry_energy_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Energy Change to {}", self.angry_energy_change))
        } else if config == "-cHungerIncrement" {
            self.hunger_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Hunger Increment to {}", self.hunger_increment))
        } else if config == "-cHungerThreshold" {
            self.hunger_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Hunger Threshold to {}", self.hunger_threshold))
        } else if config == "-cAngryFoodChange" {
            self.angry_food_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Food Change to {}", self.angry_food_change))
        } else if config == "-cPreferredFoodChange" {
            self.preferred_food_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Preferred Food Change to {}", self.preferred_food_change))
        } else if config == "-cThirstIncrement" {
            self.thirst_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Thirst Increment to {}", self.thirst_increment))
        } else if config == "-cThirstThreshold" {
            self.thirst_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Thirst Threshold to {}", self.thirst_threshold))
        } else if config == "-cAngryThirstChange" {
            self.angry_thirst_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Thirst Change to {}", self.angry_thirst_change))
        } else if config == "-cBathroomIncrement" {
            self.bathroom_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Bathroom Increment to {}", self.bathroom_increment))
        } else if config == "-cBathroomThreshold" {
            self.bathroom_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Bathroom Threshold to {}", self.bathroom_threshold))
        } else if config == "-cAngryBathroomChange" {
            self.angry_bathroom_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Bathroom Change to {}", self.angry_bathroom_change))
        } else if config == "-cPriceHappy1Change" {
            self.price_happy1_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Price Happy1 Change to {}", self.price_happy1_change))
        } else if config == "-cPriceAngry1Change" {
            self.price_angry1_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Price Angry1 Change to {}", self.price_angry1_change))
        } else if config == "-cLeaveChanceLow" {
            self.leave_chance_low = value.parse::<i32>().unwrap();
            Ok(format!("Set Leave Chance Low to {}", self.leave_chance_low))
        } else if config == "-cLeaveChanceMed" {
            self.leave_chance_med = value.parse::<i32>().unwrap();
            Ok(format!("Set Leave Chance Med to {}", self.leave_chance_med))
        } else if config == "-cLeaveChanceHigh" {
            self.leave_chance_high = value.parse::<i32>().unwrap();
            Ok(format!("Set Leave Chance High to {}", self.leave_chance_high))
        } else if config == "-cLeaveChanceDone" {
            self.leave_chance_done = value.parse::<i32>().unwrap();
            Ok(format!("Set Leave Chance Done to {}", self.leave_chance_done))
        } else if config == "-cBuySouvenirChanceMed" {
            self.buy_souvenir_chance_med = value.parse::<i32>().unwrap();
            Ok(format!("Set Buy Souvenir Chance Med to {}", self.buy_souvenir_chance_med))
        } else if config == "-cBuySouvenirChanceHigh" {
            self.buy_souvenir_chance_high = value.parse::<i32>().unwrap();
            Ok(format!("Set Buy Souvenir Chance High to {}", self.buy_souvenir_chance_high))
        } else if config == "-cAngryTrashChange" {
            self.angry_trash_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Trash Change to {}", self.angry_trash_change))
        } else if config == "-cTrashInTileThreshold" {
            self.trash_in_tile_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Trash In Tile Threshold to {}", self.trash_in_tile_threshold))
        } else if config == "-cVandalizedObjectsInTileThreshold" {
            self.vandalized_objects_in_tile_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Vandalized Objects In Tile Threshold to {}", self.vandalized_objects_in_tile_threshold))
        } else if config == "-cAnimalInRowChange" {
            self.animal_in_row_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Animal In Row Change to {}", self.animal_in_row_change))
        } else if config == "-cDifferentSpeciesChange" {
            self.different_species_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Different Species Change to {}", self.different_species_change))
        } else if config == "-cDifferentSpeciesThreshold" {
            self.different_species_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Different Species Threshold to {}", self.different_species_threshold))
        } else if config == "-cSickAnimalChange" {
            self.sick_animal_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Animal Change to {}", self.sick_animal_change))
        } else if config == "-cCrowdedViewingThreshold" {
            self.crowded_viewing_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Crowded Viewing Threshold to {}", self.crowded_viewing_threshold))
        } else if config == "-cCrowdedViewingChange" {
            self.crowded_viewing_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Crowded Viewing Change to {}", self.crowded_viewing_change))
        } else if config == "-cPreferredAnimalChange" {
            self.preferred_animal_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Preferred Animal Change to {}", self.preferred_animal_change))
        } else if config == "-cHappyAnimalChange1" {
            self.happy_animal_change1 = value.parse::<i32>().unwrap();
            Ok(format!("Set Happy Animal Change1 to {}", self.happy_animal_change1))
        } else if config == "-cHappyAnimalChange2" {
            self.happy_animal_change2 = value.parse::<i32>().unwrap();
            Ok(format!("Set Happy Animal Change2 to {}", self.happy_animal_change2))
        } else if config == "-cAngryAnimalChange1" {
            self.angry_animal_change1 = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Animal Change1 to {}", self.angry_animal_change1))
        } else if config == "-cAngryAnimalChange2" {
            self.angry_animal_change2 = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Animal Change2 to {}", self.angry_animal_change2))
        } else if config == "-cAngryAnimalChange3" {
            self.angry_animal_change3 = value.parse::<i32>().unwrap();
            Ok(format!("Set Angry Animal Change3 to {}", self.angry_animal_change3))
        } else if config == "-cEscapedAnimalChange" {
            self.escaped_animal_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Escaped Animal Change to {}", self.escaped_animal_change))
        } else if config == "-cObjectEstheticThreshold" {
            self.object_esthetic_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Object Esthetic Threshold to {}", self.object_esthetic_threshold))
        } else if config == "-cHappyEstheticChange" {
            self.happy_esthetic_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Happy Esthetic Change to {}", self.happy_esthetic_change))
        } else if config == "-cStandAndEatChange" {
            self.stand_and_eat_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Stand And Eat Change to {}", self.stand_and_eat_change))
        } else if config == "-cStinkThreshold" {
            self.stink_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Stink Threshold to {}", self.stink_threshold))
        } else if config == "-cSickChance" {
            self.sick_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Chance to {}", self.sick_chance))
        } else if config == "-cSickChange" {
            self.sick_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Change to {}", self.sick_change))
        } else if config == "-cMimicChance" {
            self.mimic_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Mimic Chance to {}", self.mimic_chance))
        } else if config == "-cTestFenceChance" {
            self.test_fence_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Test Fence Chance to {}", self.test_fence_chance))
        } else if config == "-cZapHappinessHit" {
            self.zap_happiness_hit = value.parse::<i32>().unwrap();
            Ok(format!("Set Zap Happiness Hit to {}", self.zap_happiness_hit))
        } else if config == "-cTapWallChance" {
            self.tap_wall_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Tap Wall Chance to {}", self.tap_wall_chance))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
        format!("cHungerCheck: {}\ncThirstyCheck: {}\ncBathroomCheck: {}\ncLeaveZooCheck: {}\ncBuySouvenirCheck: {}\ncEnergyCheck: {}\ncChaseCheck: {}\ncTrashCheck: {}\ncLikeAnimalsCheck: {}\ncViewingAreaCheck: {}\ncEnvironmentEffectCheck: {}\ncSawAnimalReset: {}\ncInitialHappiness: {}\ncMaxEnergy: {}\ncEnergyIncrement: {}\ncEnergyThreshold: {}\ncAngryEnergyChange: {}\ncHungerIncrement: {}\ncHungerThreshold: {}\ncAngryFoodChange: {}\ncPreferredFoodChange: {}\ncThirstIncrement: {}\ncThirstThreshold: {}\ncAngryThirstChange: {}\ncBathroomIncrement: {}\ncBathroomThreshold: {}\ncAngryBathroomChange: {}\ncPriceHappy1Change: {}\ncPriceAngry1Change: {}\ncLeaveChanceLow: {}\ncLeaveChanceMed: {}\ncLeaveChanceHigh: {}\ncLeaveChanceDone: {}\ncBuySouvenirChanceMed: {}\ncBuySouvenirChanceHigh: {}\ncAngryTrashChange: {}\ncTrashInTileThreshold: {}\ncVandalizedObjectsInTileThreshold: {}\ncAnimalInRowChange: {}\ncDifferentSpeciesChange: {}\ncDifferentSpeciesThreshold: {}\ncSickAnimalChange: {}\ncCrowdedViewingThreshold: {}\ncCrowdedViewingChange: {}\ncPreferredAnimalChange: {}\ncHappyAnimalChange1: {}\ncHappyAnimalChange2: {}\ncAngryAnimalChange1: {}\ncAngryAnimalChange2: {}\ncAngryAnimalChange3: {}\ncEscapedAnimalChange: {}\ncObjectEstheticThreshold: {}\ncHappyEstheticChange: {}\ncStandAndEatChange: {}\ncStinkThreshold: {}\ncSickChance: {}\ncSickChange: {}\ncMimicChance: {}\ncTestFenceChance: {}\ncZapHappinessHit: {}\ncTapWallChance: {}\n",
        self.hunger_check,
        self.thirsty_check,
        self.bathroom_check,
        self.leave_zoo_check,
        self.buy_souvenir_check,
        self.energy_check,
        self.chase_check,
        self.trash_check,
        self.like_animals_check,
        self.viewing_area_check,
        self.environment_effect_check,
        self.saw_animal_reset,
        self.initial_happiness,
        self.max_energy,
        self.energy_increment,
        self.energy_threshold,
        self.angry_energy_change,
        self.hunger_increment,
        self.hunger_threshold,
        self.angry_food_change,
        self.preferred_food_change,
        self.thirst_increment,
        self.thirst_threshold,
        self.angry_thirst_change,
        self.bathroom_increment,
        self.bathroom_threshold,
        self.angry_bathroom_change,
        self.price_happy1_change,
        self.price_angry1_change,
        self.leave_chance_low,
        self.leave_chance_med,
        self.leave_chance_high,
        self.leave_chance_done,
        self.buy_souvenir_chance_med,
        self.buy_souvenir_chance_high,
        self.angry_trash_change,
        self.trash_in_tile_threshold,
        self.vandalized_objects_in_tile_threshold,
        self.animal_in_row_change,
        self.different_species_change,
        self.different_species_threshold,
        self.sick_animal_change,
        self.crowded_viewing_threshold,
        self.crowded_viewing_change,
        self.preferred_animal_change,
        self.happy_animal_change1,
        self.happy_animal_change2,
        self.angry_animal_change1,
        self.angry_animal_change2,
        self.angry_animal_change3,
        self.escaped_animal_change,
        self.object_esthetic_threshold,
        self.happy_esthetic_change,
        self.stand_and_eat_change,
        self.stink_threshold,
        self.sick_chance,
        self.sick_change,
        self.mimic_chance,
        self.test_fence_chance,
        self.zap_happiness_hit,
        self.tap_wall_chance,
        )
    }
}

impl Deref for ZTGuestType {
    type Target = ZTUnitType;
    fn deref(&self) -> &Self::Target {
        &self.ztunit_type
    }
}

// ------------ ZTAnimalType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTAnimalType {
    pub ztunit_type: ZTUnitType, // bytes: 0x188 - 0x100 = 0x88 = 136 bytes
    pad00: [u8; 0x1D8 - 0x188], // ----------------------- padding: 72 bytes
    box_footprint_x: i32, // 0x1D8
    box_footprint_y: i32, // 0x1DC
    box_footprint_z: i32, // 0x1E0
    family: i32, // 0x1E4
    genus: i32, // 0x1E8
    pad01: [u8; 0x1F0 - 0x1EC], // ----------------------- padding: 4 bytes
    habitat: i32, // 0x1F0
    location: i32, // 0x1F4
    era: i32, // 0x1F8
    breath_threshold: i32, // 0x1FC
    breath_increment: i32, // 0x200
    pad02: [u8; 0x20C - 0x204], // ----------------------- padding: 8 bytes
    hunger_threshold: i32, // 0x20C
    hungry_health_change: i32, // 0x210
    hunger_increment: i32, // 0x214
    food_unit_value: i32, // 0x218
    keeper_food_units_eaten: i32, // 0x21C
    needed_food: i32, // 0x220
    no_food_change: i32, // 0x224
    initial_happiness: i32, // 0x228
    pad04: [u8; 0x234 - 0x22C], // ----------------------- padding: 12 bytes
    max_hits: i32, // 0x234
    pad05: [u8; 0x23C - 0x238], // ----------------------- padding: 4 bytes
    pct_hits: i32, // 0x23C
    pad06: [u8; 0x248 - 0x240], // ----------------------- padding: 8 bytes
    max_energy: i32, // 0x248
    pad07: [u8; 0x250 - 0x24C], // ----------------------- padding: 4 bytes
    max_dirty: i32, // 0x250
    min_dirty: i32, // 0x254
    sick_change: i32, // 0x258
    other_animal_sick_change: i32, // 0x25C
    sick_chance: i32, // 0x260
    sick_random_chance: i32, // 0x264
    crowd: i32, // 0x268
    crowd_happiness_change: i32, // 0x26C
    zap_happiness_change: i32, // 0x270
    captivity: i32, // 0x274
    reproduction_chance: i32, // 0x278
    reproduction_interval: i32, // 0x27C
    mating_type: i32, // 0x280
    offspring: i32, // 0x284
    keeper_frequency: i32, // 0x288
    pad007: [u8; 0x290 - 0x28C], // ----------------------- padding: 4 bytes
    not_enough_keepers_change: i32, // 0x290
    social: i32, // 0x294
    habitat_size: i32, // 0x298
    number_animals_min: i32, // 0x29C
    number_animals_max: i32, // 0x2A0
    pad08: [u8; 0x2AC - 0x2A4], // ----------------------- padding: 8 bytes
    number_min_change: i32, // 0x2AC
    number_max_change: i32, // 0x2B0
    pad09: [u8; 0x2BC - 0x2B4], // ----------------------- padding: 8 bytes
    habitat_preference: i32, // 0x2BC
    pad10: [u8; 0x31C - 0x2C0], // ----------------------- padding: 92 bytes
    baby_born_change: i32, // 0x31C
    pad11: [u8; 0x320 - 0x320], // ----------------------- padding: 4 bytes
    energy_increment: i32, // 0x320
    energy_threshold: i32, // 0x324
    dirty_increment: i32, // 0x328
    dirty_threshold: i32, // 0x32C
    pad12: [u8; 0x330 - 0x330], // ----------------------- padding: 4 bytes
    sick_time: i32, // 0x330
    pad13: [u8; 0x344 - 0x334], // ----------------------- padding: 16 bytes
    baby_to_adult: i32, // 0x344
    pad14: [u8; 0x348 - 0x348], // ----------------------- padding: 4 bytes
    other_food: i32, // 0x348
    tree_pref: i32, // 0x34C
    rock_pref: i32, // 0x350
    space_pref: i32, // 0x354
    elevation_pref: i32, // 0x358
    depth_min: i32, // 0x35C
    depth_max: i32, // 0x360
    depth_change: i32, // 0x364
    salinity_change: i32, // 0x368
    salinity_health_change: i32, // 0x36C
    pad15: [u8; 0x378 - 0x370], // ----------------------- padding: 8 bytes
    happy_reproduce_threshold: i32, // 0x378
    pad16: [u8; 0x37C - 0x37C], // ----------------------- padding: 4 bytes
    building_use_chance: i32, // 0x37C
    no_mate_change: i32, // 0x380
    time_death: i32, // 0x384
    death_chance: i32, // 0x388
    dirt_chance: i32, // 0x38C
    water_needed: i32, // 0x390
    underwater_needed: i32, // 0x394
    land_needed: i32, // 0x398
    enter_water_chance: i32, // 0x39C
    enter_tank_chance: i32, // 0x3A0
    enter_land_chance: i32, // 0x3A4
    drink_water_chance: i32, // 0x3A8
    chase_animal_chance: i32, // 0x3AC
    climbs_cliffs: i32, // 0x3B0
    bash_strength: i32, // 0x3B4
    attractiveness: i32, // 0x3B8
    pad17: [u8; 0x3C8 - 0x3BC], // ----------------------- padding: 8 bytes
    keeper_food_type: i32, // 0x3C8
    is_climber: bool, // 0x3CC
    is_jumper: bool, // 0x3CD
    small_zoodoo: bool, // 0x3CE
    dino_zoodoo: bool, // 0x3CF
    giant_zoodoo: bool, // 0x3D0
    is_special_animal: bool, // 0x3D1
    need_shelter: bool, // 0x3D2
    need_toys: bool, // 0x3D3
    babies_attack: bool, // 0x3D4
}

impl ZTAnimalType {
    pub fn new(address: u32) -> Option<&'static mut ZTAnimalType> {
        unsafe {
            let ptr = get_from_memory::<*mut ZTAnimalType>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, &'static str> {
        if config == "-cBoxFootprintX" {
            self.box_footprint_x = value.parse::<i32>().unwrap();
            Ok(format!("Set Box Footprint X to {}", self.box_footprint_x))
        } else if config == "-cBoxFootprintY" {
            self.box_footprint_y = value.parse::<i32>().unwrap();
            Ok(format!("Set Box Footprint Y to {}", self.box_footprint_y))
        } else if config == "-cBoxFootprintZ" {
            self.box_footprint_z = value.parse::<i32>().unwrap();
            Ok(format!("Set Box Footprint Z to {}", self.box_footprint_z))
        } else if config == "-cFamily" {
            self.family = value.parse::<i32>().unwrap();
            Ok(format!("Set Family to {}", self.family))
        } else if config == "-cGenus" {
            self.genus = value.parse::<i32>().unwrap();
            Ok(format!("Set Genus to {}", self.genus))
        } else if config == "-cHabitat" {
            self.habitat = value.parse::<i32>().unwrap();
            Ok(format!("Set Habitat to {}", self.habitat))
        } else if config == "-cLocation" {
            self.location = value.parse::<i32>().unwrap();
            Ok(format!("Set Location to {}", self.location))
        } else if config == "-cEra" {
            self.era = value.parse::<i32>().unwrap();
            Ok(format!("Set Era to {}", self.era))
        } else if config == "-cBreathThreshold" {
            self.breath_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Breath Threshold to {}", self.breath_threshold))
        } else if config == "-cBreathIncrement" {
            self.breath_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Breath Increment to {}", self.breath_increment))
        } else if config == "-cHungerThreshold" {
            self.hunger_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Hunger Threshold to {}", self.hunger_threshold))
        } else if config == "-cHungryHealthChange" {
            self.hungry_health_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Hungry Health Change to {}", self.hungry_health_change))
        } else if config == "-cHungerIncrement" {
            self.hunger_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Hunger Increment to {}", self.hunger_increment))
        } else if config == "-cFoodUnitValue" {
            self.food_unit_value = value.parse::<i32>().unwrap();
            Ok(format!("Set Food Unit Value to {}", self.food_unit_value))
        } else if config == "-cKeeperFoodUnitsEaten" {
            self.keeper_food_units_eaten = value.parse::<i32>().unwrap();
            Ok(format!("Set Keeper Food Units Eaten to {}", self.keeper_food_units_eaten))
        } else if config == "-cNeededFood" {
            self.needed_food = value.parse::<i32>().unwrap();
            Ok(format!("Set Needed Food to {}", self.needed_food))
        } else if config == "-cNoFoodChange" {
            self.no_food_change = value.parse::<i32>().unwrap();
            Ok(format!("Set No Food Change to {}", self.no_food_change))
        } else if config == "-cInitialHappiness" {
            self.initial_happiness = value.parse::<i32>().unwrap();
            Ok(format!("Set Initial Happiness to {}", self.initial_happiness))
        } else if config == "-cMaxHits" {
            self.max_hits = value.parse::<i32>().unwrap();
            Ok(format!("Set Max Hits to {}", self.max_hits))
        } else if config == "-cPctHits" {
            self.pct_hits = value.parse::<i32>().unwrap();
            Ok(format!("Set Pct Hits to {}", self.pct_hits))
        } else if config == "-cMaxEnergy" {
            self.max_energy = value.parse::<i32>().unwrap();
            Ok(format!("Set Max Energy to {}", self.max_energy))
        } else if config == "-cMaxDirty" {
            self.max_dirty = value.parse::<i32>().unwrap();
            Ok(format!("Set Max Dirty to {}", self.max_dirty))
        } else if config == "-cMinDirty" {
            self.min_dirty = value.parse::<i32>().unwrap();
            Ok(format!("Set Min Dirty to {}", self.min_dirty))
        } else if config == "-cSickChange" {
            self.sick_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Change to {}", self.sick_change))
        } else if config == "-cOtherAnimalSickChange" {
            self.other_animal_sick_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Other Animal Sick Change to {}", self.other_animal_sick_change))
        } else if config == "-cSickChance" {
            self.sick_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Chance to {}", self.sick_chance))
        } else if config == "-cSickRandomChance" {
            self.sick_random_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Random Chance to {}", self.sick_random_chance))
        } else if config == "-cCrowd" {
            self.crowd = value.parse::<i32>().unwrap();
            Ok(format!("Set Crowd to {}", self.crowd))
        } else if config == "-cCrowdHappinessChange" {
            self.crowd_happiness_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Crowd Happiness Change to {}", self.crowd_happiness_change))
        } else if config == "-cZapHappinessChange" {
            self.zap_happiness_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Zap Happiness Change to {}", self.zap_happiness_change))
        } else if config == "-cCaptivity" {
            self.captivity = value.parse::<i32>().unwrap();
            Ok(format!("Set Captivity to {}", self.captivity))
        } else if config == "-cReproductionChance" {
            self.reproduction_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Reproduction Chance to {}", self.reproduction_chance))
        } else if config == "-cReproductionInterval" {
            self.reproduction_interval = value.parse::<i32>().unwrap();
            Ok(format!("Set Reproduction Interval to {}", self.reproduction_interval))
        } else if config == "-cMatingType" {
            self.mating_type = value.parse::<i32>().unwrap();
            Ok(format!("Set Mating Type to {}", self.mating_type))
        } else if config == "-cOffspring" {
            self.offspring = value.parse::<i32>().unwrap();
            Ok(format!("Set Offspring to {}", self.offspring))
        } else if config == "-cKeeperFrequency" {
            self.keeper_frequency = value.parse::<i32>().unwrap();
            Ok(format!("Set Keeper Frequency to {}", self.keeper_frequency))
        } else if config == "-cNotEnoughKeepersChange" {
            self.not_enough_keepers_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Not Enough Keepers Change to {}", self.not_enough_keepers_change))
        } else if config == "-cSocial" {
            self.social = value.parse::<i32>().unwrap();
            Ok(format!("Set Social to {}", self.social))
        } else if config == "-cHabitatSize" {
            self.habitat_size = value.parse::<i32>().unwrap();
            Ok(format!("Set Habitat Size to {}", self.habitat_size))
        } else if config == "-cNumberAnimalsMin" {
            self.number_animals_min = value.parse::<i32>().unwrap();
            Ok(format!("Set Number Animals Min to {}", self.number_animals_min))
        } else if config == "-cNumberAnimalsMax" {
            self.number_animals_max = value.parse::<i32>().unwrap();
            Ok(format!("Set Number Animals Max to {}", self.number_animals_max))
        } else if config == "-cNumberMinChange" {
            self.number_min_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Number Min Change to {}", self.number_min_change))
        } else if config == "-cNumberMaxChange" {
            self.number_max_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Number Max Change to {}", self.number_max_change))
        } else if config == "-cHabitatPreference" {
            self.habitat_preference = value.parse::<i32>().unwrap();
            Ok(format!("Set Habitat Preference to {}", self.habitat_preference))
        } else if config == "-cBabyBornChange" {
            self.baby_born_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Baby Born Change to {}", self.baby_born_change))
        } else if config == "-cEnergyIncrement" {
            self.energy_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Energy Increment to {}", self.energy_increment))
        } else if config == "-cEnergyThreshold" {
            self.energy_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Energy Threshold to {}", self.energy_threshold))
        } else if config == "-cDirtyIncrement" {
            self.dirty_increment = value.parse::<i32>().unwrap();
            Ok(format!("Set Dirty Increment to {}", self.dirty_increment))
        } else if config == "-cDirtyThreshold" {
            self.dirty_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Dirty Threshold to {}", self.dirty_threshold))
        } else if config == "-cSickTime" {
            self.sick_time = value.parse::<i32>().unwrap();
            Ok(format!("Set Sick Time to {}", self.sick_time))
        } else if config == "-cBabyToAdult" {
            self.baby_to_adult = value.parse::<i32>().unwrap();
            Ok(format!("Set Baby To Adult to {}", self.baby_to_adult))
        } else if config == "-cOtherFood" {
            self.other_food = value.parse::<i32>().unwrap();
            Ok(format!("Set Other Food to {}", self.other_food))
        } else if config == "-cTreePref" {
            self.tree_pref = value.parse::<i32>().unwrap();
            Ok(format!("Set Tree Pref to {}", self.tree_pref))
        } else if config == "-cRockPref" {
            self.rock_pref = value.parse::<i32>().unwrap();
            Ok(format!("Set Rock Pref to {}", self.rock_pref))
        } else if config == "-cSpacePref" {
            self.space_pref = value.parse::<i32>().unwrap();
            Ok(format!("Set Space Pref to {}", self.space_pref))
        } else if config == "-cElevationPref" {
            self.elevation_pref = value.parse::<i32>().unwrap();
            Ok(format!("Set Elevation Pref to {}", self.elevation_pref))
        } else if config == "-cDepthMin" {
            self.depth_min = value.parse::<i32>().unwrap();
            Ok(format!("Set Depth Min to {}", self.depth_min))
        } else if config == "-cDepthMax" {
            self.depth_max = value.parse::<i32>().unwrap();
            Ok(format!("Set Depth Max to {}", self.depth_max))
        } else if config == "-cDepthChange" {
            self.depth_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Depth Change to {}", self.depth_change))
        } else if config == "-cSalinityChange" {
            self.salinity_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Salinity Change to {}", self.salinity_change))
        } else if config == "-cSalinityHealthChange" {
            self.salinity_health_change = value.parse::<i32>().unwrap();
            Ok(format!("Set Salinity Health Change to {}", self.salinity_health_change))
        } else if config == "-cHappyReproduceThreshold" {
            self.happy_reproduce_threshold = value.parse::<i32>().unwrap();
            Ok(format!("Set Happy Reproduce Threshold to {}", self.happy_reproduce_threshold))
        } else if config == "-cBuildingUseChance" {
            self.building_use_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Building Use Chance to {}", self.building_use_chance))
        } else if config == "-cNoMateChange" {
            self.no_mate_change = value.parse::<i32>().unwrap();
            Ok(format!("Set No Mate Change to {}", self.no_mate_change))
        } else if config == "-cTimeDeath" {
            self.time_death = value.parse::<i32>().unwrap();
            Ok(format!("Set Time Death to {}", self.time_death))
        } else if config == "-cDeathChance" {
            self.death_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Death Chance to {}", self.death_chance))
        } else if config == "-cDirtChance" {
            self.dirt_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Dirt Chance to {}", self.dirt_chance))
        } else if config == "-cWaterNeeded" {
            self.water_needed = value.parse::<i32>().unwrap();
            Ok(format!("Set Water Needed to {}", self.water_needed))
        } else if config == "-cUnderwaterNeeded" {
            self.underwater_needed = value.parse::<i32>().unwrap();
            Ok(format!("Set Underwater Needed to {}", self.underwater_needed))
        } else if config == "-cLandNeeded" {
            self.land_needed = value.parse::<i32>().unwrap();
            Ok(format!("Set Land Needed to {}", self.land_needed))
        } else if config == "-cEnterWaterChance" {
            self.enter_water_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Enter Water Chance to {}", self.enter_water_chance))
        } else if config == "-cEnterTankChance" {
            self.enter_tank_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Enter Tank Chance to {}", self.enter_tank_chance))
        } else if config == "-cEnterLandChance" {
            self.enter_land_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Enter Land Chance to {}", self.enter_land_chance))
        } else if config == "-cDrinkWaterChance" {
            self.drink_water_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Drink Water Chance to {}", self.drink_water_chance))
        } else if config == "-cChaseAnimalChance" {
            self.chase_animal_chance = value.parse::<i32>().unwrap();
            Ok(format!("Set Chase Animal Chance to {}", self.chase_animal_chance))
        } else if config == "-cClimbsCliffs" {
            self.climbs_cliffs = value.parse::<i32>().unwrap();
            Ok(format!("Set Climbs Cliffs to {}", self.climbs_cliffs))
        } else if config == "-cBashStrength" {
            self.bash_strength = value.parse::<i32>().unwrap();
            Ok(format!("Set Bash Strength to {}", self.bash_strength))
        } else if config == "-cAttractiveness" {
            self.attractiveness = value.parse::<i32>().unwrap();
            Ok(format!("Set Attractiveness to {}", self.attractiveness))
        } else if config == "-cKeeperFoodType" {
            self.keeper_food_type = value.parse::<i32>().unwrap();
            Ok(format!("Set Keeper Food Type to {}", self.keeper_food_type))
        } else if config == "-cIsClimber" {
            self.is_climber = value.parse::<bool>().unwrap();
            Ok(format!("Set Is Climber to {}", self.is_climber))
        } else if config == "-cIsJumper" {
            self.is_jumper = value.parse::<bool>().unwrap();
            Ok(format!("Set Is Jumper to {}", self.is_jumper))
        } else if config == "-cSmallZoodoo" {
            self.small_zoodoo = value.parse::<i32>().unwrap();
            Ok(format!("Set Small Zoodoo to {}", self.small_zoodoo))
        } else if config == "-cDinoZoodoo" {
            self.dino_zoodoo = value.parse::<i32>().unwrap();
            Ok(format!("Set Dino Zoodoo to {}", self.dino_zoodoo))
        } else if config == "-cGiantZoodoo" {
            self.giant_zoodoo = value.parse::<i32>().unwrap();
            Ok(format!("Set Giant Zoodoo to {}", self.giant_zoodoo))
        } else if config == "-cIsSpecialAnimal" {
            self.is_special_animal = value.parse::<bool>().unwrap();
            Ok(format!("Set Is Special Animal to {}", self.is_special_animal))
        } else if config == "-cNeedShelter" {
            self.need_shelter = value.parse::<bool>().unwrap();
            Ok(format!("Set Need Shelter to {}", self.need_shelter))
        } else if config == "-cNeedToys" {
            self.need_toys = value.parse::<bool>().unwrap();
            Ok(format!("Set Need Toys to {}", self.need_toys))
        } else if config == "-cBabiesAttack" {
            self.babies_attack = value.parse::<bool>().unwrap();
            Ok(format!("Set Babies Attack to {}", self.babies_attack))
        } else {
            Err("Invalid configuration option")
        }
    }

    pub fn print_config_integers(&self) -> String {
    format!(
        "Box Footprint X: {}\nBox Footprint Y: {}\nBox Footprint Z: {}\nFamily: {}\nGenus: {}\nHabitat: {}\nLocation: {}\nEra: {}\nBreath Threshold: {}\nBreath Increment: {}\nHunger Threshold: {}\nHungry Health Change: {}\nHunger Increment: {}\nFood Unit Value: {}\nKeeper Food Units Eaten: {}\nNeeded Food: {}\nNo Food Change: {}\nInitial Happiness: {}\nMax Hits: {}\nPct Hits: {}\nMax Energy: {}\nMax Dirty: {}\nMin Dirty: {}\nSick Change: {}\nOther Animal Sick Change: {}\nSick Chance: {}\nSick Random Chance: {}\nCrowd: {}\nCrowd Happiness Change: {}\nZap Happiness Change: {}\nCaptivity: {}\nReproduction Chance: {}\nReproduction Interval: {}\nMating Type: {}\nOffspring: {}\nKeeper Frequency: {}\nNot Enough Keepers Change: {}\nSocial: {}\nHabitat Size: {}\nNumber Animals Min: {}\nNumber Animals Max: {}\nNumber Min Change: {}\nNumber Max Change: {}\nHabitat Preference: {}\nBaby Born Change: {}\nEnergy Increment: {}\nEnergy Threshold: {}\nDirty Increment: {}\nDirty Threshold: {}\nSick Time: {}\nBaby To Adult: {}\nOther Food: {}\nTree Pref: {}\nRock Pref: {}\nSpace Pref: {}\nElevation Pref: {}\nDepth Min: {}\nDepth Max: {}\nDepth Change: {}\nSalinity Change: {}\nSalinity Health Change: {}\nHappy Reproduce Threshold: {}\nBuilding Use Chance: {}\nNo Mate Change: {}\nTime Death: {}\nDeath Chance: {}\nDirt Chance: {}\nWater Needed: {}\nUnderwater Needed: {}\nLand Needed: {}\nEnter Water Chance: {}\nEnter Tank Chance: {}\nEnter Land Chance: {}\nDrink Water Chance: {}\nChase Animal Chance: {}\nClimbs Cliffs: {}\nBash Strength: {}\nAttractiveness: {}\nKeeper Food Type: {}\nIs Climber: {}\nIs Jumper: {}\nSmall Zoodoo: {}\nDino Zoodoo: {}\nGiant Zoodoo: {}\nIs Special Animal: {}\
        Need Shelter: {}\nNeed Toys: {}\nBabies Attack: {}\n",
        self.box_footprint_x,
        self.box_footprint_y,
        self.box_footprint_z,
        self.family,
        self.genus,
        self.habitat,
        self.location,
        self.era,
        self.breath_threshold,
        self.breath_increment,
        self.hunger_threshold,
        self.hungry_health_change,
        self.hunger_increment,
        self.food_unit_value,
        self.keeper_food_units_eaten,
        self.needed_food,
        self.no_food_change,
        self.initial_happiness,
        self.max_hits,
        self.pct_hits,
        self.max_energy,
        self.max_dirty,
        self.min_dirty,
        self.sick_change,
        self.other_animal_sick_change,
        self.sick_chance,
        self.sick_random_chance,
        self.crowd,
        self.crowd_happiness_change,
        self.zap_happiness_change,
        self.captivity,
        self.reproduction_chance,
        self.reproduction_interval,
        self.mating_type,
        self.offspring,
        self.keeper_frequency,
        self.not_enough_keepers_change,
        self.social,
        self.habitat_size,
        self.number_animals_min,
        self.number_animals_max,
        self.number_min_change,
        self.number_max_change,
        self.habitat_preference,
        self.baby_born_change,
        self.energy_increment,
        self.energy_threshold,
        self.dirty_increment,
        self.dirty_threshold,
        self.sick_time,
        self.baby_to_adult,
        self.other_food,
        self.tree_pref,
        self.rock_pref,
        self.space_pref,
        self.elevation_pref,
        self.depth_min,
        self.depth_max,
        self.depth_change,
        self.salinity_change,
        self.salinity_health_change,
        self.happy_reproduce_threshold,
        self.building_use_chance,
        self.no_mate_change,
        self.time_death,
        self.death_chance,
        self.dirt_chance,
        self.water_needed,
        self.underwater_needed,
        self.land_needed,
        self.enter_water_chance,
        self.enter_tank_chance,
        self.enter_land_chance,
        self.drink_water_chance,
        self.chase_animal_chance,
        self.climbs_cliffs,
        self.bash_strength,
        self.attractiveness,
        self.keeper_food_type,
        self.is_climber,
        self.is_jumper,
        self.small_zoodoo,
        self.dino_zoodoo,
        self.giant_zoodoo,
        self.is_special_animal,
        self.need_shelter,
        self.need_toys,
        self.babies_attack
        )
    }
}

impl Deref for ZTAnimalType {
    type Target = ZTUnitType;

    fn deref(&self) -> &Self::Target {
        &self.ztunit_type
    }
}

// ------------ Custom Command Implementation ------------ //

fn command_sel_type(args: Vec<&str>) -> Result<String, &'static str> {
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
    if entity_type_address == 0 {
        return Err("No entity selected");
    }

    let entity_type = BFEntityType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
                                                                       // if -v flag is used, print the entity type configuration and other details
    if args.is_empty() {
        Ok(entity_type.print_details())
    } else if args[0] == "-v" {
        info!(
            "Printing configuration for entity type at address {:#x}",
            entity_type_print
        );

        // print the entity type configuration for the selected entity type
        Ok(print_config_for_type())
    } else if args.len() == 2 {
        // parse the subargs for the entity type
        parse_subargs_for_type(args)
    } else {
        Ok("Invalid argument".to_string())
    }
}

fn print_info_image_name(entity_type: &BFEntityType, config: &mut String) {
    info!("Checking for cInfoImageName...");
    // TODO: move cInfoImageName to a separate struct (probably ZTSceneryType). crashes when trying to access it from guests
    if entity_type.get_info_image_name() != "" {
        info!(
            "Entity type has cInfoImageName: {}",
            entity_type.get_info_image_name()
        );
        config.push_str("\n[Characteristics/Strings]\n");
        config.push_str(&entity_type.get_info_image_name());
    }
}

// prints the configuration for the selected entity type
fn print_config_for_type() -> String {
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let entity_type = BFEntityType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
    let mut config: String = String::new();

    info!(
        "Printing configuration for entity type at address {:#x}",
        entity_type_address
    );

    let class_type = determine_entity_type(entity_type_address);

    config.push_str(&entity_type.print_details());
    config.push_str("Class Type: ");
    config.push_str(&class_type);
    config.push_str("\n\n[Configuration/Integers]\n\n");
    config.push_str(&entity_type.print_config_integers());

    if class_type == "Building" {
        info!("Entity type is a building. Printing building type configuration.");
        let building_type = ZTBuildingType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&building_type.ztscenerytype.print_config_integers());
        config.push_str(&building_type.print_config_integers());
        config.push_str(&building_type.print_config_floats());
        print_info_image_name(entity_type, &mut config);
    } else if class_type == "Scenery" {
        info!("Entity type is a scenery. Printing scenery type configuration.");
        let scenery_type = ZTSceneryType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&scenery_type.print_config_integers());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "Fences" {
        info!("Entity type is a fence. Printing fence type configuration.");
        let fence_type = ZTFenceType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&fence_type.print_config_integers());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "TankWall" {
        let tank_wall_type = ZTTankWallType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(
            &tank_wall_type
                .ztfencetype
                .ztscenerytype
                .bfentitytype
                .print_config_integers(),
        );
        config.push_str(
            &tank_wall_type
                .ztfencetype
                .ztscenerytype
                .print_config_integers(),
        );
        config.push_str(&tank_wall_type.ztfencetype.print_config_integers());
        config.push_str(&tank_wall_type.print_portal_sounds());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "Food" {
        info!("Entity type is a food. Printing food type configuration.");
        let food_type = ZTFoodType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&food_type.ztscenerytype.bfentitytype.print_config_integers());
        config.push_str(&food_type.ztscenerytype.print_config_integers());
        config.push_str(&food_type.print_config_integers());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "TankFilter" {
        info!("Entity type is a tank filter. Printing tank filter type configuration.");
        let tank_filter_type = ZTTankFilterType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(
            &tank_filter_type
                .ztscenerytype
                .bfentitytype
                .print_config_integers(),
        );
        config.push_str(&tank_filter_type.ztscenerytype.print_config_integers());
        config.push_str(&tank_filter_type.print_config_integers());
        config.push_str(&tank_filter_type.print_filter_sounds());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "Path" {
        info!("Entity type is a path. Printing path type configuration.");
        let path_type = ZTPathType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&path_type.ztscenerytype.bfentitytype.print_config_integers());
        config.push_str(&path_type.ztscenerytype.print_config_integers());
        config.push_str(&path_type.print_config_integers());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "Rubble" {
        info!("Entity type is a rubble. Printing rubble type configuration.");
        let rubble_type = ZTRubbleType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(
            &rubble_type
                .ztscenerytype
                .bfentitytype
                .print_config_integers(),
        );
        config.push_str(&rubble_type.ztscenerytype.print_config_integers());
        config.push_str(&rubble_type.print_config_integers());

        print_info_image_name(entity_type, &mut config);
    } else if class_type == "Keeper" || class_type == "MaintenanceWorker" || class_type == "TourGuide" || class_type == "DRT" {
        info!("Entity type is a ZTUnit. Printing ZTUnit type configuration.");
        let ztunit_type = ZTUnitType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&ztunit_type.bfunit_type.bfentitytype.print_config_integers());
        config.push_str(&ztunit_type.bfunit_type.print_config_integers());
        config.push_str(&ztunit_type.print_config_integers());
        // config.push_str(&ztunit_type.get_list_name());

        // print_info_image_name(entity_type, &mut config);
    } else if class_type == "Guest" {
        info!("Entity type is a ZTGuest. Printing ZTGuest type configuration.");
        let ztguest_type = ZTGuestType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&ztguest_type.ztunit_type.bfunit_type.bfentitytype.print_config_integers());
        config.push_str(&ztguest_type.ztunit_type.bfunit_type.print_config_integers());
        config.push_str(&ztguest_type.ztunit_type.print_config_integers());
        config.push_str(&ztguest_type.print_config_integers());

        // print_info_image_name(entity_type, &mut config);
    } else if class_type == "Animal" {
        info!("Entity type is a ZTAnimal. Printing ZTAnimal type configuration.");
        let ztanimal_type = ZTAnimalType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&ztanimal_type.ztunit_type.bfunit_type.bfentitytype.print_config_integers());
        config.push_str(&ztanimal_type.ztunit_type.bfunit_type.print_config_integers());
        config.push_str(&ztanimal_type.ztunit_type.print_config_integers());
        config.push_str(&ztanimal_type.print_config_integers());

        // print_info_image_name(entity_type, &mut config);
    } else {
        info!("Entity type is not a known type. Skipping additional configuration.");
    }

    // print [colorrep] section of the configuration - available in all entity types
    // config.push_str(&entity_type.print_colorrep());
    // info!("Colorrep printed successfully.");

    info!("Configuration printed successfully.");
    config
}

// parses the subargs for the entity type
fn parse_subargs_for_type(_args: Vec<&str>) -> Result<String, &'static str> {
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let building_type = ZTBuildingType::new(entity_type_address).unwrap(); // create a copied instance of the entity type

    // test for arguments in the entity type, scenery type, and building type
    let result_entity_type = building_type
        .ztscenerytype
        .bfentitytype
        .set_config(_args[0], _args[1]);
    let result_scenery_type = building_type.ztscenerytype.set_config(_args[0], _args[1]);
    let result_building_type = building_type.set_config(_args[0], _args[1]);
    let result_fence_type = ZTFenceType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_food_type = ZTFoodType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_tank_filter_type = ZTTankFilterType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_tank_wall_type = ZTTankWallType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_path_type = ZTPathType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_rubble_type = ZTRubbleType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_bfunit_type = BFUnitType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_ztunit_type = ZTUnitType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_ztguest_type = ZTGuestType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);
    let result_ztanimal_type = ZTAnimalType::new(entity_type_address)
        .unwrap()
        .set_config(_args[0], _args[1]);

    // return the result of the first successful configuration change
    if result_entity_type.is_ok() {
        result_entity_type
    } else if result_scenery_type.is_ok() {
        result_scenery_type
    } else if result_building_type.is_ok() {
        result_building_type
    } else if result_fence_type.is_ok() {
        result_fence_type
    } else if result_tank_wall_type.is_ok() {
        result_tank_wall_type
    } else if result_food_type.is_ok() {
        result_food_type
    } else if result_tank_filter_type.is_ok() {
        result_tank_filter_type
    } else if result_path_type.is_ok() {
        result_path_type
    } else if result_rubble_type.is_ok() {
        result_rubble_type
    } else if result_bfunit_type.is_ok() {
        result_bfunit_type
    } else if result_ztunit_type.is_ok() {
        result_ztunit_type
    } else if result_ztguest_type.is_ok() {
        result_ztguest_type
    } else if result_ztanimal_type.is_ok() {
        result_ztanimal_type
    } else {
        Err("Invalid configuration option")
    }
}

// initializes the custom command
pub fn init() {
    add_to_command_register("sel_type".to_string(), command_sel_type);
}
