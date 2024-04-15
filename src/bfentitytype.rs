// ------------ BFEntityType, Implementation, and Related Functions ------------ //
use std::ops::Deref;

use getset::{Getters, Setters};
use tracing::info;

use crate::{
    console::{add_to_command_register, CommandError},
    debug_dll::{get_from_memory, get_string_from_memory, map_from_memory},
    ztui::get_selected_entity_type_address,
    ztworldmgr::ZTEntityTypeClass,
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
    vtable: u32,                      // 0x000
    pad1: [u8; 0x034],                // ----------------------- padding: 52 bytes
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
        match config {
            "-cIconZoom" => {
                self.icon_zoom = value.parse()?;
                Ok(format!("Set cIconZoom to {}", self.icon_zoom))
            }
            "-cExpansionID" => {
                self.expansion_id = value.parse()?;
                Ok(format!("Set cExpansionID to {}", self.expansion_id))
            }
            "-cMovable" => {
                self.movable = value.parse()?;
                Ok(format!("Set cMovable to {}", self.movable))
            }
            "-cWalkable" => {
                self.walkable = value.parse()?;
                Ok(format!("Set cWalkable to {}", self.walkable))
            }
            "-cWalkableByTall" => {
                self.walkable_by_tall = value.parse()?;
                Ok(format!("Set cWalkableByTall to {}", self.walkable_by_tall))
            }
            "-cRubbleable" => {
                self.rubbleable = value.parse()?;
                Ok(format!("Set cRubbleable to {}", self.rubbleable))
            }
            "-cUseNumbersInName" => {
                self.use_numbers_in_name = value.parse()?;
                Ok(format!("Set cUseNumbersInName to {}", self.use_numbers_in_name))
            }
            "-cUsesRealShadows" => {
                self.uses_real_shadows = value.parse()?;
                Ok(format!("Set cUsesRealShadows to {}", self.uses_real_shadows))
            }
            "-cHasShadowImages" => {
                self.has_shadow_images = value.parse()?;
                Ok(format!("Set cHasShadowImages to {}", self.has_shadow_images))
            }
            "-cForceShadowBlack" => {
                self.force_shadow_black = value.parse()?;
                Ok(format!("Set cForceShadowBlack to {}", self.force_shadow_black))
            }
            "-cDrawsLate" => {
                self.draws_late = value.parse()?;
                Ok(format!("Set cDrawsLate to {}", self.draws_late))
            }
            "-cHeight" => {
                self.height = value.parse()?;
                Ok(format!("Set cHeight to {}", self.height))
            }
            "-cDepth" => {
                self.depth = value.parse()?;
                Ok(format!("Set cDepth to {}", self.depth))
            }
            "-cHasUnderwaterSection" => {
                self.has_underwater_section = value.parse()?;
                Ok(format!("Set cHasUnderwaterSection to {}", self.has_underwater_section))
            }
            "-cIsTransient" => {
                self.is_transient = value.parse()?;
                Ok(format!("Set cIsTransient to {}", self.is_transient))
            }
            "-cUsesPlacementCube" => {
                self.uses_placement_cube = value.parse()?;
                Ok(format!("Set cUsesPlacementCube to {}", self.uses_placement_cube))
            }
            "-cShow" => {
                self.show = value.parse()?;
                Ok(format!("Set cShow to {}", self.show))
            }
            "-cHitThreshold" => {
                self.hit_threshold = value.parse()?;
                Ok(format!("Set cHitThreshold to {}", self.hit_threshold))
            }
            "-cAvoidEdges" => {
                self.avoid_edges = value.parse()?;
                Ok(format!("Set cAvoidEdges to {}", self.avoid_edges))
            }
            "-cFootprintX" => {
                self.footprintx = value.parse()?;
                Ok(format!("Set cFootprintX to {}", self.footprintx))
            }
            "-cFootprintY" => {
                self.footprinty = value.parse()?;
                Ok(format!("Set cFootprintY to {}", self.footprinty))
            }
            "-cFootprintZ" => {
                self.footprintz = value.parse()?;
                Ok(format!("Set cFootprintZ to {}", self.footprintz))
            }
            "-cPlacementFootprintX" => {
                self.placement_footprintx = value.parse()?;
                Ok(format!("Set cPlacementFootprintX to {}", self.placement_footprintx))
            }
            "-cPlacementFootprintY" => {
                self.placement_footprinty = value.parse()?;
                Ok(format!("Set cPlacementFootprintY to {}", self.placement_footprinty))
            }
            "-cPlacementFootprintZ" => {
                self.placement_footprintz = value.parse()?;
                Ok(format!("Set cPlacementFootprintZ to {}", self.placement_footprintz))
            }
            "-cAvailableAtStartup" => {
                self.available_at_startup = value.parse()?;
                Ok(format!("Set cAvailableAtStartup to {}", self.available_at_startup))
            }
            _ => Err(CommandError::new(format!("Invalid configuration option: {}", config))),
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
        match config {
            "-cPurchaseCost" => {
                self.purchase_cost = value.parse()?;
                Ok(format!("Set Purchase Cost to {}", self.purchase_cost))
            }
            "-cNameID" => {
                self.name_id = value.parse()?;
                Ok(format!("Set Name ID to {}", self.name_id))
            }
            "-cHelpID" => {
                self.help_id = value.parse()?;
                Ok(format!("Set Help ID to {}", self.help_id))
            }
            "-cHabitat" => {
                self.habitat = value.parse()?;
                Ok(format!("Set Habitat to {}", self.habitat))
            }
            "-cLocation" => {
                self.location = value.parse()?;
                Ok(format!("Set Location to {}", self.location))
            }
            "-cEra" => {
                self.era = value.parse()?;
                Ok(format!("Set Era to {}", self.era))
            }
            "-cMaxFoodUnits" => {
                self.max_food_units = value.parse()?;
                Ok(format!("Set Max Food Units to {}", self.max_food_units))
            }
            "-cStink" => {
                self.stink = value.parse()?;
                Ok(format!("Set Stink to {}", self.stink))
            }
            "-cEstheticWeight" => {
                self.esthetic_weight = value.parse()?;
                Ok(format!("Set Esthetic Weight to {}", self.esthetic_weight))
            }
            "-cSelectable" => {
                self.selectable = value.parse()?;
                Ok(format!("Set Selectable to {}", self.selectable))
            }
            "-cDeletable" => {
                self.deletable = value.parse()?;
                Ok(format!("Set Deletable to {}", self.deletable))
            }
            "-cFoliage" => {
                self.foliage = value.parse()?;
                Ok(format!("Set Foliage to {}", self.foliage))
            }
            "-cAutoRotate" => {
                self.auto_rotate = value.parse()?;
                Ok(format!("Set Auto Rotate to {}", self.auto_rotate))
            }
            "-cLand" => {
                self.land = value.parse()?;
                Ok(format!("Set Land to {}", self.land))
            }
            "-cSwims" => {
                self.swims = value.parse()?;
                Ok(format!("Set Swims to {}", self.swims))
            }
            "-cUnderwater" => {
                self.underwater = value.parse()?;
                Ok(format!("Set Underwater to {}", self.underwater))
            }
            "-cSurface" => {
                self.surface = value.parse()?;
                Ok(format!("Set Surface to {}", self.surface))
            }
            "-cSubmerge" => {
                self.submerge = value.parse()?;
                Ok(format!("Set Submerge to {}", self.submerge))
            }
            "-cOnlySwims" => {
                self.only_swims = value.parse()?;
                Ok(format!("Set Only Swims to {}", self.only_swims))
            }
            "-cNeedsConfirm" => {
                self.needs_confirm = value.parse()?;
                Ok(format!("Set Needs Confirm to {}", self.needs_confirm))
            }
            "-cGawkOnlyFromFront" => {
                self.gawk_only_from_front = value.parse()?;
                Ok(format!("Set Gawk Only From Front to {}", self.gawk_only_from_front))
            }
            "-cDeadOnLand" => {
                self.dead_on_land = value.parse()?;
                Ok(format!("Set Dead On Land to {}", self.dead_on_land))
            }
            "-cDeadOnFlatWater" => {
                self.dead_on_flat_water = value.parse()?;
                Ok(format!("Set Dead On Flat Water to {}", self.dead_on_flat_water))
            }
            "-cDeadUnderwater" => {
                self.dead_underwater = value.parse()?;
                Ok(format!("Set Dead Underwater to {}", self.dead_underwater))
            }
            "-cUsesTreeRubble" => {
                self.uses_tree_rubble = value.parse()?;
                Ok(format!("Set Uses Tree Rubble to {}", self.uses_tree_rubble))
            }
            "-cForcesSceneryRubble" => {
                self.forces_scenery_rubble = value.parse()?;
                Ok(format!("Set Forces Scenery Rubble to {}", self.forces_scenery_rubble))
            }
            "-cBlocksLOS" => {
                self.blocks_los = value.parse()?;
                Ok(format!("Set Blocks LOS to {}", self.blocks_los))
            }
            _ => Ok(self.bfentitytype.set_config(config, value)?),
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
        match config {
            "-cCapacity" => {
                self.i_capacity = value.parse()?;
                Ok(format!("Set Capacity to {}", self.i_capacity))
            }
            "-cToySatisfaction" => {
                self.toy_satisfaction = value.parse()?;
                Ok(format!("Set Toy Satisfaction to {}", self.toy_satisfaction))
            }
            "-cTimeInside" => {
                self.time_inside = value.parse()?;
                Ok(format!("Set Time Inside to {}", self.time_inside))
            }
            "-cDefaultCost" => {
                self.default_cost = value.parse()?;
                Ok(format!("Set Default Cost to {}", self.default_cost))
            }
            "-cLowCost" => {
                self.low_cost = value.parse()?;
                Ok(format!("Set Low Cost to {}", self.low_cost))
            }
            "-cMedCost" => {
                self.med_cost = value.parse()?;
                Ok(format!("Set Med Cost to {}", self.med_cost))
            }
            "-cHighCost" => {
                self.high_cost = value.parse()?;
                Ok(format!("Set High Cost to {}", self.high_cost))
            }
            "-cPriceFactor" => {
                self.price_factor = value.parse()?;
                Ok(format!("Set Price Factor to {}", self.price_factor))
            }
            "-cUpkeep" => {
                self.upkeep = value.parse()?;
                Ok(format!("Set Upkeep to {}", self.upkeep))
            }
            "-cHideUser" => {
                self.hide_user = value.parse()?;
                Ok(format!("Set Hide User to {}", self.hide_user))
            }
            "-cSetLetterFacing" => {
                self.set_letter_facing = value.parse()?;
                Ok(format!("Set Set Letter Facing to {}", self.set_letter_facing))
            }
            "-cDrawUser" => {
                self.draw_user = value.parse()?;
                Ok(format!("Set Draw User to {}", self.draw_user))
            }
            "-cHideCostChange" => {
                self.hide_cost_change = value.parse()?;
                Ok(format!("Set Hide Cost Change to {}", self.hide_cost_change))
            }
            "-cHideCommerceInfo" => {
                self.hide_commerce_info = value.parse()?;
                Ok(format!("Set Hide Commerce Info to {}", self.hide_commerce_info))
            }
            "-cHideRegularInfo" => {
                self.hide_regular_info = value.parse()?;
                Ok(format!("Set Hide Regular Info to {}", self.hide_regular_info))
            }
            "-cHoldsOntoUser" => {
                self.holds_onto_user = value.parse()?;
                Ok(format!("Set Holds Onto User to {}", self.holds_onto_user))
            }
            "-cUserTracker" => {
                self.user_tracker = value.parse()?;
                Ok(format!("Set User Tracker to {}", self.user_tracker))
            }
            "-cIdler" => {
                self.idler = value.parse()?;
                Ok(format!("Set Idler to {}", self.idler))
            }
            "-cExhibitViewer" => {
                self.exhibit_viewer = value.parse()?;
                Ok(format!("Set Exhibit Viewer to {}", self.exhibit_viewer))
            }
            "-cAlternatePanelTitle" => {
                self.alternate_panel_title = value.parse()?;
                Ok(format!("Set Alternate Panel Title to {}", self.alternate_panel_title))
            }
            "-cDirectEntrance" => {
                self.direct_entrance = value.parse()?;
                Ok(format!("Set Direct Entrance to {}", self.direct_entrance))
            }
            "-cHideBuilding" => {
                self.hide_building = value.parse()?;
                Ok(format!("Set Hide Building to {}", self.hide_building))
            }
            "-cUserStaysOutside" => {
                self.user_stays_outside = value.parse()?;
                Ok(format!("Set User Stays Outside to {}", self.user_stays_outside))
            }
            "-cUserTeleportsInside" => {
                self.user_teleports_inside = value.parse()?;
                Ok(format!("Set User Teleports Inside to {}", self.user_teleports_inside))
            }
            "-cUserUsesExit" => {
                self.user_uses_exit = value.parse()?;
                Ok(format!("Set User Uses Exit to {}", self.user_uses_exit))
            }
            "-cUserUsesEntranceAsEmergencyExit" => {
                self.user_uses_entrance_as_emergency_exit = value.parse()?;
                Ok(format!(
                    "Set User Uses Entrance As Emergency Exit to {}",
                    self.user_uses_entrance_as_emergency_exit
                ))
            }
            "-cAdultChange" => {
                self.adult_change = value.parse()?;
                Ok(format!("Set Adult Change to {}", self.adult_change))
            }
            "-cChildChange" => {
                self.child_change = value.parse()?;
                Ok(format!("Set Child Change to {}", self.child_change))
            }
            "-cHungerChange" => {
                self.hunger_change = value.parse()?;
                Ok(format!("Set Hunger Change to {}", self.hunger_change))
            }
            "-cThirstChange" => {
                self.thirst_change = value.parse()?;
                Ok(format!("Set Thirst Change to {}", self.thirst_change))
            }
            "-cBathroomChange" => {
                self.bathroom_change = value.parse()?;
                Ok(format!("Set Bathroom Change to {}", self.bathroom_change))
            }
            "-cEnergyChange" => {
                self.energy_change = value.parse()?;
                Ok(format!("Set Energy Change to {}", self.energy_change))
            }
            _ => Ok(self.ztscenerytype.set_config(config, value)?),
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
        match config {
            "-cStrength" => {
                self.strength = value.parse()?;
                Ok(format!("Set Strength to {}", self.strength))
            }
            "-cLife" => {
                self.life = value.parse()?;
                Ok(format!("Set Life to {}", self.life))
            }
            "-cDecayedLife" => {
                self.decayed_life = value.parse()?;
                Ok(format!("Set Decayed Life to {}", self.decayed_life))
            }
            "-cDecayedDelta" => {
                self.decayed_delta = value.parse()?;
                Ok(format!("Set Decayed Delta to {}", self.decayed_delta))
            }
            "-cBreakSoundAtten" => {
                self.break_sound_atten = value.parse()?;
                Ok(format!("Set Break Sound Atten to {}", self.break_sound_atten))
            }
            "-cOpenSoundAtten" => {
                self.open_sound_atten = value.parse()?;
                Ok(format!("Set Open Sound Atten to {}", self.open_sound_atten))
            }
            "-cSeeThrough" => {
                self.see_through = value.parse()?;
                Ok(format!("Set See Through to {}", self.see_through))
            }
            "-cIsJumpable" => {
                self.is_jumpable = value.parse()?;
                Ok(format!("Set Is Jumpable to {}", self.is_jumpable))
            }
            "-cIsClimbable" => {
                self.is_climbable = value.parse()?;
                Ok(format!("Set Is Climbable to {}", self.is_climbable))
            }
            "-cIndestructible" => {
                self.indestructible = value.parse()?;
                Ok(format!("Set Indestructible to {}", self.indestructible))
            }
            "-cIsElectrified" => {
                self.is_electrified = value.parse()?;
                Ok(format!("Set Is Electrified to {}", self.is_electrified))
            }
            "-cNoDrawWater" => {
                self.no_draw_water = value.parse()?;
                Ok(format!("Set No Draw Water to {}", self.no_draw_water))
            }
            _ => Ok(self.ztscenerytype.set_config(config, value)?),
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
        match config {
            "-cPortalOpenSoundAtten" => {
                self.portal_open_sound_atten = value.parse()?;
                Ok(format!("Set Portal Open Sound Atten to {}", self.portal_open_sound_atten))
            }
            "-cPortalCloseSoundAtten" => {
                self.portal_close_sound_atten = value.parse()?;
                Ok(format!("Set Portal Close Sound Atten to {}", self.portal_close_sound_atten))
            }
            _ => Ok(self.ztfencetype.set_config(config, value)?),
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
        match config {
            "-cKeeperFoodType" => {
                self.keeper_food_type = value.parse()?;
                Ok(format!("Set Keeper Food Type to {}", self.keeper_food_type))
            }
            _ => Ok(self.ztscenerytype.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!(
            "{}\ncKeeperFoodType: {}\n",
            self.ztscenerytype.print_config_integers(),
            self.keeper_food_type
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
        match config {
            "-cStartingHealth" => {
                self.starting_health = value.parse()?;
                Ok(format!("Set Starting Health to {}", self.starting_health))
            }
            "-cDecayedHealth" => {
                self.decayed_health = value.parse()?;
                Ok(format!("Set Decayed Health to {}", self.decayed_health))
            }
            "-cDecayTime" => {
                self.decay_time = value.parse()?;
                Ok(format!("Set Decay Time to {}", self.decay_time))
            }
            "-cFilterDelay" => {
                self.filter_delay = value.parse()?;
                Ok(format!("Set Filter Delay to {}", self.filter_delay))
            }
            "-cFilterUpkeep" => {
                self.filter_upkeep = value.parse()?;
                Ok(format!("Set Filter Upkeep to {}", self.filter_upkeep))
            }
            "-cFilterCleanAmount" => {
                self.filter_clean_amount = value.parse()?;
                Ok(format!("Set Filter Clean Amount to {}", self.filter_clean_amount))
            }
            "-cFilterDecayedCleanAmount" => {
                self.filter_decayed_clean_amount = value.parse()?;
                Ok(format!(
                    "Set Filter Decayed Clean Amount to {}",
                    self.filter_decayed_clean_amount
                ))
            }
            "-cHealthyAtten" => {
                self.healthy_atten = value.parse()?;
                Ok(format!("Set Healthy Atten to {}", self.healthy_atten))
            }
            "-cDecayedAtten" => {
                self.decayed_atten = value.parse()?;
                Ok(format!("Set Decayed Atten to {}", self.decayed_atten))
            }
            _ => Ok(self.ztscenerytype.set_config(config, value)?),
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
        match config {
            "-cMaterial" => {
                self.material = value.parse()?;
                Ok(format!("Set Material to {}", self.material))
            }
            _ => Ok(self.ztscenerytype.set_config(config, value)?),
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
        match config {
            "-cExplosionSoundAtten" => {
                self.explosion_sound_atten = value.parse()?;
                Ok(format!("Set Explosion Sound Atten to {}", self.explosion_sound_atten))
            }
            _ => Ok(self.ztscenerytype.set_config(config, value)?),
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
        match config {
            "-cSlowRate" => {
                self.slow_rate = value.parse()?;
                Ok(format!("Set Slow Rate to {}", self.slow_rate))
            }
            "-cMediumRate" => {
                self.medium_rate = value.parse()?;
                Ok(format!("Set Medium Rate to {}", self.medium_rate))
            }
            "-cFastRate" => {
                self.fast_rate = value.parse()?;
                Ok(format!("Set Fast Rate to {}", self.fast_rate))
            }
            "-cSlowAnimSpeed" => {
                self.slow_anim_speed = value.parse()?;
                Ok(format!("Set Slow Anim Speed to {}", self.slow_anim_speed))
            }
            "-cMediumAnimSpeed" => {
                self.medium_anim_speed = value.parse()?;
                Ok(format!("Set Medium Anim Speed to {}", self.medium_anim_speed))
            }
            "-cFastAnimSpeed" => {
                self.fast_anim_speed = value.parse()?;
                Ok(format!("Set Fast Anim Speed to {}", self.fast_anim_speed))
            }
            "-cMinHeight" => {
                self.min_height = value.parse()?;
                Ok(format!("Set Min Height to {}", self.min_height))
            }
            "-cMaxHeight" => {
                self.max_height = value.parse()?;
                Ok(format!("Set Max Height to {}", self.max_height))
            }
            _ => Ok(self.bfentitytype.set_config(config, value)?),
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
        match config {
            "-cPurchaseCost" => {
                self.purchase_cost = value.parse()?;
                Ok(format!("Set Purchase Cost to {}", self.purchase_cost))
            }
            "-cNameID" => {
                self.name_id = value.parse()?;
                Ok(format!("Set Name ID to {}", self.name_id))
            }
            "-cHelpID" => {
                self.help_id = value.parse()?;
                Ok(format!("Set Help ID to {}", self.help_id))
            }
            "-cMapFootprint" => {
                self.map_footprint = value.parse()?;
                Ok(format!("Set Map Footprint to {}", self.map_footprint))
            }
            "-cSlowAnimSpeedWater" => {
                self.slow_anim_speed_water = value.parse()?;
                Ok(format!("Set Slow Anim Speed Water to {}", self.slow_anim_speed_water))
            }
            "-cMediumAnimSpeedWater" => {
                self.medium_anim_speed_water = value.parse()?;
                Ok(format!("Set Medium Anim Speed Water to {}", self.medium_anim_speed_water))
            }
            "-cFastAnimSpeedWater" => {
                self.fast_anim_speed_water = value.parse()?;
                Ok(format!("Set Fast Anim Speed Water to {}", self.fast_anim_speed_water))
            }
            "-cSwims" => {
                self.swims = value.parse()?;
                Ok(format!("Set Swims to {}", self.swims))
            }
            "-cSurface" => {
                self.surface = value.parse()?;
                Ok(format!("Set Surface to {}", self.surface))
            }
            "-cUnderwater" => {
                self.underwater = value.parse()?;
                Ok(format!("Set Underwater to {}", self.underwater))
            }
            "-cOnlyUnderwater" => {
                self.only_underwater = value.parse()?;
                Ok(format!("Set Only Underwater to {}", self.only_underwater))
            }
            "-cSkipTrickHappiness" => {
                self.skip_trick_happiness = value.parse()?;
                Ok(format!("Set Skip Trick Happiness to {}", self.skip_trick_happiness))
            }
            "-cSkipTrickChance" => {
                self.skip_trick_chance = value.parse()?;
                Ok(format!("Set Skip Trick Chance to {}", self.skip_trick_chance))
            }
            _ => Ok(self.bfunit_type.set_config(config, value)?),
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
        match config {
            "-cHungerCheck" => {
                self.hunger_check = value.parse()?;
                Ok(format!("Set Hunger Check to {}", self.hunger_check))
            }
            "-cThirstyCheck" => {
                self.thirsty_check = value.parse()?;
                Ok(format!("Set Thirsty Check to {}", self.thirsty_check))
            }
            "-cBathroomCheck" => {
                self.bathroom_check = value.parse()?;
                Ok(format!("Set Bathroom Check to {}", self.bathroom_check))
            }
            "-cLeaveZooCheck" => {
                self.leave_zoo_check = value.parse()?;
                Ok(format!("Set Leave Zoo Check to {}", self.leave_zoo_check))
            }
            "-cBuySouvenirCheck" => {
                self.buy_souvenir_check = value.parse()?;
                Ok(format!("Set Buy Souvenir Check to {}", self.buy_souvenir_check))
            }
            "-cEnergyCheck" => {
                self.energy_check = value.parse()?;
                Ok(format!("Set Energy Check to {}", self.energy_check))
            }
            "-cChaseCheck" => {
                self.chase_check = value.parse()?;
                Ok(format!("Set Chase Check to {}", self.chase_check))
            }
            "-cTrashCheck" => {
                self.trash_check = value.parse()?;
                Ok(format!("Set Trash Check to {}", self.trash_check))
            }
            "-cLikeAnimalsCheck" => {
                self.like_animals_check = value.parse()?;
                Ok(format!("Set Like Animals Check to {}", self.like_animals_check))
            }
            "-cViewingAreaCheck" => {
                self.viewing_area_check = value.parse()?;
                Ok(format!("Set Viewing Area Check to {}", self.viewing_area_check))
            }
            "-cEnvironmentEffectCheck" => {
                self.environment_effect_check = value.parse()?;
                Ok(format!("Set Environment Effect Check to {}", self.environment_effect_check))
            }
            "-cSawAnimalReset" => {
                self.saw_animal_reset = value.parse()?;
                Ok(format!("Set Saw Animal Reset to {}", self.saw_animal_reset))
            }
            "-cInitialHappiness" => {
                self.initial_happiness = value.parse()?;
                Ok(format!("Set Initial Happiness to {}", self.initial_happiness))
            }
            "-cMaxEnergy" => {
                self.max_energy = value.parse()?;
                Ok(format!("Set Max Energy to {}", self.max_energy))
            }
            "-cEnergyIncrement" => {
                self.energy_increment = value.parse()?;
                Ok(format!("Set Energy Increment to {}", self.energy_increment))
            }
            "-cEnergyThreshold" => {
                self.energy_threshold = value.parse()?;
                Ok(format!("Set Energy Threshold to {}", self.energy_threshold))
            }
            "-cAngryEnergyChange" => {
                self.angry_energy_change = value.parse()?;
                Ok(format!("Set Angry Energy Change to {}", self.angry_energy_change))
            }
            "-cHungerIncrement" => {
                self.hunger_increment = value.parse()?;
                Ok(format!("Set Hunger Increment to {}", self.hunger_increment))
            }
            "-cHungerThreshold" => {
                self.hunger_threshold = value.parse()?;
                Ok(format!("Set Hunger Threshold to {}", self.hunger_threshold))
            }
            "-cAngryFoodChange" => {
                self.angry_food_change = value.parse()?;
                Ok(format!("Set Angry Food Change to {}", self.angry_food_change))
            }
            "-cPreferredFoodChange" => {
                self.preferred_food_change = value.parse()?;
                Ok(format!("Set Preferred Food Change to {}", self.preferred_food_change))
            }
            "-cThirstIncrement" => {
                self.thirst_increment = value.parse()?;
                Ok(format!("Set Thirst Increment to {}", self.thirst_increment))
            }
            "-cThirstThreshold" => {
                self.thirst_threshold = value.parse()?;
                Ok(format!("Set Thirst Threshold to {}", self.thirst_threshold))
            }
            "-cAngryThirstChange" => {
                self.angry_thirst_change = value.parse()?;
                Ok(format!("Set Angry Thirst Change to {}", self.angry_thirst_change))
            }
            "-cBathroomIncrement" => {
                self.bathroom_increment = value.parse()?;
                Ok(format!("Set Bathroom Increment to {}", self.bathroom_increment))
            }
            "-cBathroomThreshold" => {
                self.bathroom_threshold = value.parse()?;
                Ok(format!("Set Bathroom Threshold to {}", self.bathroom_threshold))
            }
            "-cAngryBathroomChange" => {
                self.angry_bathroom_change = value.parse()?;
                Ok(format!("Set Angry Bathroom Change to {}", self.angry_bathroom_change))
            }
            "-cPriceHappy1Change" => {
                self.price_happy1_change = value.parse()?;
                Ok(format!("Set Price Happy1 Change to {}", self.price_happy1_change))
            }
            "-cPriceAngry1Change" => {
                self.price_angry1_change = value.parse()?;
                Ok(format!("Set Price Angry1 Change to {}", self.price_angry1_change))
            }
            "-cLeaveChanceLow" => {
                self.leave_chance_low = value.parse()?;
                Ok(format!("Set Leave Chance Low to {}", self.leave_chance_low))
            }
            "-cLeaveChanceMed" => {
                self.leave_chance_med = value.parse()?;
                Ok(format!("Set Leave Chance Med to {}", self.leave_chance_med))
            }
            "-cLeaveChanceHigh" => {
                self.leave_chance_high = value.parse()?;
                Ok(format!("Set Leave Chance High to {}", self.leave_chance_high))
            }
            "-cLeaveChanceDone" => {
                self.leave_chance_done = value.parse()?;
                Ok(format!("Set Leave Chance Done to {}", self.leave_chance_done))
            }
            "-cBuySouvenirChanceMed" => {
                self.buy_souvenir_chance_med = value.parse()?;
                Ok(format!("Set Buy Souvenir Chance Med to {}", self.buy_souvenir_chance_med))
            }
            "-cBuySouvenirChanceHigh" => {
                self.buy_souvenir_chance_high = value.parse()?;
                Ok(format!("Set Buy Souvenir Chance High to {}", self.buy_souvenir_chance_high))
            }
            "-cAngryTrashChange" => {
                self.angry_trash_change = value.parse()?;
                Ok(format!("Set Angry Trash Change to {}", self.angry_trash_change))
            }
            "-cTrashInTileThreshold" => {
                self.trash_in_tile_threshold = value.parse()?;
                Ok(format!("Set Trash In Tile Threshold to {}", self.trash_in_tile_threshold))
            }
            "-cVandalizedObjectsInTileThreshold" => {
                self.vandalized_objects_in_tile_threshold = value.parse()?;
                Ok(format!(
                    "Set Vandalized Objects In Tile Threshold to {}",
                    self.vandalized_objects_in_tile_threshold
                ))
            }
            "-cAnimalInRowChange" => {
                self.animal_in_row_change = value.parse()?;
                Ok(format!("Set Animal In Row Change to {}", self.animal_in_row_change))
            }
            "-cDifferentSpeciesChange" => {
                self.different_species_change = value.parse()?;
                Ok(format!("Set Different Species Change to {}", self.different_species_change))
            }
            "-cDifferentSpeciesThreshold" => {
                self.different_species_threshold = value.parse()?;
                Ok(format!(
                    "Set Different Species Threshold to {}",
                    self.different_species_threshold
                ))
            }
            "-cSickAnimalChange" => {
                self.sick_animal_change = value.parse()?;
                Ok(format!("Set Sick Animal Change to {}", self.sick_animal_change))
            }
            "-cCrowdedViewingThreshold" => {
                self.crowded_viewing_threshold = value.parse()?;
                Ok(format!("Set Crowded Viewing Threshold to {}", self.crowded_viewing_threshold))
            }
            "-cCrowdedViewingChange" => {
                self.crowded_viewing_change = value.parse()?;
                Ok(format!("Set Crowded Viewing Change to {}", self.crowded_viewing_change))
            }
            "-cPreferredAnimalChange" => {
                self.preferred_animal_change = value.parse()?;
                Ok(format!("Set Preferred Animal Change to {}", self.preferred_animal_change))
            }
            "-cHappyAnimalChange1" => {
                self.happy_animal_change1 = value.parse()?;
                Ok(format!("Set Happy Animal Change1 to {}", self.happy_animal_change1))
            }
            "-cHappyAnimalChange2" => {
                self.happy_animal_change2 = value.parse()?;
                Ok(format!("Set Happy Animal Change2 to {}", self.happy_animal_change2))
            }
            "-cAngryAnimalChange1" => {
                self.angry_animal_change1 = value.parse()?;
                Ok(format!("Set Angry Animal Change1 to {}", self.angry_animal_change1))
            }
            "-cAngryAnimalChange2" => {
                self.angry_animal_change2 = value.parse()?;
                Ok(format!("Set Angry Animal Change2 to {}", self.angry_animal_change2))
            }
            "-cAngryAnimalChange3" => {
                self.angry_animal_change3 = value.parse()?;
                Ok(format!("Set Angry Animal Change3 to {}", self.angry_animal_change3))
            }
            "-cEscapedAnimalChange" => {
                self.escaped_animal_change = value.parse()?;
                Ok(format!("Set Escaped Animal Change to {}", self.escaped_animal_change))
            }
            "-cObjectEstheticThreshold" => {
                self.object_esthetic_threshold = value.parse()?;
                Ok(format!("Set Object Esthetic Threshold to {}", self.object_esthetic_threshold))
            }
            "-cHappyEstheticChange" => {
                self.happy_esthetic_change = value.parse()?;
                Ok(format!("Set Happy Esthetic Change to {}", self.happy_esthetic_change))
            }
            "-cStandAndEatChange" => {
                self.stand_and_eat_change = value.parse()?;
                Ok(format!("Set Stand And Eat Change to {}", self.stand_and_eat_change))
            }
            "-cStinkThreshold" => {
                self.stink_threshold = value.parse()?;
                Ok(format!("Set Stink Threshold to {}", self.stink_threshold))
            }
            "-cSickChance" => {
                self.sick_chance = value.parse()?;
                Ok(format!("Set Sick Chance to {}", self.sick_chance))
            }
            "-cSickChange" => {
                self.sick_change = value.parse()?;
                Ok(format!("Set Sick Change to {}", self.sick_change))
            }
            "-cMimicChance" => {
                self.mimic_chance = value.parse()?;
                Ok(format!("Set Mimic Chance to {}", self.mimic_chance))
            }
            "-cTestFenceChance" => {
                self.test_fence_chance = value.parse()?;
                Ok(format!("Set Test Fence Chance to {}", self.test_fence_chance))
            }
            "-cZapHappinessHit" => {
                self.zap_happiness_hit = value.parse()?;
                Ok(format!("Set Zap Happiness Hit to {}", self.zap_happiness_hit))
            }
            "-cTapWallChance" => {
                self.tap_wall_chance = value.parse()?;
                Ok(format!("Set Tap Wall Chance to {}", self.tap_wall_chance))
            }
            _ => Ok(self.ztunit_type.set_config(config, value)?),
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

// ------------ ZTAnimalType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTAnimalType {
    pub ztunit_type: ZTUnitType,   // bytes: 0x188 - 0x100 = 0x88 = 136 bytes
    pad00: [u8; 0x1D8 - 0x188],    // ----------------------- padding: 72 bytes
    pub box_footprint_x: i32,      // 0x1D8
    pub box_footprint_y: i32,      // 0x1DC
    pub box_footprint_z: i32,      // 0x1E0
    pub family: i32,               // 0x1E4
    pub genus: i32,                // 0x1E8
    pad01: [u8; 0x1F0 - 0x1EC],    // ----------------------- padding: 4 bytes
    pub habitat: i32,              // 0x1F0
    pub location: i32,             // 0x1F4
    pub era: i32,                  // 0x1F8
    pub breath_threshold: i32,     // 0x1FC
    pub breath_increment: i32,     // 0x200
    pad02: [u8; 0x20C - 0x204],    // ----------------------- padding: 8 bytes
    pub hunger_threshold: i32,     // 0x20C
    pub hungry_health_change: i32, // 0x210
    pub hunger_increment: i32,     // 0x214
    pub food_unit_value: i32,      // 0x218
    pub keeper_food_units_eaten: i32, // 0x21C
    pub needed_food: i32,          // 0x220
    pub no_food_change: i32,       // 0x224
    pub initial_happiness: i32,    // 0x228
    pad04: [u8; 0x234 - 0x22C],    // ----------------------- padding: 12 bytes
    pub max_hits: i32,             // 0x234
    pad004: [u8; 0x23C - 0x238],   // ----------------------- padding: 4 bytes
    pub pct_hits: i32,             // 0x23C
    pad05: [u8; 0x248 - 0x240],    // ----------------------- padding: 8 bytes
    pub max_energy: i32,           // 0x248
    pad07: [u8; 0x250 - 0x24C],    // ----------------------- padding: 4 bytes
    pub max_dirty: i32,            // 0x250
    pub min_dirty: i32,            // 0x254
    pub sick_change: i32,          // 0x258
    pub other_animal_sick_change: i32, // 0x25C
    pub sick_chance: i32,          // 0x260
    pub sick_random_chance: i32,   // 0x264
    pub crowd: i32,                // 0x268
    pub crowd_happiness_change: i32, // 0x26C
    pub zap_happiness_change: i32, // 0x270
    pub captivity: i32,            // 0x274
    pub reproduction_chance: i32,  // 0x278
    pub reproduction_interval: i32, // 0x27C
    pub mating_type: i32,          // 0x280
    pub offspring: i32,            // 0x284
    pub keeper_frequency: i32,     // 0x288
    pad08: [u8; 0x290 - 0x28C],    // ----------------------- padding: 4 bytes
    pub not_enough_keepers_change: i32, // 0x290
    pub social: i32,               // 0x294
    pub habitat_size: i32,         // 0x298
    pub number_animals_min: i32,   // 0x29C
    pub number_animals_max: i32,   // 0x2A0
    pad09: [u8; 0x2AC - 0x2A4],    // ----------------------- padding: 8 bytes
    pub number_min_change: i32,    // 0x2AC
    pub number_max_change: i32,    // 0x2B0
    pad10: [u8; 0x2BC - 0x2B4],    // ----------------------- padding: 8 bytes
    pub habitat_preference: i32,   // 0x2BC
    pad11: [u8; 0x31C - 0x2C0],    // ----------------------- padding: 92 bytes
    pub baby_born_change: i32,     // 0x31C
    pad12: [u8; 0x320 - 0x320],    // ----------------------- padding: 4 bytes
    pub energy_increment: i32,     // 0x320
    pub energy_threshold: i32,     // 0x324
    pub dirty_increment: i32,      // 0x328
    pub dirty_threshold: i32,      // 0x32C
    pad13: [u8; 0x330 - 0x330],    // ----------------------- padding: 4 bytes
    pub sick_time: i32,            // 0x330
    pad14: [u8; 0x344 - 0x334],    // ----------------------- padding: 16 bytes
    pub baby_to_adult: i32,        // 0x344
    pad15: [u8; 0x348 - 0x348],    // ----------------------- padding: 4 bytes
    pub other_food: i32,           // 0x348
    pub tree_pref: i32,            // 0x34C
    pub rock_pref: i32,            // 0x350
    pub space_pref: i32,           // 0x354
    pub elevation_pref: i32,       // 0x358
    pub depth_min: i32,            // 0x35C
    pub depth_max: i32,            // 0x360
    pub depth_change: i32,         // 0x364
    pub salinity_change: i32,      // 0x368
    pub salinity_health_change: i32, // 0x36C
    pad16: [u8; 0x378 - 0x370],    // ----------------------- padding: 8 bytes
    pub happy_reproduce_threshold: i32, // 0x378
    pad17: [u8; 0x37C - 0x37C],    // ----------------------- padding: 4 bytes
    pub building_use_chance: i32,  // 0x37C
    pub no_mate_change: i32,       // 0x380
    pub time_death: i32,           // 0x384
    pub death_chance: i32,         // 0x388
    pub dirt_chance: i32,          // 0x38C
    pub water_needed: i32,         // 0x390
    pub underwater_needed: i32,    // 0x394
    pub land_needed: i32,          // 0x398
    pub enter_water_chance: i32,   // 0x39C
    pub enter_tank_chance: i32,    // 0x3A0
    pub enter_land_chance: i32,    // 0x3A4
    pub drink_water_chance: i32,   // 0x3A8
    pub chase_animal_chance: i32,  // 0x3AC
    pub climbs_cliffs: i32,        // 0x3B0
    pub bash_strength: i32,        // 0x3B4
    pub attractiveness: i32,       // 0x3B8
    pad18: [u8; 0x3C8 - 0x3BC],    // ----------------------- padding: 8 bytes
    pub keeper_food_type: i32,     // 0x3C8
    pub is_climber: bool,          // 0x3CC
    pub is_jumper: bool,           // 0x3CD
    pub small_zoodoo: bool,        // 0x3CE
    pub dino_zoodoo: bool,         // 0x3CF
    pub giant_zoodoo: bool,        // 0x3D0
    pub is_special_animal: bool,   // 0x3D1
    pub need_shelter: bool,        // 0x3D2
    pub need_toys: bool,           // 0x3D3
    pub babies_attack: bool,       // 0x3D4
}

impl EntityType for ZTAnimalType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        match config {
            "-cBoxFootprintX" => {
                self.box_footprint_x = value.parse()?;
                Ok(format!("Set Box Footprint X to {}", self.box_footprint_x))
            }
            "-cBoxFootprintY" => {
                self.box_footprint_y = value.parse()?;
                Ok(format!("Set Box Footprint Y to {}", self.box_footprint_y))
            }
            "-cBoxFootprintZ" => {
                self.box_footprint_z = value.parse()?;
                Ok(format!("Set Box Footprint Z to {}", self.box_footprint_z))
            }
            "-cFamily" => {
                self.family = value.parse()?;
                Ok(format!("Set Family to {}", self.family))
            }
            "-cGenus" => {
                self.genus = value.parse()?;
                Ok(format!("Set Genus to {}", self.genus))
            }
            "-cHabitat" => {
                self.habitat = value.parse()?;
                Ok(format!("Set Habitat to {}", self.habitat))
            }
            "-cLocation" => {
                self.location = value.parse()?;
                Ok(format!("Set Location to {}", self.location))
            }
            "-cEra" => {
                self.era = value.parse()?;
                Ok(format!("Set Era to {}", self.era))
            }
            "-cBreathThreshold" => {
                self.breath_threshold = value.parse()?;
                Ok(format!("Set Breath Threshold to {}", self.breath_threshold))
            }
            "-cBreathIncrement" => {
                self.breath_increment = value.parse()?;
                Ok(format!("Set Breath Increment to {}", self.breath_increment))
            }
            "-cHungerThreshold" => {
                self.hunger_threshold = value.parse()?;
                Ok(format!("Set Hunger Threshold to {}", self.hunger_threshold))
            }
            "-cHungryHealthChange" => {
                self.hungry_health_change = value.parse()?;
                Ok(format!("Set Hungry Health Change to {}", self.hungry_health_change))
            }
            "-cHungerIncrement" => {
                self.hunger_increment = value.parse()?;
                Ok(format!("Set Hunger Increment to {}", self.hunger_increment))
            }
            "-cFoodUnitValue" => {
                self.food_unit_value = value.parse()?;
                Ok(format!("Set Food Unit Value to {}", self.food_unit_value))
            }
            "-cKeeperFoodUnitsEaten" => {
                self.keeper_food_units_eaten = value.parse()?;
                Ok(format!("Set Keeper Food Units Eaten to {}", self.keeper_food_units_eaten))
            }
            "-cNeededFood" => {
                self.needed_food = value.parse()?;
                Ok(format!("Set Needed Food to {}", self.needed_food))
            }
            "-cNoFoodChange" => {
                self.no_food_change = value.parse()?;
                Ok(format!("Set No Food Change to {}", self.no_food_change))
            }
            "-cInitialHappiness" => {
                self.initial_happiness = value.parse()?;
                Ok(format!("Set Initial Happiness to {}", self.initial_happiness))
            }
            "-cMaxHits" => {
                self.max_hits = value.parse()?;
                Ok(format!("Set Max Hits to {}", self.max_hits))
            }
            "-cPctHits" => {
                self.pct_hits = value.parse()?;
                Ok(format!("Set Pct Hits to {}", self.pct_hits))
            }
            "-cMaxEnergy" => {
                self.max_energy = value.parse()?;
                Ok(format!("Set Max Energy to {}", self.max_energy))
            }
            "-cMaxDirty" => {
                self.max_dirty = value.parse()?;
                Ok(format!("Set Max Dirty to {}", self.max_dirty))
            }
            "-cMinDirty" => {
                self.min_dirty = value.parse()?;
                Ok(format!("Set Min Dirty to {}", self.min_dirty))
            }
            "-cSickChange" => {
                self.sick_change = value.parse()?;
                Ok(format!("Set Sick Change to {}", self.sick_change))
            }
            "-cOtherAnimalSickChange" => {
                self.other_animal_sick_change = value.parse()?;
                Ok(format!("Set Other Animal Sick Change to {}", self.other_animal_sick_change))
            }
            "-cSickChance" => {
                self.sick_chance = value.parse()?;
                Ok(format!("Set Sick Chance to {}", self.sick_chance))
            }
            "-cSickRandomChance" => {
                self.sick_random_chance = value.parse()?;
                Ok(format!("Set Sick Random Chance to {}", self.sick_random_chance))
            }
            "-cCrowd" => {
                self.crowd = value.parse()?;
                Ok(format!("Set Crowd to {}", self.crowd))
            }
            "-cCrowdHappinessChange" => {
                self.crowd_happiness_change = value.parse()?;
                Ok(format!("Set Crowd Happiness Change to {}", self.crowd_happiness_change))
            }
            "-cZapHappinessChange" => {
                self.zap_happiness_change = value.parse()?;
                Ok(format!("Set Zap Happiness Change to {}", self.zap_happiness_change))
            }
            "-cCaptivity" => {
                self.captivity = value.parse()?;
                Ok(format!("Set Captivity to {}", self.captivity))
            }
            "-cReproductionChance" => {
                self.reproduction_chance = value.parse()?;
                Ok(format!("Set Reproduction Chance to {}", self.reproduction_chance))
            }
            "-cReproductionInterval" => {
                self.reproduction_interval = value.parse()?;
                Ok(format!("Set Reproduction Interval to {}", self.reproduction_interval))
            }
            "-cMatingType" => {
                self.mating_type = value.parse()?;
                Ok(format!("Set Mating Type to {}", self.mating_type))
            }
            "-cOffspring" => {
                self.offspring = value.parse()?;
                Ok(format!("Set Offspring to {}", self.offspring))
            }
            "-cKeeperFrequency" => {
                self.keeper_frequency = value.parse()?;
                Ok(format!("Set Keeper Frequency to {}", self.keeper_frequency))
            }
            "-cNotEnoughKeepersChange" => {
                self.not_enough_keepers_change = value.parse()?;
                Ok(format!("Set Not Enough Keepers Change to {}", self.not_enough_keepers_change))
            }
            "-cSocial" => {
                self.social = value.parse()?;
                Ok(format!("Set Social to {}", self.social))
            }
            "-cHabitatSize" => {
                self.habitat_size = value.parse()?;
                Ok(format!("Set Habitat Size to {}", self.habitat_size))
            }
            "-cNumberAnimalsMin" => {
                self.number_animals_min = value.parse()?;
                Ok(format!("Set Number Animals Min to {}", self.number_animals_min))
            }
            "-cNumberAnimalsMax" => {
                self.number_animals_max = value.parse()?;
                Ok(format!("Set Number Animals Max to {}", self.number_animals_max))
            }
            "-cNumberMinChange" => {
                self.number_min_change = value.parse()?;
                Ok(format!("Set Number Min Change to {}", self.number_min_change))
            }
            "-cNumberMaxChange" => {
                self.number_max_change = value.parse()?;
                Ok(format!("Set Number Max Change to {}", self.number_max_change))
            }
            "-cHabitatPreference" => {
                self.habitat_preference = value.parse()?;
                Ok(format!("Set Habitat Preference to {}", self.habitat_preference))
            }
            "-cBabyBornChange" => {
                self.baby_born_change = value.parse()?;
                Ok(format!("Set Baby Born Change to {}", self.baby_born_change))
            }
            "-cEnergyIncrement" => {
                self.energy_increment = value.parse()?;
                Ok(format!("Set Energy Increment to {}", self.energy_increment))
            }
            "-cEnergyThreshold" => {
                self.energy_threshold = value.parse()?;
                Ok(format!("Set Energy Threshold to {}", self.energy_threshold))
            }
            "-cDirtyIncrement" => {
                self.dirty_increment = value.parse()?;
                Ok(format!("Set Dirty Increment to {}", self.dirty_increment))
            }
            "-cDirtyThreshold" => {
                self.dirty_threshold = value.parse()?;
                Ok(format!("Set Dirty Threshold to {}", self.dirty_threshold))
            }
            "-cSickTime" => {
                self.sick_time = value.parse()?;
                Ok(format!("Set Sick Time to {}", self.sick_time))
            }
            "-cBabyToAdult" => {
                self.baby_to_adult = value.parse()?;
                Ok(format!("Set Baby To Adult to {}", self.baby_to_adult))
            }
            "-cOtherFood" => {
                self.other_food = value.parse()?;
                Ok(format!("Set Other Food to {}", self.other_food))
            }
            "-cTreePref" => {
                self.tree_pref = value.parse()?;
                Ok(format!("Set Tree Pref to {}", self.tree_pref))
            }
            "-cRockPref" => {
                self.rock_pref = value.parse()?;
                Ok(format!("Set Rock Pref to {}", self.rock_pref))
            }
            "-cSpacePref" => {
                self.space_pref = value.parse()?;
                Ok(format!("Set Space Pref to {}", self.space_pref))
            }
            "-cElevationPref" => {
                self.elevation_pref = value.parse()?;
                Ok(format!("Set Elevation Pref to {}", self.elevation_pref))
            }
            "-cDepthMin" => {
                self.depth_min = value.parse()?;
                Ok(format!("Set Depth Min to {}", self.depth_min))
            }
            "-cDepthMax" => {
                self.depth_max = value.parse()?;
                Ok(format!("Set Depth Max to {}", self.depth_max))
            }
            "-cDepthChange" => {
                self.depth_change = value.parse()?;
                Ok(format!("Set Depth Change to {}", self.depth_change))
            }
            "-cSalinityChange" => {
                self.salinity_change = value.parse()?;
                Ok(format!("Set Salinity Change to {}", self.salinity_change))
            }
            "-cSalinityHealthChange" => {
                self.salinity_health_change = value.parse()?;
                Ok(format!("Set Salinity Health Change to {}", self.salinity_health_change))
            }
            "-cHappyReproduceThreshold" => {
                self.happy_reproduce_threshold = value.parse()?;
                Ok(format!("Set Happy Reproduce Threshold to {}", self.happy_reproduce_threshold))
            }
            "-cBuildingUseChance" => {
                self.building_use_chance = value.parse()?;
                Ok(format!("Set Building Use Chance to {}", self.building_use_chance))
            }
            "-cNoMateChange" => {
                self.no_mate_change = value.parse()?;
                Ok(format!("Set No Mate Change to {}", self.no_mate_change))
            }
            "-cTimeDeath" => {
                self.time_death = value.parse()?;
                Ok(format!("Set Time Death to {}", self.time_death))
            }
            "-cDeathChance" => {
                self.death_chance = value.parse()?;
                Ok(format!("Set Death Chance to {}", self.death_chance))
            }
            "-cDirtChance" => {
                self.dirt_chance = value.parse()?;
                Ok(format!("Set Dirt Chance to {}", self.dirt_chance))
            }
            "-cWaterNeeded" => {
                self.water_needed = value.parse()?;
                Ok(format!("Set Water Needed to {}", self.water_needed))
            }
            "-cUnderwaterNeeded" => {
                self.underwater_needed = value.parse()?;
                Ok(format!("Set Underwater Needed to {}", self.underwater_needed))
            }
            "-cLandNeeded" => {
                self.land_needed = value.parse()?;
                Ok(format!("Set Land Needed to {}", self.land_needed))
            }
            "-cEnterWaterChance" => {
                self.enter_water_chance = value.parse()?;
                Ok(format!("Set Enter Water Chance to {}", self.enter_water_chance))
            }
            "-cEnterTankChance" => {
                self.enter_tank_chance = value.parse()?;
                Ok(format!("Set Enter Tank Chance to {}", self.enter_tank_chance))
            }
            "-cEnterLandChance" => {
                self.enter_land_chance = value.parse()?;
                Ok(format!("Set Enter Land Chance to {}", self.enter_land_chance))
            }
            "-cDrinkWaterChance" => {
                self.drink_water_chance = value.parse()?;
                Ok(format!("Set Drink Water Chance to {}", self.drink_water_chance))
            }
            "-cChaseAnimalChance" => {
                self.chase_animal_chance = value.parse()?;
                Ok(format!("Set Chase Animal Chance to {}", self.chase_animal_chance))
            }
            "-cClimbsCliffs" => {
                self.climbs_cliffs = value.parse()?;
                Ok(format!("Set Climbs Cliffs to {}", self.climbs_cliffs))
            }
            "-cBashStrength" => {
                self.bash_strength = value.parse()?;
                Ok(format!("Set Bash Strength to {}", self.bash_strength))
            }
            "-cAttractiveness" => {
                self.attractiveness = value.parse()?;
                Ok(format!("Set Attractiveness to {}", self.attractiveness))
            }
            "-cKeeperFoodType" => {
                self.keeper_food_type = value.parse()?;
                Ok(format!("Set Keeper Food Type to {}", self.keeper_food_type))
            }
            "-cIsClimber" => {
                self.is_climber = value.parse()?;
                Ok(format!("Set Is Climber to {}", self.is_climber))
            }
            "-cIsJumper" => {
                self.is_jumper = value.parse()?;
                Ok(format!("Set Is Jumper to {}", self.is_jumper))
            }
            "-cSmallZoodoo" => {
                self.small_zoodoo = value.parse()?;
                Ok(format!("Set Small Zoodoo to {}", self.small_zoodoo))
            }
            "-cDinoZoodoo" => {
                self.dino_zoodoo = value.parse()?;
                Ok(format!("Set Dino Zoodoo to {}", self.dino_zoodoo))
            }
            "-cGiantZoodoo" => {
                self.giant_zoodoo = value.parse()?;
                Ok(format!("Set Giant Zoodoo to {}", self.giant_zoodoo))
            }
            "-cIsSpecialAnimal" => {
                self.is_special_animal = value.parse()?;
                Ok(format!("Set Is Special Animal to {}", self.is_special_animal))
            }
            "-cNeedShelter" => {
                self.need_shelter = value.parse()?;
                Ok(format!("Set Need Shelter to {}", self.need_shelter))
            }
            "-cNeedToys" => {
                self.need_toys = value.parse()?;
                Ok(format!("Set Need Toys to {}", self.need_toys))
            }
            "-cBabiesAttack" => {
                self.babies_attack = value.parse()?;
                Ok(format!("Set Babies Attack to {}", self.babies_attack))
            }
            _ => Ok(self.ztunit_type.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncBoxFootprintX: {}\ncBoxFootprintY: {}\ncBoxFootprintZ: {}\ncFamily: {}\ncGenus: {}\ncHabitat: {}\ncLocation: {}\ncEra: {}\ncBreathThreshold: {}\ncBreathIncrement: {}\ncHungerThreshold: {}\ncHungryHealthChange: {}\ncHungerIncrement: {}\ncFoodUnitValue: {}\ncKeeperFoodUnitsEaten: {}\ncNeededFood: {}\ncNoFoodChange: {}\ncInitialHappiness: {}\ncMaxHits: {}\ncPctHits: {}\ncMaxEnergy: {}\ncMaxDirty: {}\ncMinDirty: {}\ncSickChange: {}\ncOtherAnimalSickChange: {}\ncSickChance: {}\ncSickRandomChance: {}\ncCrowd: {}\ncCrowdHappinessChange: {}\ncZapHappinessChange: {}\ncCaptivity: {}\ncReproductionChance: {}\ncReproductionInterval: {}\ncMatingType: {}\ncOffspring: {}\ncKeeperFrequency: {}\ncNotEnoughKeepersChange: {}\ncSocial: {}\ncHabitatSize: {}\ncNumberAnimalsMin: {}\ncNumberAnimalsMax: {}\ncNumberMinChange: {}\ncNumberMaxChange: {}\ncHabitatPreference: {}\ncBabyBornChange: {}\ncEnergyIncrement: {}\ncEnergyThreshold: {}\ncDirtyIncrement: {}\ncDirtyThreshold: {}\ncSickTime: {}\ncBabyToAdult: {}\ncOtherFood: {}\ncTreePref: {}\ncRockPref: {}\ncSpacePref: {}\ncElevationPref: {}\ncDepthMin: {}\ncDepthMax: {}\ncDepthChange: {}\ncSalinityChange: {}\ncSalinityHealthChange: {}\ncHappyReproduceThreshold: {}\ncBuildingUseChance: {}\ncNoMateChange: {}\ncTimeDeath: {}\ncDeathChance: {}\ncDirtChance: {}\ncWaterNeeded: {}\ncUnderwaterNeeded: {}\ncLandNeeded: {}\ncEnterWaterChance: {}\ncEnterTankChance: {}\ncEnterLandChance: {}\ncDrinkWaterChance: {}\ncChaseAnimalChance: {}\ncClimbsCliffs: {}\ncBashStrength: {}\ncAttractiveness: {}\ncKeeperFoodType: {}\ncIsClimber: {}\ncIsJumper: {}\ncSmallZoodoo: {}\ncDinoZoodoo: {}\ncGiantZoodoo: {}\ncIsSpecialAnimal: {}\ncNeedShelter: {}\ncNeedToys: {}\ncBabiesAttack: {}\n",
        self.ztunit_type.print_config_integers(),
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
        self.water_needed as i32,
        self.underwater_needed as i32,
        self.land_needed as i32,
        self.enter_water_chance,
        self.enter_tank_chance,
        self.enter_land_chance,
        self.drink_water_chance,
        self.chase_animal_chance,
        self.climbs_cliffs,
        self.bash_strength,
        self.attractiveness,
        self.keeper_food_type,
        self.is_climber as i32,
        self.is_jumper as i32,
        self.small_zoodoo as i32,
        self.dino_zoodoo as i32,
        self.giant_zoodoo as i32,
        self.is_special_animal as i32,
        self.need_shelter as i32,
        self.need_toys as i32,
        self.babies_attack as i32,
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

impl Deref for ZTAnimalType {
    type Target = ZTUnitType;

    fn deref(&self) -> &Self::Target {
        &self.ztunit_type
    }
}

// ------------ ZTStaffType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTStaffType {
    pub ztunit_type: ZTUnitType, // bytes: 0x188 - 0x100 = 0x88 = 136 bytes
    pad01: [u8; 0x1B4 - 0x188],  // ----------------------- padding: 44 bytes
    pub work_check: i32,         // 0x1B4
    pub chase_check: i32,        // 0x1B8
    pad02: [u8; 0x1BC - 0x1BC],  // ----------------------- padding: 4 bytes
    pub monthly_cost: f32,       // 0x1BC
    // pub training_icon_name: string ptr, // 0x1D8 TODO: implement string ptr as function getter
    pad03: [u8; 0x1E8 - 0x1C0], // ----------------------- padding: 24 bytes
    pub duties_text_id: i32,    // 0x1E8
    pub weapon_range: i32,      // 0x1EC
}

impl EntityType for ZTStaffType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        match config {
            "-cWorkCheck" => {
                self.work_check = value.parse()?;
                Ok(format!("Set Work Check to {}", self.work_check))
            }
            "-cChaseCheck" => {
                self.chase_check = value.parse()?;
                Ok(format!("Set Chase Check to {}", self.chase_check))
            }
            "-cMonthlyCost" => {
                self.monthly_cost = value.parse()?;
                Ok(format!("Set Monthly Cost to {}", self.monthly_cost))
            }
            "-cDutiesTextID" => {
                self.duties_text_id = value.parse()?;
                Ok(format!("Set Duties Text ID to {}", self.duties_text_id))
            }
            "-cWeaponRange" => {
                self.weapon_range = value.parse()?;
                Ok(format!("Set Weapon Range to {}", self.weapon_range))
            }
            _ => Ok(self.ztunit_type.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncWorkCheck: {}\ncChaseCheck: {}\ncMonthlyCost: {}\ncDutiesTextID: {}\ncWeaponRange: {}\n",
        self.ztunit_type.print_config_integers(),
        self.work_check,
        self.chase_check,
        self.monthly_cost,
        self.duties_text_id,
        self.weapon_range,
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

impl Deref for ZTStaffType {
    type Target = ZTUnitType;

    fn deref(&self) -> &Self::Target {
        &self.ztunit_type
    }
}

// ------------ ZTMaintType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTMaintType {
    pub ztstaff_type: ZTStaffType, // bytes: 0x1F0 - 0x1B4 = 0x3C = 60 bytes
    pad01: [u8; 0x1F4 - 0x1F0],    // ----------------------- padding: 4 bytes
    pub clean_trash_radius: i32,   // 0x1F4
    pub fix_fence_modifier: i32,   // 0x1F8
    pub clear_invalid_list_interval: i32, // 0x1FC
}

impl EntityType for ZTMaintType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        match config {
            "-cCleanTrashRadius" => {
                self.clean_trash_radius = value.parse()?;
                Ok(format!("Set Clean Trash Radius to {}", self.clean_trash_radius))
            }
            "-cFixFenceModifier" => {
                self.fix_fence_modifier = value.parse()?;
                Ok(format!("Set Fix Fence Modifier to {}", self.fix_fence_modifier))
            }
            "-cClearInvalidListInterval" => {
                self.clear_invalid_list_interval = value.parse()?;
                Ok(format!(
                    "Set Clear Invalid List Interval to {}",
                    self.clear_invalid_list_interval
                ))
            }
            _ => Ok(self.ztstaff_type.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!(
            "{}\ncCleanTrashRadius: {}\ncFixFenceModifier: {}\ncClearInvalidListInterval: {}\n",
            self.ztstaff_type.print_config_integers(),
            self.clean_trash_radius,
            self.fix_fence_modifier,
            self.clear_invalid_list_interval,
        )
    }

    fn print_config_floats(&self) -> String {
        self.ztstaff_type.print_config_floats()
    }

    fn print_config_strings(&self) -> String {
        self.ztstaff_type.print_config_strings()
    }

    fn print_config_details(&self) -> String {
        self.ztstaff_type.print_config_details()
    }
}

impl Deref for ZTMaintType {
    type Target = ZTStaffType;

    fn deref(&self) -> &Self::Target {
        &self.ztstaff_type
    }
}

// ------------ ZTHelicopterType, Implementation, and Related Functions ------------ //

// TODO: DRT staff are not selectable in-game, so this struct needs a bit more testing to ensure it works as expected.
// For now, assumptions are that the offets are correct and the struct is implemented correctly.

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTHelicopterType {
    pub ztstaff_type: ZTStaffType, // bytes: 0x1F0 - 0x1B4 = 0x3C = 60 bytes
    pad01: [u8; 0x1F4 - 0x1F0],    // ----------------------- padding: 4 bytes
    // pub loop_sound_name: i32, // 0x1F4 TODO: implement string ptr as function getter
    pad02: [u8; 0x1F8 - 0x1F4], // ----------------------- padding: 4 bytes
    pub loop_sound_atten: i32,  // 0x1F8
}

impl EntityType for ZTHelicopterType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        match config {
            "-cLoopSoundAtten" => {
                self.loop_sound_atten = value.parse()?;
                Ok(format!("Set Loop Sound Atten to {}", self.loop_sound_atten))
            }
            _ => Ok(self.ztstaff_type.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!(
            "{}\ncLoopSoundAtten: {}\n",
            self.ztstaff_type.print_config_integers(),
            self.loop_sound_atten,
        )
    }

    fn print_config_floats(&self) -> String {
        self.ztstaff_type.print_config_floats()
    }

    fn print_config_strings(&self) -> String {
        self.ztstaff_type.print_config_strings()
    }

    fn print_config_details(&self) -> String {
        self.ztstaff_type.print_config_details()
    }
}

impl Deref for ZTHelicopterType {
    type Target = ZTStaffType;

    fn deref(&self) -> &Self::Target {
        &self.ztstaff_type
    }
}

// ------------ ZTGuideType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTGuideType {
    pub ztstaff_type: ZTStaffType, // bytes: 0x1F0 - 0x1B4 = 0x3C = 60 bytes
    pad01: [u8; 0x1F4 - 0x1F0],    // ----------------------- padding: 4 bytes
    pub inform_guest_time: i32,    // 0x1F4
    pub tour_guide_bonus: i32,     // 0x1F8
    pub crowd_check: i32,          // 0x1FC
    pub crowd_radius: i32,         // 0x200
    pub follow_chance: i32,        // 0x204
    pub max_group_size: i32,       // 0x208
}

impl EntityType for ZTGuideType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        match config {
            "-cInformGuestTime" => {
                self.inform_guest_time = value.parse()?;
                Ok(format!("Set Inform Guest Time to {}", self.inform_guest_time))
            }
            "-cTourGuideBonus" => {
                self.tour_guide_bonus = value.parse()?;
                Ok(format!("Set Tour Guide Bonus to {}", self.tour_guide_bonus))
            }
            "-cCrowdCheck" => {
                self.crowd_check = value.parse()?;
                Ok(format!("Set Crowd Check to {}", self.crowd_check))
            }
            "-cCrowdRadius" => {
                self.crowd_radius = value.parse()?;
                Ok(format!("Set Crowd Radius to {}", self.crowd_radius))
            }
            "-cFollowChance" => {
                self.follow_chance = value.parse()?;
                Ok(format!("Set Follow Chance to {}", self.follow_chance))
            }
            "-cMaxGroupSize" => {
                self.max_group_size = value.parse()?;
                Ok(format!("Set Max Group Size to {}", self.max_group_size))
            }
            _ => Ok(self.ztstaff_type.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncInformGuestTime: {}\ncTourGuideBonus: {}\ncCrowdCheck: {}\ncCrowdRadius: {}\ncFollowChance: {}\ncMaxGroupSize: {}\n",
                self.ztstaff_type.print_config_integers(),
        self.inform_guest_time,
        self.tour_guide_bonus,
        self.crowd_check,
        self.crowd_radius,
        self.follow_chance,
        self.max_group_size,
        )
    }

    fn print_config_floats(&self) -> String {
        self.ztstaff_type.print_config_floats()
    }

    fn print_config_strings(&self) -> String {
        self.ztstaff_type.print_config_strings()
    }

    fn print_config_details(&self) -> String {
        self.ztstaff_type.print_config_details()
    }
}

impl Deref for ZTGuideType {
    type Target = ZTStaffType;

    fn deref(&self) -> &Self::Target {
        &self.ztstaff_type
    }
}

// ------------ ZTKeeperType, Implementation, and Related Functions ------------ //

#[derive(Debug, Getters, Setters)]
#[repr(C)]
struct ZTKeeperType {
    pub ztstaff_type: ZTStaffType, // bytes: 0x1F0 - 0x1B4 = 0x3C = 60 bytes
    pad01: [u8; 0x1F4 - 0x1F0],    // ----------------------- padding: 4 bytes
    pub food_units_second: i32,    // 0x1F4
    pub clean_time: i32,           // 0x1F8
    pub heal_units_second: i32,    // 0x1FC
    pub food_per_tile: i32,        // 0x200
    // pub sickly_animal_pct: i32, // 0x6386F8
    pub clean_tank_pct: i32, // 0x204
    pub clean_tank_threshold: i32, // 0x208
                             // pub dirt: i16, // 0x20C TODO: Appears to pull from a different address, possibly a different struct
}

impl ZTKeeperType {
    // TODO: fix sickly_animal_pct, currently crashes when trying to access it
    pub fn get_sickly_animal_pct(&self) -> i32 {
        unsafe {
            let ptr = get_from_memory::<*mut i32>(0x6386F8);
            if !ptr.is_null() {
                *ptr
            } else {
                0
            }
        }
    }
}

impl EntityType for ZTKeeperType {
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, CommandError> {
        match config {
            "-cFoodUnitsSecond" => {
                self.food_units_second = value.parse()?;
                Ok(format!("Set Food Units Second to {}", self.food_units_second))
            }
            "-cCleanTime" => {
                self.clean_time = value.parse()?;
                Ok(format!("Set Clean Time to {}", self.clean_time))
            }
            "-cHealUnitsSecond" => {
                self.heal_units_second = value.parse()?;
                Ok(format!("Set Heal Units Second to {}", self.heal_units_second))
            }
            "-cFoodPerTile" => {
                self.food_per_tile = value.parse()?;
                Ok(format!("Set Food Per Tile to {}", self.food_per_tile))
            }
            "-cCleanTankPct" => {
                self.clean_tank_pct = value.parse()?;
                Ok(format!("Set Clean Tank Pct to {}", self.clean_tank_pct))
            }
            "-cCleanTankThreshold" => {
                self.clean_tank_threshold = value.parse()?;
                Ok(format!("Set Clean Tank Threshold to {}", self.clean_tank_threshold))
            }
            // else if config == "-cDirt" {
            //     self.dirt = value.parse()?;
            //     Ok(format!("Set Dirt to {}", self.dirt))
            // }
            _ => Ok(self.ztstaff_type.set_config(config, value)?),
        }
    }

    fn print_config_integers(&self) -> String {
        format!("{}\ncFoodUnitsSecond: {}\ncCleanTime: {}\ncHealUnitsSecond: {}\ncFoodPerTile: {}\ncCleanTankPct: {}\ncCleanTankThreshold: {}\n", //cDirt: {}\n", //cSicklyAnimalPct: {}\n",
            self.ztstaff_type.print_config_integers(),
        self.food_units_second,
        self.clean_time,
        self.heal_units_second,
        self.food_per_tile,
        self.clean_tank_pct,
        self.clean_tank_threshold,
        // self.dirt,
        //self.get_sickly_animal_pct(),
        )
    }

    fn print_config_floats(&self) -> String {
        self.ztstaff_type.print_config_floats()
    }

    fn print_config_strings(&self) -> String {
        self.ztstaff_type.print_config_strings()
    }

    fn print_config_details(&self) -> String {
        self.ztstaff_type.print_config_details()
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

    let entity_type = map_bfentitytype(entity_type_address)?;

    if args.is_empty() {
        Ok(entity_type.print_config_details())
    } else if args[0] == "-v" {
        // if -v flag is used, print the entity type configuration and other details
        info!(
            "Printing configuration for entity type at address {:#x}",
            entity_type_address as u32
        );
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

// This returns a dynamic trait object, which lets us call the methods of the entity type without knowing the exact type
fn get_bfentitytype(address: u32) -> Result<Box<dyn EntityType>, String> {
    // create a copied instance of the entity type
    let entity_type_vtable: u32 = get_from_memory(address);
    let entity: Box<dyn EntityType> = match ZTEntityTypeClass::from(entity_type_vtable) {
        ZTEntityTypeClass::Animal => Box::new(get_from_memory::<ZTAnimalType>(address)),
        ZTEntityTypeClass::Ambient => Box::new(get_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Guest => Box::new(get_from_memory::<ZTGuestType>(address)),
        ZTEntityTypeClass::Fences => Box::new(get_from_memory::<ZTFenceType>(address)),
        ZTEntityTypeClass::TourGuide => Box::new(get_from_memory::<ZTGuideType>(address)),
        ZTEntityTypeClass::Building => Box::new(get_from_memory::<ZTBuildingType>(address)),
        ZTEntityTypeClass::Scenery => Box::new(get_from_memory::<ZTSceneryType>(address)),
        ZTEntityTypeClass::Food => Box::new(get_from_memory::<ZTFoodType>(address)),
        ZTEntityTypeClass::TankFilter => Box::new(get_from_memory::<ZTTankFilterType>(address)),
        ZTEntityTypeClass::Path => Box::new(get_from_memory::<ZTPathType>(address)),
        ZTEntityTypeClass::Rubble => Box::new(get_from_memory::<ZTRubbleType>(address)),
        ZTEntityTypeClass::TankWall => Box::new(get_from_memory::<ZTTankWallType>(address)),
        ZTEntityTypeClass::Keeper => Box::new(get_from_memory::<ZTKeeperType>(address)),
        ZTEntityTypeClass::MaintenanceWorker => Box::new(get_from_memory::<ZTMaintType>(address)),
        ZTEntityTypeClass::Drt => Box::new(get_from_memory::<ZTHelicopterType>(address)),
        ZTEntityTypeClass::Unknown => return Err("Unknown entity type".to_string()),
    };
    Ok(entity)
}

fn map_bfentitytype(address: u32) -> Result<Box<&'static mut dyn EntityType>, String> {
    // create a copied instance of the entity type
    info!("Mapping entity type at address {:#x}", address);
    let entity_type_vtable: u32 = get_from_memory(address);
    info!("Entity type vtable: {:#x}", entity_type_vtable);
    let entity: Box<&mut dyn EntityType> = match ZTEntityTypeClass::from(entity_type_vtable) {
        ZTEntityTypeClass::Animal => Box::new(map_from_memory::<ZTAnimalType>(address)),
        ZTEntityTypeClass::Ambient => Box::new(map_from_memory::<ZTUnitType>(address)),
        ZTEntityTypeClass::Guest => Box::new(map_from_memory::<ZTGuestType>(address)),
        ZTEntityTypeClass::Fences => Box::new(map_from_memory::<ZTFenceType>(address)),
        ZTEntityTypeClass::TourGuide => Box::new(map_from_memory::<ZTGuideType>(address)),
        ZTEntityTypeClass::Building => Box::new(map_from_memory::<ZTBuildingType>(address)),
        ZTEntityTypeClass::Scenery => Box::new(map_from_memory::<ZTSceneryType>(address)),
        ZTEntityTypeClass::Food => Box::new(map_from_memory::<ZTFoodType>(address)),
        ZTEntityTypeClass::TankFilter => Box::new(map_from_memory::<ZTTankFilterType>(address)),
        ZTEntityTypeClass::Path => Box::new(map_from_memory::<ZTPathType>(address)),
        ZTEntityTypeClass::Rubble => Box::new(map_from_memory::<ZTRubbleType>(address)),
        ZTEntityTypeClass::TankWall => Box::new(map_from_memory::<ZTTankWallType>(address)),
        ZTEntityTypeClass::Keeper => Box::new(map_from_memory::<ZTKeeperType>(address)),
        ZTEntityTypeClass::MaintenanceWorker => Box::new(map_from_memory::<ZTMaintType>(address)),
        ZTEntityTypeClass::Drt => Box::new(map_from_memory::<ZTHelicopterType>(address)),
        ZTEntityTypeClass::Unknown => return Err("Unknown entity type".to_string()),
    };
    Ok(entity)
}

// initializes the custom command
pub fn init() {
    add_to_command_register("sel_type".to_string(), command_sel_type);
}
