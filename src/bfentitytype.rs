// ------------ BFEntityType, Implementation, and Related Functions ------------ //
use std::ops::Deref;

use getset::{Getters, Setters};
use tracing::info;

use crate::{
    console::{add_to_command_register, CommandError},
    debug_dll::{get_from_memory, get_string_from_memory, map_from_memory},
    ztui::get_selected_entity_type_address, ztworldmgr::{ZTEntityType, ZTEntityTypeClass},
};

pub trait EntityType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError>;
    fn print_config_integers(&self) -> String;
    fn print_config_floats(&self) -> String;
    fn print_config_strings(&self) -> String;
    fn print_config_details(&self) -> String;
    fn print_config(&self) -> String {
        format!(
            "[Details]\n{}\n[Configurations/Integers]\n{}\n[Configurations/Floats]\n{}\n[Configurations/Strings]\n{}",
            self.print_config_details(),
            self.print_config_integers(),
            self.print_config_floats(),
            self.print_config_strings(),
        )
    }
}

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

     // prints [colorrep] section of the configuration
     fn print_colorrep(&self) -> String {
        // NOTE: ncolors is part of a separate structure in memory withn BFEntityType, so we need to grab the pointer to it first
        // this is temporary until the struct can be fully implemented
        let entity_type_address = get_selected_entity_type_address(); // grab the address of the selected entity type
        let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
        let ncolors_ptr = get_from_memory::<u32>(entity_type_print + 0x038);
        let ncolors = get_from_memory::<u32>(ncolors_ptr);

        format!("\n\n[colorrep]\nncolors: {}\n", ncolors)
    }
}

impl EntityType for BFEntityType {
    // allows setting the configuration of the entity type
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cIconZoom" {
            self.icon_zoom = value.parse()?;
            Ok(format!("Set cIconZoom to {}", self.icon_zoom))
        } else if config == "-cExpansionID" {
            self.expansion_id = value.parse()?;
            Ok(format!("Set cExpansionID to {}", self.expansion_id))
        } else if config == "-cMovable" {
            self.movable = value.parse()?;
            Ok(format!("Set cMovable to {}", self.movable))
        } else if config == "-cWalkable" {
            self.walkable = value.parse()?;
            Ok(format!("Set cWalkable to {}", self.walkable))
        } else if config == "-cWalkableByTall" {
            self.walkable_by_tall = value.parse()?;
            Ok(format!("Set cWalkableByTall to {}", self.walkable_by_tall))
        } else if config == "-cRubbleable" {
            self.rubbleable = value.parse()?;
            Ok(format!("Set cRubbleable to {}", self.rubbleable))
        } else if config == "-cUseNumbersInName" {
            self.use_numbers_in_name = value.parse()?;
            Ok(format!("Set cUseNumbersInName to {}", self.use_numbers_in_name))
        } else if config == "-cUsesRealShadows" {
            self.uses_real_shadows = value.parse()?;
            Ok(format!("Set cUsesRealShadows to {}", self.uses_real_shadows))
        } else if config == "-cHasShadowImages" {
            self.has_shadow_images = value.parse()?;
            Ok(format!("Set cHasShadowImages to {}", self.has_shadow_images))
        } else if config == "-cForceShadowBlack" {
            self.force_shadow_black = value.parse()?;
            Ok(format!("Set cForceShadowBlack to {}", self.force_shadow_black))
        } else if config == "-cDrawsLate" {
            self.draws_late = value.parse()?;
            Ok(format!("Set cDrawsLate to {}", self.draws_late))
        } else if config == "-cHeight" {
            self.height = value.parse()?;
            Ok(format!("Set cHeight to {}", self.height))
        } else if config == "-cDepth" {
            self.depth = value.parse()?;
            Ok(format!("Set cDepth to {}", self.depth))
        } else if config == "-cHasUnderwaterSection" {
            self.has_underwater_section = value.parse()?;
            Ok(format!("Set cHasUnderwaterSection to {}", self.has_underwater_section))
        } else if config == "-cIsTransient" {
            self.is_transient = value.parse()?;
            Ok(format!("Set cIsTransient to {}", self.is_transient))
        } else if config == "-cUsesPlacementCube" {
            self.uses_placement_cube = value.parse()?;
            Ok(format!("Set cUsesPlacementCube to {}", self.uses_placement_cube))
        } else if config == "-cShow" {
            self.show = value.parse()?;
            Ok(format!("Set cShow to {}", self.show))
        } else if config == "-cHitThreshold" {
            self.hit_threshold = value.parse()?;
            Ok(format!("Set cHitThreshold to {}", self.hit_threshold))
        } else if config == "-cAvoidEdges" {
            self.avoid_edges = value.parse()?;
            Ok(format!("Set cAvoidEdges to {}", self.avoid_edges))
        } else if config == "-cFootprintX" {
            self.footprintx = value.parse()?;
            Ok(format!("Set cFootprintX to {}", self.footprintx))
        } else if config == "-cFootprintY" {
            self.footprinty = value.parse()?;
            Ok(format!("Set cFootprintY to {}", self.footprinty))
        } else if config == "-cFootprintZ" {
            self.footprintz = value.parse()?;
            Ok(format!("Set cFootprintZ to {}", self.footprintz))
        } else if config == "-cPlacementFootprintX" {
            self.placement_footprintx = value.parse()?;
            Ok(format!("Set cPlacementFootprintX to {}", self.placement_footprintx))
        } else if config == "-cPlacementFootprintY" {
            self.placement_footprinty = value.parse()?;
            Ok(format!("Set cPlacementFootprintY to {}", self.placement_footprinty))
        } else if config == "-cPlacementFootprintZ" {
            self.placement_footprintz = value.parse()?;
            Ok(format!("Set cPlacementFootprintZ to {}", self.placement_footprintz))
        } else if config == "-cAvailableAtStartup" {
            self.available_at_startup = value.parse()?;
            Ok(format!("Set cAvailableAtStartup to {}", self.available_at_startup))
        } else {
            Err(CommandError::new(format!("Invalid configuration option: {}", config)))
        }
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

    fn print_config_floats(&self) -> String {
        String::new()
    }

    fn print_config_strings(&self) -> String {
        String::new()
    }

    // prints misc details of the entity type
    fn print_config_details(&self) -> String {
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
    pub fn get_info_image_name(&self) -> String {
        let obj_ptr = self as *const ZTSceneryType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x14C))
    }
}

impl EntityType for ZTSceneryType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cPurchaseCost" {
            self.purchase_cost = value.parse()?;
            Ok(format!("Set Purchase Cost to {}", self.purchase_cost))
        } else if config == "-cNameID" {
            self.name_id = value.parse()?;
            Ok(format!("Set Name ID to {}", self.name_id))
        } else if config == "-cHelpID" {
            self.help_id = value.parse()?;
            Ok(format!("Set Help ID to {}", self.help_id))
        } else if config == "-cHabitat" {
            self.habitat = value.parse()?;
            Ok(format!("Set Habitat to {}", self.habitat))
        } else if config == "-cLocation" {
            self.location = value.parse()?;
            Ok(format!("Set Location to {}", self.location))
        } else if config == "-cEra" {
            self.era = value.parse()?;
            Ok(format!("Set Era to {}", self.era))
        } else if config == "-cMaxFoodUnits" {
            self.max_food_units = value.parse()?;
            Ok(format!("Set Max Food Units to {}", self.max_food_units))
        } else if config == "-cStink" {
            self.stink = value.parse()?;
            Ok(format!("Set Stink to {}", self.stink))
        } else if config == "-cEstheticWeight" {
            self.esthetic_weight = value.parse()?;
            Ok(format!("Set Esthetic Weight to {}", self.esthetic_weight))
        } else if config == "-cSelectable" {
            self.selectable = value.parse()?;
            Ok(format!("Set Selectable to {}", self.selectable))
        } else if config == "-cDeletable" {
            self.deletable = value.parse()?;
            Ok(format!("Set Deletable to {}", self.deletable))
        } else if config == "-cFoliage" {
            self.foliage = value.parse()?;
            Ok(format!("Set Foliage to {}", self.foliage))
        } else if config == "-cAutoRotate" {
            self.auto_rotate = value.parse()?;
            Ok(format!("Set Auto Rotate to {}", self.auto_rotate))
        } else if config == "-cLand" {
            self.land = value.parse()?;
            Ok(format!("Set Land to {}", self.land))
        } else if config == "-cSwims" {
            self.swims = value.parse()?;
            Ok(format!("Set Swims to {}", self.swims))
        } else if config == "-cUnderwater" {
            self.underwater = value.parse()?;
            Ok(format!("Set Underwater to {}", self.underwater))
        } else if config == "-cSurface" {
            self.surface = value.parse()?;
            Ok(format!("Set Surface to {}", self.surface))
        } else if config == "-cSubmerge" {
            self.submerge = value.parse()?;
            Ok(format!("Set Submerge to {}", self.submerge))
        } else if config == "-cOnlySwims" {
            self.only_swims = value.parse()?;
            Ok(format!("Set Only Swims to {}", self.only_swims))
        } else if config == "-cNeedsConfirm" {
            self.needs_confirm = value.parse()?;
            Ok(format!("Set Needs Confirm to {}", self.needs_confirm))
        } else if config == "-cGawkOnlyFromFront" {
            self.gawk_only_from_front = value.parse()?;
            Ok(format!("Set Gawk Only From Front to {}", self.gawk_only_from_front))
        } else if config == "-cDeadOnLand" {
            self.dead_on_land = value.parse()?;
            Ok(format!("Set Dead On Land to {}", self.dead_on_land))
        } else if config == "-cDeadOnFlatWater" {
            self.dead_on_flat_water = value.parse()?;
            Ok(format!("Set Dead On Flat Water to {}", self.dead_on_flat_water))
        } else if config == "-cDeadUnderwater" {
            self.dead_underwater = value.parse()?;
            Ok(format!("Set Dead Underwater to {}", self.dead_underwater))
        } else if config == "-cUsesTreeRubble" {
            self.uses_tree_rubble = value.parse()?;
            Ok(format!("Set Uses Tree Rubble to {}", self.uses_tree_rubble))
        } else if config == "-cForcesSceneryRubble" {
            self.forces_scenery_rubble = value.parse()?;
            Ok(format!("Set Forces Scenery Rubble to {}", self.forces_scenery_rubble))
        } else if config == "-cBlocksLOS" {
            self.blocks_los = value.parse()?;
            Ok(format!("Set Blocks LOS to {}", self.blocks_los))
        } else {
            Ok(self.bfentitytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncPurchaseCost: {}\ncNameID: {}\ncHelpID: {}\ncHabitat: {}\ncLocation: {}\ncEra: {}\ncMaxFoodUnits: {}\ncStink: {}\ncEstheticWeight: {}\ncSelectable: {}\ncDeletable: {}\ncFoliage: {}\ncAutoRotate: {}\ncLand: {}\ncSwims: {}\ncUnderwater: {}\ncSurface: {}\ncSubmerge: {}\ncOnlySwims: {}\ncNeedsConfirm: {}\ncGawkOnlyFromFront: {}\ncDeadOnLand: {}\ncDeadOnFlatWater: {}\ncDeadUnderwater: {}\ncUsesTreeRubble: {}\ncForcesSceneryRubble: {}\ncBlocksLOS: {}\n",
                self.bfentitytype.print_config_integers(),
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
    
    fn print_config_floats(&self) -> String {
        self.bfentitytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.bfentitytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.bfentitytype.print_config_details()
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

impl EntityType for ZTBuildingType {
    // sets the configuration of the building type in the console
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cCapacity" {
            self.i_capacity = value.parse()?;
            Ok(format!("Set Capacity to {}", self.i_capacity))
        } else if config == "-cToySatisfaction" {
            self.toy_satisfaction = value.parse()?;
            Ok(format!("Set Toy Satisfaction to {}", self.toy_satisfaction))
        } else if config == "-cTimeInside" {
            self.time_inside = value.parse()?;
            Ok(format!("Set Time Inside to {}", self.time_inside))
        } else if config == "-cDefaultCost" {
            self.default_cost = value.parse()?;
            Ok(format!("Set Default Cost to {}", self.default_cost))
        } else if config == "-cLowCost" {
            self.low_cost = value.parse()?;
            Ok(format!("Set Low Cost to {}", self.low_cost))
        } else if config == "-cMedCost" {
            self.med_cost = value.parse()?;
            Ok(format!("Set Med Cost to {}", self.med_cost))
        } else if config == "-cHighCost" {
            self.high_cost = value.parse()?;
            Ok(format!("Set High Cost to {}", self.high_cost))
        } else if config == "-cPriceFactor" {
            self.price_factor = value.parse()?;
            Ok(format!("Set Price Factor to {}", self.price_factor))
        } else if config == "-cUpkeep" {
            self.upkeep = value.parse()?;
            Ok(format!("Set Upkeep to {}", self.upkeep))
        } else if config == "-cHideUser" {
            self.hide_user = value.parse()?;
            Ok(format!("Set Hide User to {}", self.hide_user))
        } else if config == "-cSetLetterFacing" {
            self.set_letter_facing = value.parse()?;
            Ok(format!("Set Set Letter Facing to {}", self.set_letter_facing))
        } else if config == "-cDrawUser" {
            self.draw_user = value.parse()?;
            Ok(format!("Set Draw User to {}", self.draw_user))
        } else if config == "-cHideCostChange" {
            self.hide_cost_change = value.parse()?;
            Ok(format!("Set Hide Cost Change to {}", self.hide_cost_change))
        } else if config == "-cHideCommerceInfo" {
            self.hide_commerce_info = value.parse()?;
            Ok(format!("Set Hide Commerce Info to {}", self.hide_commerce_info))
        } else if config == "-cHideRegularInfo" {
            self.hide_regular_info = value.parse()?;
            Ok(format!("Set Hide Regular Info to {}", self.hide_regular_info))
        } else if config == "-cHoldsOntoUser" {
            self.holds_onto_user = value.parse()?;
            Ok(format!("Set Holds Onto User to {}", self.holds_onto_user))
        } else if config == "-cUserTracker" {
            self.user_tracker = value.parse()?;
            Ok(format!("Set User Tracker to {}", self.user_tracker))
        } else if config == "-cIdler" {
            self.idler = value.parse()?;
            Ok(format!("Set Idler to {}", self.idler))
        } else if config == "-cExhibitViewer" {
            self.exhibit_viewer = value.parse()?;
            Ok(format!("Set Exhibit Viewer to {}", self.exhibit_viewer))
        } else if config == "-cAlternatePanelTitle" {
            self.alternate_panel_title = value.parse()?;
            Ok(format!("Set Alternate Panel Title to {}", self.alternate_panel_title))
        } else if config == "-cDirectEntrance" {
            self.direct_entrance = value.parse()?;
            Ok(format!("Set Direct Entrance to {}", self.direct_entrance))
        } else if config == "-cHideBuilding" {
            self.hide_building = value.parse()?;
            Ok(format!("Set Hide Building to {}", self.hide_building))
        } else if config == "-cUserStaysOutside" {
            self.user_stays_outside = value.parse()?;
            Ok(format!("Set User Stays Outside to {}", self.user_stays_outside))
        } else if config == "-cUserTeleportsInside" {
            self.user_teleports_inside = value.parse()?;
            Ok(format!("Set User Teleports Inside to {}", self.user_teleports_inside))
        } else if config == "-cUserUsesExit" {
            self.user_uses_exit = value.parse()?;
            Ok(format!("Set User Uses Exit to {}", self.user_uses_exit))
        } else if config == "-cUserUsesEntranceAsEmergencyExit" {
            self.user_uses_entrance_as_emergency_exit = value.parse()?;
            Ok(format!(
                "Set User Uses Entrance As Emergency Exit to {}",
                self.user_uses_entrance_as_emergency_exit
            ))
        } else if config == "-cAdultChange" {
            self.adult_change = value.parse()?;
            Ok(format!("Set Adult Change to {}", self.adult_change))
        } else if config == "-cChildChange" {
            self.child_change = value.parse()?;
            Ok(format!("Set Child Change to {}", self.child_change))
        } else if config == "-cHungerChange" {
            self.hunger_change = value.parse()?;
            Ok(format!("Set Hunger Change to {}", self.hunger_change))
        } else if config == "-cThirstChange" {
            self.thirst_change = value.parse()?;
            Ok(format!("Set Thirst Change to {}", self.thirst_change))
        } else if config == "-cBathroomChange" {
            self.bathroom_change = value.parse()?;
            Ok(format!("Set Bathroom Change to {}", self.bathroom_change))
        } else if config == "-cEnergyChange" {
            self.energy_change = value.parse()?;
            Ok(format!("Set Energy Change to {}", self.energy_change))
        } else {
            Ok(self.ztscenerytype.set_config(config, value)?)
        }
    }

    // print [Configuration/Floats] section of the configuration
    fn print_config_floats(&self) -> String {
        format!("{}\n\n[Configuration/Floats]\n\ncDefaultCost: {:.2}\ncLowCost: {:.2}\ncMedCost: {:.2}\ncHighCost: {:.2}\ncPriceFactor: {:.2}\ncUpkeep: {:.2}\n",
                self.ztscenerytype.print_config_floats(),
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
        format!("{}\ncCapacity: {}\ncToySatisfaction: {}\ncTimeInside: {}\ncHideUser: {}\ncSetLetterFacing: {}\ncDrawUser: {}\ncHideCostChange: {}\ncHideCommerceInfo: {}\ncHideRegularInfo: {}\ncHoldsOntoUser: {}\ncUserTracker: {}\ncIdler: {}\ncExhibitViewer: {}\ncAlternatePanelTitle: {}\ncDirectEntrance: {}\ncHideBuilding: {}\ncUserStaysOutside: {}\ncUserTeleportsInside: {}\ncUserUsesExit: {}\ncUserUsesEntranceAsEmergencyExit: {}\ncAdultChange: {}\ncChildChange: {}\ncHungerChange: {}\ncThirstChange: {}\ncBathroomChange: {}\ncEnergyChange: {}\n",
                self.ztscenerytype.print_config_integers(),
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
    
    fn print_config_strings(&self) -> String {
        self.ztscenerytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.ztscenerytype.print_config_details()
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
    fn get_break_sound(&self) -> String {
        let obj_ptr = self as *const ZTFenceType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x184))
    }

    fn get_open_sound(&self) -> String {
        let obj_ptr = self as *const ZTFenceType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x188))
    }
}

impl EntityType for ZTFenceType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cStrength" {
            self.strength = value.parse()?;
            Ok(format!("Set Strength to {}", self.strength))
        } else if config == "-cLife" {
            self.life = value.parse()?;
            Ok(format!("Set Life to {}", self.life))
        } else if config == "-cDecayedLife" {
            self.decayed_life = value.parse()?;
            Ok(format!("Set Decayed Life to {}", self.decayed_life))
        } else if config == "-cDecayedDelta" {
            self.decayed_delta = value.parse()?;
            Ok(format!("Set Decayed Delta to {}", self.decayed_delta))
        } else if config == "-cBreakSoundAtten" {
            self.break_sound_atten = value.parse()?;
            Ok(format!("Set Break Sound Atten to {}", self.break_sound_atten))
        } else if config == "-cOpenSoundAtten" {
            self.open_sound_atten = value.parse()?;
            Ok(format!("Set Open Sound Atten to {}", self.open_sound_atten))
        } else if config == "-cSeeThrough" {
            self.see_through = value.parse()?;
            Ok(format!("Set See Through to {}", self.see_through))
        } else if config == "-cIsJumpable" {
            self.is_jumpable = value.parse()?;
            Ok(format!("Set Is Jumpable to {}", self.is_jumpable))
        } else if config == "-cIsClimbable" {
            self.is_climbable = value.parse()?;
            Ok(format!("Set Is Climbable to {}", self.is_climbable))
        } else if config == "-cIndestructible" {
            self.indestructible = value.parse()?;
            Ok(format!("Set Indestructible to {}", self.indestructible))
        } else if config == "-cIsElectrified" {
            self.is_electrified = value.parse()?;
            Ok(format!("Set Is Electrified to {}", self.is_electrified))
        } else if config == "-cNoDrawWater" {
            self.no_draw_water = value.parse()?;
            Ok(format!("Set No Draw Water to {}", self.no_draw_water))
        } else {
            Ok(self.ztscenerytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncStrength: {}\ncLife: {}\ncDecayedLife: {}\ncDecayedDelta: {}\ncBreakSoundAtten: {}\ncOpenSoundAtten: {}\ncSeeThrough: {}\ncIsJumpable: {}\ncIsClimbable: {}\ncIndestructible: {}\ncIsElectrified: {}\ncNoDrawWater: {}\n", // cBreakSound: {}\ncOpenSound: {}\n",
                self.ztscenerytype.print_config_integers(),
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
    
    fn print_config_floats(&self) -> String {
        self.ztscenerytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztscenerytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.ztscenerytype.print_config_details()
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
    fn get_portal_open_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankWallType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x1A4))
    }

    fn get_portal_close_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankWallType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x1B0))
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

impl EntityType for ZTTankWallType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        // if config == "-cPortalOpenSound" {
        //     self.portal_open_sound = value.parse()?;
        //     Ok(format!("Set Portal Open Sound to {}", self.portal_open_sound))
        // }
        // else if config == "-cPortalCloseSound" {
        //     self.portal_close_sound = value.parse()?;
        //     Ok(format!("Set Portal Close Sound to {}", self.portal_close_sound))
        // }
        if config == "-cPortalOpenSoundAtten" {
            self.portal_open_sound_atten = value.parse()?;
            Ok(format!("Set Portal Open Sound Atten to {}", self.portal_open_sound_atten))
        } else if config == "-cPortalCloseSoundAtten" {
            self.portal_close_sound_atten = value.parse()?;
            Ok(format!("Set Portal Close Sound Atten to {}", self.portal_close_sound_atten))
        } else {
            Ok(self.ztfencetype.set_config(config, value)?)
        }
    }
    
    fn print_config_integers(&self) -> String {
        self.ztfencetype.print_config_integers()
    }
    
    fn print_config_floats(&self) -> String {
        self.ztfencetype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztfencetype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        format!("{}\n{}\n", self.ztfencetype.print_config_details(), self.print_portal_sounds())
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
    pub ztscenerytype: ZTSceneryType,
    // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub keeper_food_type: u32, // 0x168
}

impl EntityType for ZTFoodType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cKeeperFoodType" {
            self.keeper_food_type = value.parse()?;
            Ok(format!("Set Keeper Food Type to {}", self.keeper_food_type))
        } else {
            Ok(self.ztscenerytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncKeeperFoodType: {}\n", self.ztscenerytype.print_config_integers(), self.keeper_food_type)
    }
    
    fn print_config_floats(&self) -> String {
        self.ztscenerytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztscenerytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.ztscenerytype.print_config_details()
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
    fn get_healthy_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankFilterType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x184))
    }

    fn get_decayed_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankFilterType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x190))
    }
}

impl EntityType for ZTTankFilterType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cStartingHealth" {
            self.starting_health = value.parse()?;
            Ok(format!("Set Starting Health to {}", self.starting_health))
        } else if config == "-cDecayedHealth" {
            self.decayed_health = value.parse()?;
            Ok(format!("Set Decayed Health to {}", self.decayed_health))
        } else if config == "-cDecayTime" {
            self.decay_time = value.parse()?;
            Ok(format!("Set Decay Time to {}", self.decay_time))
        } else if config == "-cFilterDelay" {
            self.filter_delay = value.parse()?;
            Ok(format!("Set Filter Delay to {}", self.filter_delay))
        } else if config == "-cFilterUpkeep" {
            self.filter_upkeep = value.parse()?;
            Ok(format!("Set Filter Upkeep to {}", self.filter_upkeep))
        } else if config == "-cFilterCleanAmount" {
            self.filter_clean_amount = value.parse()?;
            Ok(format!("Set Filter Clean Amount to {}", self.filter_clean_amount))
        } else if config == "-cFilterDecayedCleanAmount" {
            self.filter_decayed_clean_amount = value.parse()?;
            Ok(format!("Set Filter Decayed Clean Amount to {}", self.filter_decayed_clean_amount))
        } else if config == "-cHealthyAtten" {
            self.healthy_atten = value.parse()?;
            Ok(format!("Set Healthy Atten to {}", self.healthy_atten))
        } else if config == "-cDecayedAtten" {
            self.decayed_atten = value.parse()?;
            Ok(format!("Set Decayed Atten to {}", self.decayed_atten))
        } else {
            Ok(self.ztscenerytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncStartingHealth: {}\ncDecayedHealth: {}\ncDecayTime: {}\ncFilterDelay: {}\ncFilterUpkeep: {}\ncFilterCleanAmount: {}\ncFilterDecayedCleanAmount: {}\ncHealthyAtten: {}\ncDecayedAtten: {}\ncHealthySound: {}\ncDecayedSound: {}\n",
                self.ztscenerytype.print_config_integers(),
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

    fn print_config_details(&self) -> String {
        format!("{}\n\n[FilterSounds]\n\ncHealthySound: {}\ncHealthyAtten: {}\ncDecayedSound: {}\ncDecayedAtten: {}\n\n",
                self.ztscenerytype.print_config_details(),
                self.get_healthy_sound(),
                self.healthy_atten,
                self.get_decayed_sound(),
                self.decayed_atten
        )
    }
    
    fn print_config_floats(&self) -> String {
        self.ztscenerytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztscenerytype.print_config_strings()
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
    ztscenerytype: ZTSceneryType,
    // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub material: u32, // 0x168
                       // TODO: missing Shapes structure in paths. Could not find.
}

impl EntityType for ZTPathType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cMaterial" {
            self.material = value.parse()?;
            Ok(format!("Set Material to {}", self.material))
        } else {
            Ok(self.ztscenerytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncMaterial: {}\n", self.ztscenerytype.print_config_integers(), self.material,)
    }
    
    fn print_config_floats(&self) -> String {
        self.ztscenerytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztscenerytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.ztscenerytype.print_config_details()
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
    ztscenerytype: ZTSceneryType,
    // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    // explosion_sound: String, // 0x168
    pad0: [u8; 0x16C - 0x168],
    // ----------------------- padding: 4 bytes
    pub explosion_sound_atten: i32, // 0x16C
}

impl ZTRubbleType {
    fn get_explosion_sound(&self) -> String {
        let obj_ptr = self as *const ZTRubbleType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x168))
    }
}

impl EntityType for ZTRubbleType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        // if config == "-cExplosionSound" {
        //     self.explosion_sound = value.parse::<String>()?;
        //     Ok(format!("Set Explosion Sound to {}", self.explosion_sound))
        // }
        if config == "-cExplosionSoundAtten" {
            self.explosion_sound_atten = value.parse()?;
            Ok(format!("Set Explosion Sound Atten to {}", self.explosion_sound_atten))
        } else {
            Ok(self.ztscenerytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!(
            "{}\ncExplosionSound: {}\ncExplosionSoundAtten: {}\n",
            self.ztscenerytype.print_config_integers(),
            self.get_explosion_sound(),
            self.explosion_sound_atten,
        )
    }
    
    fn print_config_floats(&self) -> String {
        self.ztscenerytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztscenerytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.ztscenerytype.print_config_details()
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
    pub slow_rate: u32,         // 0x100
    pub medium_rate: u32,       // 0x104
    pub fast_rate: u32,         // 0x108
    pub slow_anim_speed: u16,   // 0x10C
    pub medium_anim_speed: u16, // 0x10E
    pub fast_anim_speed: u16,   // 0x110
    pad0: [u8; 0x114 - 0x112],  // ----------------------- padding: 2 bytes
    pub min_height: u32,        // 0x114 <--- unsure if accurate
    pub max_height: u32,        // 0x118 <--- unsure if accurate
}

impl EntityType for BFUnitType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cSlowRate" {
            self.slow_rate = value.parse()?;
            Ok(format!("Set Slow Rate to {}", self.slow_rate))
        } else if config == "-cMediumRate" {
            self.medium_rate = value.parse()?;
            Ok(format!("Set Medium Rate to {}", self.medium_rate))
        } else if config == "-cFastRate" {
            self.fast_rate = value.parse()?;
            Ok(format!("Set Fast Rate to {}", self.fast_rate))
        } else if config == "-cSlowAnimSpeed" {
            self.slow_anim_speed = value.parse()?;
            Ok(format!("Set Slow Anim Speed to {}", self.slow_anim_speed))
        } else if config == "-cMediumAnimSpeed" {
            self.medium_anim_speed = value.parse()?;
            Ok(format!("Set Medium Anim Speed to {}", self.medium_anim_speed))
        } else if config == "-cFastAnimSpeed" {
            self.fast_anim_speed = value.parse()?;
            Ok(format!("Set Fast Anim Speed to {}", self.fast_anim_speed))
        } else if config == "-cMinHeight" {
            self.min_height = value.parse()?;
            Ok(format!("Set Min Height to {}", self.min_height))
        } else if config == "-cMaxHeight" {
            self.max_height = value.parse()?;
            Ok(format!("Set Max Height to {}", self.max_height))
        } else {
            Ok(self.bfentitytype.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncSlowRate: {}\ncMediumRate: {}\ncFastRate: {}\ncSlowAnimSpeed: {}\ncMediumAnimSpeed: {}\ncFastAnimSpeed: {}\ncMinHeight: {}\ncMaxHeight: {}\n",
                self.bfentitytype.print_config_integers(),
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
    
    fn print_config_floats(&self) -> String {
        self.bfentitytype.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.bfentitytype.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.bfentitytype.print_config_details()
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
    pub bfunit_type: BFUnitType,    // bytes: 0x11C - 0x100 = 0x1C = 28 bytes
    pad0: [u8; 0x12C - 0x11C],      // ----------------------- padding: 16 bytes
    pub purchase_cost: f32,         // 0x12C
    pub name_id: i32,               // 0x130
    pub help_id: i32,               // 0x134
    pad1: [u8; 0x150 - 0x138],      // ----------------------- padding: 24 bytes
    pub map_footprint: i32,         // 0x150
    pub slow_anim_speed_water: u16, // 0x154
    pub medium_anim_speed_water: u16, // 0x156
    pub fast_anim_speed_water: u16, // 0x158
    pad2: [u8; 0x17C - 0x15C],      // ----------------------- padding: 32 bytes
    // pub list_image_name: String,    // 0x168 TODO: fix offset for string getters in unittype
    pub swims: bool,               // 0x17C
    pub surface: bool,             // 0x17D
    pub underwater: bool,          // 0x17E
    pub only_underwater: bool,     // 0x17F
    pad3: [u8; 0x180 - 0x17F],     // ----------------------- padding: 1 byte
    pub skip_trick_happiness: u32, // 0x180 TODO: potentially not accurate
    pub skip_trick_chance: i32,    // 0x184
}

impl ZTUnitType {
    pub fn get_list_name(&self) -> String {
        let obj_ptr = self as *const ZTUnitType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x168))
    }
}

impl EntityType for ZTUnitType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cPurchaseCost" {
            self.purchase_cost = value.parse()?;
            Ok(format!("Set Purchase Cost to {}", self.purchase_cost))
        } else if config == "-cNameID" {
            self.name_id = value.parse()?;
            Ok(format!("Set Name ID to {}", self.name_id))
        } else if config == "-cHelpID" {
            self.help_id = value.parse()?;
            Ok(format!("Set Help ID to {}", self.help_id))
        } else if config == "-cMapFootprint" {
            self.map_footprint = value.parse()?;
            Ok(format!("Set Map Footprint to {}", self.map_footprint))
        } else if config == "-cSlowAnimSpeedWater" {
            self.slow_anim_speed_water = value.parse()?;
            Ok(format!("Set Slow Anim Speed Water to {}", self.slow_anim_speed_water))
        } else if config == "-cMediumAnimSpeedWater" {
            self.medium_anim_speed_water = value.parse()?;
            Ok(format!("Set Medium Anim Speed Water to {}", self.medium_anim_speed_water))
        } else if config == "-cFastAnimSpeedWater" {
            self.fast_anim_speed_water = value.parse()?;
            Ok(format!("Set Fast Anim Speed Water to {}", self.fast_anim_speed_water))
        } else if config == "-cSwims" {
            self.swims = value.parse()?;
            Ok(format!("Set Swims to {}", self.swims))
        } else if config == "-cSurface" {
            self.surface = value.parse()?;
            Ok(format!("Set Surface to {}", self.surface))
        } else if config == "-cUnderwater" {
            self.underwater = value.parse()?;
            Ok(format!("Set Underwater to {}", self.underwater))
        } else if config == "-cOnlyUnderwater" {
            self.only_underwater = value.parse()?;
            Ok(format!("Set Only Underwater to {}", self.only_underwater))
        } else if config == "-cSkipTrickHappiness" {
            self.skip_trick_happiness = value.parse()?;
            Ok(format!("Set Skip Trick Happiness to {}", self.skip_trick_happiness))
        } else if config == "-cSkipTrickChance" {
            self.skip_trick_chance = value.parse()?;
            Ok(format!("Set Skip Trick Chance to {}", self.skip_trick_chance))
        } else {
            Ok(self.bfunit_type.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncPurchaseCost: {}\ncNameID: {}\ncHelpID: {}\ncMapFootprint: {}\ncSlowAnimSpeedWater: {}\ncMediumAnimSpeedWater: {}\ncFastAnimSpeedWater: {}\ncSwims: {}\ncSurface: {}\ncUnderwater: {}\ncOnlyUnderwater: {}\ncSkipTrickHappiness: {}\ncSkipTrickChance: {}\n",
                self.bfunit_type.print_config_integers(),
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
    
    fn print_config_floats(&self) -> String {
        self.bfunit_type.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.bfunit_type.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.bfunit_type.print_config_details()
    }
}

// ------------ ZTGuestType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTGuestType {
    pub ztunit_type: ZTUnitType, // bytes: 0x188 - 0x100 = 0x88 = 136 bytes
    pad00: [u8; 0x1B4 - 0x188],  // ----------------------- padding: 44 bytes
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
    pad01: [u8; 0x1E8 - 0x1E4],  // ----------------------- padding: 4 bytes
    pub initial_happiness: i32,  // 0x1E8
    pad02: [u8; 0x200 - 0x1EC],  // ----------------------- padding: 20 bytes
    pub max_energy: i32,         // 0x200
    pad03: [u8; 0x210 - 0x204],  // ----------------------- padding: 12 bytes
    pub energy_increment: i32,   // 0x210
    pub energy_threshold: i32,   // 0x214
    pub angry_energy_change: i32, // 0x218
    pub hunger_increment: i32,   // 0x21C
    pub hunger_threshold: i32,   // 0x220
    pub angry_food_change: i32,  // 0x224
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

impl EntityType for ZTGuestType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        if config == "-cHungerCheck" {
            self.hunger_check = value.parse()?;
            Ok(format!("Set Hunger Check to {}", self.hunger_check))
        } else if config == "-cThirstyCheck" {
            self.thirsty_check = value.parse()?;
            Ok(format!("Set Thirsty Check to {}", self.thirsty_check))
        } else if config == "-cBathroomCheck" {
            self.bathroom_check = value.parse()?;
            Ok(format!("Set Bathroom Check to {}", self.bathroom_check))
        } else if config == "-cLeaveZooCheck" {
            self.leave_zoo_check = value.parse()?;
            Ok(format!("Set Leave Zoo Check to {}", self.leave_zoo_check))
        } else if config == "-cBuySouvenirCheck" {
            self.buy_souvenir_check = value.parse()?;
            Ok(format!("Set Buy Souvenir Check to {}", self.buy_souvenir_check))
        } else if config == "-cEnergyCheck" {
            self.energy_check = value.parse()?;
            Ok(format!("Set Energy Check to {}", self.energy_check))
        } else if config == "-cChaseCheck" {
            self.chase_check = value.parse()?;
            Ok(format!("Set Chase Check to {}", self.chase_check))
        } else if config == "-cTrashCheck" {
            self.trash_check = value.parse()?;
            Ok(format!("Set Trash Check to {}", self.trash_check))
        } else if config == "-cLikeAnimalsCheck" {
            self.like_animals_check = value.parse()?;
            Ok(format!("Set Like Animals Check to {}", self.like_animals_check))
        } else if config == "-cViewingAreaCheck" {
            self.viewing_area_check = value.parse()?;
            Ok(format!("Set Viewing Area Check to {}", self.viewing_area_check))
        } else if config == "-cEnvironmentEffectCheck" {
            self.environment_effect_check = value.parse()?;
            Ok(format!("Set Environment Effect Check to {}", self.environment_effect_check))
        } else if config == "-cSawAnimalReset" {
            self.saw_animal_reset = value.parse()?;
            Ok(format!("Set Saw Animal Reset to {}", self.saw_animal_reset))
        } else if config == "-cInitialHappiness" {
            self.initial_happiness = value.parse()?;
            Ok(format!("Set Initial Happiness to {}", self.initial_happiness))
        } else if config == "-cMaxEnergy" {
            self.max_energy = value.parse()?;
            Ok(format!("Set Max Energy to {}", self.max_energy))
        } else if config == "-cEnergyIncrement" {
            self.energy_increment = value.parse()?;
            Ok(format!("Set Energy Increment to {}", self.energy_increment))
        } else if config == "-cEnergyThreshold" {
            self.energy_threshold = value.parse()?;
            Ok(format!("Set Energy Threshold to {}", self.energy_threshold))
        } else if config == "-cAngryEnergyChange" {
            self.angry_energy_change = value.parse()?;
            Ok(format!("Set Angry Energy Change to {}", self.angry_energy_change))
        } else if config == "-cHungerIncrement" {
            self.hunger_increment = value.parse()?;
            Ok(format!("Set Hunger Increment to {}", self.hunger_increment))
        } else if config == "-cHungerThreshold" {
            self.hunger_threshold = value.parse()?;
            Ok(format!("Set Hunger Threshold to {}", self.hunger_threshold))
        } else if config == "-cAngryFoodChange" {
            self.angry_food_change = value.parse()?;
            Ok(format!("Set Angry Food Change to {}", self.angry_food_change))
        } else if config == "-cPreferredFoodChange" {
            self.preferred_food_change = value.parse()?;
            Ok(format!("Set Preferred Food Change to {}", self.preferred_food_change))
        } else if config == "-cThirstIncrement" {
            self.thirst_increment = value.parse()?;
            Ok(format!("Set Thirst Increment to {}", self.thirst_increment))
        } else if config == "-cThirstThreshold" {
            self.thirst_threshold = value.parse()?;
            Ok(format!("Set Thirst Threshold to {}", self.thirst_threshold))
        } else if config == "-cAngryThirstChange" {
            self.angry_thirst_change = value.parse()?;
            Ok(format!("Set Angry Thirst Change to {}", self.angry_thirst_change))
        } else if config == "-cBathroomIncrement" {
            self.bathroom_increment = value.parse()?;
            Ok(format!("Set Bathroom Increment to {}", self.bathroom_increment))
        } else if config == "-cBathroomThreshold" {
            self.bathroom_threshold = value.parse()?;
            Ok(format!("Set Bathroom Threshold to {}", self.bathroom_threshold))
        } else if config == "-cAngryBathroomChange" {
            self.angry_bathroom_change = value.parse()?;
            Ok(format!("Set Angry Bathroom Change to {}", self.angry_bathroom_change))
        } else if config == "-cPriceHappy1Change" {
            self.price_happy1_change = value.parse()?;
            Ok(format!("Set Price Happy1 Change to {}", self.price_happy1_change))
        } else if config == "-cPriceAngry1Change" {
            self.price_angry1_change = value.parse()?;
            Ok(format!("Set Price Angry1 Change to {}", self.price_angry1_change))
        } else if config == "-cLeaveChanceLow" {
            self.leave_chance_low = value.parse()?;
            Ok(format!("Set Leave Chance Low to {}", self.leave_chance_low))
        } else if config == "-cLeaveChanceMed" {
            self.leave_chance_med = value.parse()?;
            Ok(format!("Set Leave Chance Med to {}", self.leave_chance_med))
        } else if config == "-cLeaveChanceHigh" {
            self.leave_chance_high = value.parse()?;
            Ok(format!("Set Leave Chance High to {}", self.leave_chance_high))
        } else if config == "-cLeaveChanceDone" {
            self.leave_chance_done = value.parse()?;
            Ok(format!("Set Leave Chance Done to {}", self.leave_chance_done))
        } else if config == "-cBuySouvenirChanceMed" {
            self.buy_souvenir_chance_med = value.parse()?;
            Ok(format!("Set Buy Souvenir Chance Med to {}", self.buy_souvenir_chance_med))
        } else if config == "-cBuySouvenirChanceHigh" {
            self.buy_souvenir_chance_high = value.parse()?;
            Ok(format!("Set Buy Souvenir Chance High to {}", self.buy_souvenir_chance_high))
        } else if config == "-cAngryTrashChange" {
            self.angry_trash_change = value.parse()?;
            Ok(format!("Set Angry Trash Change to {}", self.angry_trash_change))
        } else if config == "-cTrashInTileThreshold" {
            self.trash_in_tile_threshold = value.parse()?;
            Ok(format!("Set Trash In Tile Threshold to {}", self.trash_in_tile_threshold))
        } else if config == "-cVandalizedObjectsInTileThreshold" {
            self.vandalized_objects_in_tile_threshold = value.parse()?;
            Ok(format!("Set Vandalized Objects In Tile Threshold to {}", self.vandalized_objects_in_tile_threshold))
        } else if config == "-cAnimalInRowChange" {
            self.animal_in_row_change = value.parse()?;
            Ok(format!("Set Animal In Row Change to {}", self.animal_in_row_change))
        } else if config == "-cDifferentSpeciesChange" {
            self.different_species_change = value.parse()?;
            Ok(format!("Set Different Species Change to {}", self.different_species_change))
        } else if config == "-cDifferentSpeciesThreshold" {
            self.different_species_threshold = value.parse()?;
            Ok(format!("Set Different Species Threshold to {}", self.different_species_threshold))
        } else if config == "-cSickAnimalChange" {
            self.sick_animal_change = value.parse()?;
            Ok(format!("Set Sick Animal Change to {}", self.sick_animal_change))
        } else if config == "-cCrowdedViewingThreshold" {
            self.crowded_viewing_threshold = value.parse()?;
            Ok(format!("Set Crowded Viewing Threshold to {}", self.crowded_viewing_threshold))
        } else if config == "-cCrowdedViewingChange" {
            self.crowded_viewing_change = value.parse()?;
            Ok(format!("Set Crowded Viewing Change to {}", self.crowded_viewing_change))
        } else if config == "-cPreferredAnimalChange" {
            self.preferred_animal_change = value.parse()?;
            Ok(format!("Set Preferred Animal Change to {}", self.preferred_animal_change))
        } else if config == "-cHappyAnimalChange1" {
            self.happy_animal_change1 = value.parse()?;
            Ok(format!("Set Happy Animal Change1 to {}", self.happy_animal_change1))
        } else if config == "-cHappyAnimalChange2" {
            self.happy_animal_change2 = value.parse()?;
            Ok(format!("Set Happy Animal Change2 to {}", self.happy_animal_change2))
        } else if config == "-cAngryAnimalChange1" {
            self.angry_animal_change1 = value.parse()?;
            Ok(format!("Set Angry Animal Change1 to {}", self.angry_animal_change1))
        } else if config == "-cAngryAnimalChange2" {
            self.angry_animal_change2 = value.parse()?;
            Ok(format!("Set Angry Animal Change2 to {}", self.angry_animal_change2))
        } else if config == "-cAngryAnimalChange3" {
            self.angry_animal_change3 = value.parse()?;
            Ok(format!("Set Angry Animal Change3 to {}", self.angry_animal_change3))
        } else if config == "-cEscapedAnimalChange" {
            self.escaped_animal_change = value.parse()?;
            Ok(format!("Set Escaped Animal Change to {}", self.escaped_animal_change))
        } else if config == "-cObjectEstheticThreshold" {
            self.object_esthetic_threshold = value.parse()?;
            Ok(format!("Set Object Esthetic Threshold to {}", self.object_esthetic_threshold))
        } else if config == "-cHappyEstheticChange" {
            self.happy_esthetic_change = value.parse()?;
            Ok(format!("Set Happy Esthetic Change to {}", self.happy_esthetic_change))
        } else if config == "-cStandAndEatChange" {
            self.stand_and_eat_change = value.parse()?;
            Ok(format!("Set Stand And Eat Change to {}", self.stand_and_eat_change))
        } else if config == "-cStinkThreshold" {
            self.stink_threshold = value.parse()?;
            Ok(format!("Set Stink Threshold to {}", self.stink_threshold))
        } else if config == "-cSickChance" {
            self.sick_chance = value.parse()?;
            Ok(format!("Set Sick Chance to {}", self.sick_chance))
        } else if config == "-cSickChange" {
            self.sick_change = value.parse()?;
            Ok(format!("Set Sick Change to {}", self.sick_change))
        } else if config == "-cMimicChance" {
            self.mimic_chance = value.parse()?;
            Ok(format!("Set Mimic Chance to {}", self.mimic_chance))
        } else if config == "-cTestFenceChance" {
            self.test_fence_chance = value.parse()?;
            Ok(format!("Set Test Fence Chance to {}", self.test_fence_chance))
        } else if config == "-cZapHappinessHit" {
            self.zap_happiness_hit = value.parse()?;
            Ok(format!("Set Zap Happiness Hit to {}", self.zap_happiness_hit))
        } else if config == "-cTapWallChance" {
            self.tap_wall_chance = value.parse()?;
            Ok(format!("Set Tap Wall Chance to {}", self.tap_wall_chance))
        } else {
            Ok(self.ztunit_type.set_config(config, value)?)
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncHungerCheck: {}\ncThirstyCheck: {}\ncBathroomCheck: {}\ncLeaveZooCheck: {}\ncBuySouvenirCheck: {}\ncEnergyCheck: {}\ncChaseCheck: {}\ncTrashCheck: {}\ncLikeAnimalsCheck: {}\ncViewingAreaCheck: {}\ncEnvironmentEffectCheck: {}\ncSawAnimalReset: {}\ncInitialHappiness: {}\ncMaxEnergy: {}\ncEnergyIncrement: {}\ncEnergyThreshold: {}\ncAngryEnergyChange: {}\ncHungerIncrement: {}\ncHungerThreshold: {}\ncAngryFoodChange: {}\ncPreferredFoodChange: {}\ncThirstIncrement: {}\ncThirstThreshold: {}\ncAngryThirstChange: {}\ncBathroomIncrement: {}\ncBathroomThreshold: {}\ncAngryBathroomChange: {}\ncPriceHappy1Change: {}\ncPriceAngry1Change: {}\ncLeaveChanceLow: {}\ncLeaveChanceMed: {}\ncLeaveChanceHigh: {}\ncLeaveChanceDone: {}\ncBuySouvenirChanceMed: {}\ncBuySouvenirChanceHigh: {}\ncAngryTrashChange: {}\ncTrashInTileThreshold: {}\ncVandalizedObjectsInTileThreshold: {}\ncAnimalInRowChange: {}\ncDifferentSpeciesChange: {}\ncDifferentSpeciesThreshold: {}\ncSickAnimalChange: {}\ncCrowdedViewingThreshold: {}\ncCrowdedViewingChange: {}\ncPreferredAnimalChange: {}\ncHappyAnimalChange1: {}\ncHappyAnimalChange2: {}\ncAngryAnimalChange1: {}\ncAngryAnimalChange2: {}\ncAngryAnimalChange3: {}\ncEscapedAnimalChange: {}\ncObjectEstheticThreshold: {}\ncHappyEstheticChange: {}\ncStandAndEatChange: {}\ncStinkThreshold: {}\ncSickChance: {}\ncSickChange: {}\ncMimicChance: {}\ncTestFenceChance: {}\ncZapHappinessHit: {}\ncTapWallChance: {}\n",
        self.ztunit_type.print_config_integers(),
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
    
    fn print_config_floats(&self) -> String {
        self.ztunit_type.print_config_floats()
    }
    
    fn print_config_strings(&self) -> String {
        self.ztunit_type.print_config_strings()
    }
    
    fn print_config_details(&self) -> String {
        self.ztunit_type.print_config_details()
    }
}

impl Deref for ZTGuestType {
    type Target = ZTUnitType;
    fn deref(&self) -> &Self::Target {
        &self.ztunit_type
    }
}

// ------------ Custom Command Implementation ------------ //

fn command_sel_type(args: Vec<&str>) -> Result<String, CommandError> {
    let entity_type_address = get_selected_entity_type_address();
    // let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    // let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
    if entity_type_address == 0 {
        return Err(CommandError::new("No entity selected".to_string()));
    }

    // let Some(entity_type) = BFEntityType::new(entity_type_address) else {
    //     return Err(CommandError::new("Failed to create entity type".to_string()));
    // }; // create a copied instance of the entity type

    let Ok(mut entity_type) = get_bfentitytype(entity_type_address.clone()) else {
        return Err(CommandError::new("Failed to create entity type".to_string()));
    };
                                                                       
    if args.is_empty() {
        Ok(entity_type.print_config_details())
    } else if args[0] == "-v" {                 // if -v flag is used, print the entity type configuration and other details
        info!("Printing configuration for entity type at address {:#x}", entity_type_address as u32);
        // print the entity type configuration for the selected entity type
        Ok(entity_type.print_config())
    } else if args.len() == 2 {
        // parse the subargs for the entity type
        Ok(entity_type.set_config(args[0], args[1])?)
    } else {
        Ok("Invalid argument".to_string())
    }
}

fn print_info_image_name(entity_type: &BFEntityType, config: &mut String) {
    info!("Checking for cInfoImageName...");
    // TODO: move cInfoImageName to a separate struct (probably ZTSceneryType). crashes when trying to access it from guests
    if entity_type.get_info_image_name() != "" {
        info!("Entity type has cInfoImageName: {}", entity_type.get_info_image_name());
        config.push_str("\n[Characteristics/Strings]\n");
        config.push_str(&entity_type.get_info_image_name());
    }
}

fn get_bfentitytype(address: u32) -> Result<Box<dyn EntityType>, String> {
    // let entity: Box<&mut dyn EntityType> = match ZTEntityTypeClass::from(address) { // create a copied instance of the entity type
    let entity: Box<dyn EntityType> = match ZTEntityTypeClass::from(address) { // create a copied instance of the entity type
        ZTEntityTypeClass::Animal => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Ambient => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Guest => Box::new(get_from_memory::<ZTGuestType>(address)),
        ZTEntityTypeClass::Fences => Box::new(get_from_memory::<ZTFenceType>(address)),
        ZTEntityTypeClass::TourGuide => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Building => Box::new(get_from_memory::<ZTBuildingType>(address)),
        ZTEntityTypeClass::Scenery => Box::new(get_from_memory::<ZTSceneryType>(address)),
        ZTEntityTypeClass::Food => Box::new(get_from_memory::<ZTFoodType>(address)),
        ZTEntityTypeClass::TankFilter => Box::new(get_from_memory::<ZTTankFilterType>(address)),
        ZTEntityTypeClass::Path => Box::new(get_from_memory::<ZTPathType>(address)),
        ZTEntityTypeClass::Rubble => Box::new(get_from_memory::<ZTRubbleType>(address)),
        ZTEntityTypeClass::TankWall => Box::new(get_from_memory::<ZTTankWallType>(address)),
        ZTEntityTypeClass::Keeper => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::MaintenanceWorker => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Drt => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Unknown => return Err("Unknown entity type".to_string()),
    };
    Ok(entity)
}

// initializes the custom command
pub fn init() {
    add_to_command_register("sel_type".to_string(), command_sel_type);
}
