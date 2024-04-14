// ------------ BFEntityType, Implementation, and Related Functions ------------ //
use std::ops::Deref;

// TODO: These should both be sub crates of a third crate that re-exports the trait and macro
use field_accessor_as_string::FieldAccessorAsString;
use field_accessor_as_string_trait::FieldAccessorAsStringTrait;
use getset::{Getters, Setters};
use tracing::info;

use crate::{
    console::{add_to_command_register, CommandError},
    debug_dll::{get_from_memory, get_string_from_memory},
    ztui::get_selected_entity_type,
    ztworldmgr::determine_entity_type,
};

// We use this trait to tag all structs that are considered EntityTypes
pub trait EntityType {}

// We then use a veru simple macro to implement the EntityType trait for all the structs that are considered EntityTypes
macro_rules! impl_EntityType (( $($int:ident),* ) => {$(impl EntityType for $int {})*});

#[rustfmt::skip]
impl_EntityType!(BFEntityType, ZTSceneryType, ZTBuildingType, ZTFenceType, ZTTankWallType, ZTFoodType, ZTTankFilterType, ZTPathType, ZTRubbleType, BFUnitType, ZTUnitType);

pub trait EntityTypeImpl<T>: EntityType + FieldAccessorAsStringTrait {
    // returns the instance of the EntityType struct
    fn new(address: u32) -> Option<&'static mut T> {
        unsafe {
            let ptr = get_from_memory::<*mut T>(address);
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                None
            }
        }
    }

    // TODO: Move this to private trait?
    fn set_config_inner(&mut self, config: &str, value: &str) -> Result<String, String> {
        let trimmed_field_name = config.strip_prefix('-').unwrap_or(config);
        match self.set_field(trimmed_field_name, value) {
            Ok(_) => Ok(format!("Set {} to {}", config, value)),
            Err(e) => Err(format!("Failed to set {} to {}: {}", config, value, e)),
        }
    }

    fn get_config(&self, config: &str) -> Result<String, String> {
        let trimmed_field_name = config.strip_prefix('-').unwrap_or(config);
        match self.get_field(trimmed_field_name) {
            Ok(value) => Ok(format!("{}: {}", config, value)),
            Err(e) => Err(format!("Failed to get {}: {}", config, e)),
        }
    }
}

impl<T: EntityType + FieldAccessorAsStringTrait> EntityTypeImpl<T> for T {}

#[derive(Debug, Getters, Setters)]
#[repr(C)]
pub struct BFEntityType {
    pad1: [u8; 0x038], // ----------------------- padding: 56 bytes
    pub ncolors: u32, // 0x038
    pad2: [u8; 0x050 - 0x03C], // ----------------------- padding: 20 bytes
    pub icon_zoom: bool, // 0x050
    pad3: [u8; 0x054 - 0x051], // ----------------------- padding: 3 bytes
    pub expansion_id: bool, // 0x054
    pub movable: bool, // 0x055
    pub walkable: bool, // 0x056
    pub walkable_by_tall: bool, // 0x057
    pad4: [u8; 0x059 - 0x058], // ----------------------- padding: 1 byte
    pub rubbleable: bool, // 0x059
    pad5: [u8; 0x05B - 0x05A], // ----------------------- padding: 1 byte
    pub use_numbers_in_name: bool, // 0x05B
    pub uses_real_shadows: bool, // 0x05C
    pub has_shadow_images: bool, // 0x05D
    pub force_shadow_black: bool, // 0x05E
    pad6: [u8; 0x060 - 0x05F], // ----------------------- padding: 1 byte
    pub draws_late: bool, // 0x060
    pad7: [u8; 0x064 - 0x061], // ----------------------- padding: 3 bytes
    pub height: u32,  // 0x064
    pub depth: u32,   // 0x068
    pub has_underwater_section: bool, // 0x06C
    pub is_transient: bool, // 0x06D
    pub uses_placement_cube: bool, // 0x06E
    pub show: bool,   // 0x06F
    pub hit_threshold: u32, // 0x070
    pub avoid_edges: bool, // 0x074
    pad10: [u8; 0x0B4 - 0x075], // ----------------------- padding: 47 bytes
    pub footprintx: i32, // 0x0B4
    pub footprinty: i32, // 0x0B8
    pub footprintz: i32, // 0x0BC
    pub placement_footprintx: i32, // 0x0C0
    pub placement_footprinty: i32, // 0x0C4
    pub placement_footprintz: i32, // 0x0C8
    pub available_at_startup: bool, // 0x0CC
    pad11: [u8; 0x100 - 0x0CD], // ----------------------- padding: 35 bytes
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

    // allows setting the configuration of the entity type
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            Ok(format!("Set cUseNumbersInName to {}", self.use_numbers_in_name))
        } else if config == "-cUsesRealShadows" {
            self.uses_real_shadows = value.parse::<bool>().unwrap();
            Ok(format!("Set cUsesRealShadows to {}", self.uses_real_shadows))
        } else if config == "-cHasShadowImages" {
            self.has_shadow_images = value.parse::<bool>().unwrap();
            Ok(format!("Set cHasShadowImages to {}", self.has_shadow_images))
        } else if config == "-cForceShadowBlack" {
            self.force_shadow_black = value.parse::<bool>().unwrap();
            Ok(format!("Set cForceShadowBlack to {}", self.force_shadow_black))
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
            Ok(format!("Set cHasUnderwaterSection to {}", self.has_underwater_section))
        } else if config == "-cIsTransient" {
            self.is_transient = value.parse::<bool>().unwrap();
            Ok(format!("Set cIsTransient to {}", self.is_transient))
        } else if config == "-cUsesPlacementCube" {
            self.uses_placement_cube = value.parse::<bool>().unwrap();
            Ok(format!("Set cUsesPlacementCube to {}", self.uses_placement_cube))
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
            Ok(format!("Set cPlacementFootprintX to {}", self.placement_footprintx))
        } else if config == "-cPlacementFootprintY" {
            self.placement_footprinty = value.parse::<i32>().unwrap();
            Ok(format!("Set cPlacementFootprintY to {}", self.placement_footprinty))
        } else if config == "-cPlacementFootprintZ" {
            self.placement_footprintz = value.parse::<i32>().unwrap();
            Ok(format!("Set cPlacementFootprintZ to {}", self.placement_footprintz))
        } else if config == "-cAvailableAtStartup" {
            self.available_at_startup = value.parse::<bool>().unwrap();
            Ok(format!("Set cAvailableAtStartup to {}", self.available_at_startup))
        } else {
            self.set_config_inner(config, value)
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
    pub purchase_cost: f32,  // 0x100
    pub name_id: u32,        // 0x104
    pub help_id: u32,        // 0x108
    pub habitat: u32,        // 0x10C
    pub location: u32,       // 0x110
    pub era: u32,            // 0x114
    pub max_food_units: u32, // 0x118
    pub stink: bool,         // 0x11C
    pad3: [u8; 0x120 - 0x11D], // ----------------------- padding: 3 bytes
    pub esthetic_weight: u32, // 0x120
    pad4: [u8; 0x128 - 0x124], // ----------------------- padding: 4 bytes
    pub selectable: bool,    // 0x128
    pub deletable: bool,     // 0x129
    pub foliage: bool,       // 0x12A
    pad6: [u8; 0x12D - 0x12B], // ----------------------- padding: 2 bytes
    pub auto_rotate: bool,   // 0x12D
    pub land: bool,          // 0x12E
    pub swims: bool,         // 0x12F
    pub underwater: bool,    // 0x130
    pub surface: bool,       // 0x131
    pub submerge: bool,      // 0x132
    pub only_swims: bool,    // 0x133
    pub needs_confirm: bool, // 0x134
    pub gawk_only_from_front: bool, // 0x135
    pub dead_on_land: bool,  // 0x136
    pub dead_on_flat_water: bool, // 0x137
    pub dead_underwater: bool, // 0x138
    pub uses_tree_rubble: bool, // 0x139
    pub forces_scenery_rubble: bool, // 0x13A
    pub blocks_los: bool,    // 0x13B
    pad7: [u8; 0x168 - 0x13C], // ----------------------- padding: 51 bytes
}

impl ZTSceneryType {
    pub fn get_info_image_name(&self) -> String {
        let obj_ptr = self as *const ZTSceneryType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x14C))
    }

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            Ok(format!("Set Gawk Only From Front to {}", self.gawk_only_from_front))
        } else if config == "-cDeadOnLand" {
            self.dead_on_land = value.parse::<bool>().unwrap();
            Ok(format!("Set Dead On Land to {}", self.dead_on_land))
        } else if config == "-cDeadOnFlatWater" {
            self.dead_on_flat_water = value.parse::<bool>().unwrap();
            Ok(format!("Set Dead On Flat Water to {}", self.dead_on_flat_water))
        } else if config == "-cDeadUnderwater" {
            self.dead_underwater = value.parse::<bool>().unwrap();
            Ok(format!("Set Dead Underwater to {}", self.dead_underwater))
        } else if config == "-cUsesTreeRubble" {
            self.uses_tree_rubble = value.parse::<bool>().unwrap();
            Ok(format!("Set Uses Tree Rubble to {}", self.uses_tree_rubble))
        } else if config == "-cForcesSceneryRubble" {
            self.forces_scenery_rubble = value.parse::<bool>().unwrap();
            Ok(format!("Set Forces Scenery Rubble to {}", self.forces_scenery_rubble))
        } else if config == "-cBlocksLOS" {
            self.blocks_los = value.parse::<bool>().unwrap();
            Ok(format!("Set Blocks LOS to {}", self.blocks_los))
        } else {
            self.set_config_inner(config, value)
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
    pad0: [u8; 0x16C - 0x168], // -------------------------- padding: 4 bytes
    pub i_capacity: i32,       // 0x16C
    pub toy_satisfaction: i32, // 0x170
    pub time_inside: i32,      // 0x174
    pub default_cost: f32,     // 0x178
    pub low_cost: f32,         // 0x17C
    pub med_cost: f32,         // 0x180
    pub high_cost: f32,        // 0x184
    pub price_factor: f32,     // 0x188
    pub upkeep: f32,           // 0x18C
    pad1: [u8; 0x194 - 0x190], // -------------------------- padding: 4 bytes
    pub hide_user: bool,       // 0x194
    pub set_letter_facing: bool, // 0x195
    pub draw_user: bool,       // 0x196
    pub hide_cost_change: bool, // 0x197
    pub hide_commerce_info: bool, // 0x198
    pub hide_regular_info: bool, // 0x199
    pub holds_onto_user: bool, // 0x19A
    pub user_tracker: bool,    // 0x19B
    pub idler: bool,           // 0x19C
    pub exhibit_viewer: bool,  // 0x19D
    pad2: [u8; 0x1A0 - 0x19E], // -------------------------- padding: 2 bytes
    pub alternate_panel_title: u32, // 0x1A0
    pub direct_entrance: bool, // 0x1A4
    pub hide_building: bool,   // 0x1A5
    pub user_stays_outside: bool, // 0x1A6
    pub user_teleports_inside: bool, // 0x1A7
    pub user_uses_exit: bool,  // 0x1A8
    pub user_uses_entrance_as_emergency_exit: bool, // 0x1A9
    pad3: [u8; 0x1B8 - 0x1AA], // -------------------------- padding: 9 bytes
    pub adult_change: i32,     // 0x1B8
    pub child_change: i32,     // 0x1BC
    pub hunger_change: i32,    // 0x1C0
    pub thirst_change: i32,    // 0x1C4
    pub bathroom_change: i32,  // 0x1C8
    pub energy_change: i32,    // 0x1CC
}

impl ZTBuildingType {
    // sets the configuration of the building type in the console
    fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            Ok(format!("Set Set Letter Facing to {}", self.set_letter_facing))
        } else if config == "-cDrawUser" {
            self.draw_user = value.parse::<bool>().unwrap();
            Ok(format!("Set Draw User to {}", self.draw_user))
        } else if config == "-cHideCostChange" {
            self.hide_cost_change = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide Cost Change to {}", self.hide_cost_change))
        } else if config == "-cHideCommerceInfo" {
            self.hide_commerce_info = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide Commerce Info to {}", self.hide_commerce_info))
        } else if config == "-cHideRegularInfo" {
            self.hide_regular_info = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide Regular Info to {}", self.hide_regular_info))
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
            Ok(format!("Set Alternate Panel Title to {}", self.alternate_panel_title))
        } else if config == "-cDirectEntrance" {
            self.direct_entrance = value.parse::<bool>().unwrap();
            Ok(format!("Set Direct Entrance to {}", self.direct_entrance))
        } else if config == "-cHideBuilding" {
            self.hide_building = value.parse::<bool>().unwrap();
            Ok(format!("Set Hide Building to {}", self.hide_building))
        } else if config == "-cUserStaysOutside" {
            self.user_stays_outside = value.parse::<bool>().unwrap();
            Ok(format!("Set User Stays Outside to {}", self.user_stays_outside))
        } else if config == "-cUserTeleportsInside" {
            self.user_teleports_inside = value.parse::<bool>().unwrap();
            Ok(format!("Set User Teleports Inside to {}", self.user_teleports_inside))
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
            self.set_config_inner(config, value)
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
    pub strength: i32,          // 0x168
    pub life: i32,              // 0x16C
    pub decayed_life: i32,      // 0x170
    pub decayed_delta: i32,     // 0x174
    pub break_sound_atten: i32, // 0x178
    pub open_sound_atten: i32,  // 0x17C
    // break_sound: String, // 0x184
    // open_sound: String, // 0x188
    pad2: [u8; 0x194 - 0x180], // ----------------------- padding: 20 bytes
    pub see_through: bool,    // 0x194
    pub is_jumpable: bool,    // 0x195
    pub is_climbable: bool,   // 0x196
    pub indestructible: bool, // 0x197
    pub is_electrified: bool, // 0x198
    pub no_draw_water: bool,  // 0x199
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

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            Ok(format!("Set Break Sound Atten to {}", self.break_sound_atten))
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
            self.set_config_inner(config, value)
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
    fn get_portal_open_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankWallType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x1A4))
    }

    fn get_portal_close_sound(&self) -> String {
        let obj_ptr = self as *const ZTTankWallType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x1B0))
    }

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            Ok(format!("Set Portal Open Sound Atten to {}", self.portal_open_sound_atten))
        } else if config == "-cPortalCloseSoundAtten" {
            self.portal_close_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Portal Close Sound Atten to {}", self.portal_close_sound_atten))
        } else {
            self.set_config_inner(config, value)
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
    pub ztscenerytype: ZTSceneryType,
    // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub keeper_food_type: u32, // 0x168
}

impl ZTFoodType {
    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
        //TODO: Add matches and handling for string values
        self.set_config_inner(config, value)
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
    pub starting_health: i32,             // 0x168
    pub decayed_health: i32,              // 0x16C
    pub decay_time: i32,                  // 0x170
    pub filter_delay: i32,                // 0x174
    pub filter_upkeep: i32,               // 0x178
    pub filter_clean_amount: i32,         // 0x17C
    pub filter_decayed_clean_amount: i32, // 0x180
    // healthy_sound: String, // 0x184
    // decayed_sound: String, // 0x190
    pad1: [u8; 0x19C - 0x184], // ----------------------- padding: 24 bytes
    pub healthy_atten: i32, // 0x19C
    pub decayed_atten: i32, // 0x1A0
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

    fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            Ok(format!("Set Filter Clean Amount to {}", self.filter_clean_amount))
        } else if config == "-cFilterDecayedCleanAmount" {
            self.filter_decayed_clean_amount = value.parse::<i32>().unwrap();
            Ok(format!("Set Filter Decayed Clean Amount to {}", self.filter_decayed_clean_amount))
        } else if config == "-cHealthyAtten" {
            self.healthy_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Healthy Atten to {}", self.healthy_atten))
        } else if config == "-cDecayedAtten" {
            self.decayed_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Decayed Atten to {}", self.decayed_atten))
        } else {
            self.set_config_inner(config, value)
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
    ztscenerytype: ZTSceneryType,
    // bytes: 0x168 - 0x000 = 0x168 = 360 bytes
    pub material: u32, // 0x168
                       // TODO: missing Shapes structure in paths. Could not find.
}

impl ZTPathType {
    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
        //TODO: Add matches and handling for string values
        self.set_config_inner(config, value)
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

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
        // if config == "-cExplosionSound" {
        //     self.explosion_sound = value.parse::<String>().unwrap();
        //     Ok(format!("Set Explosion Sound to {}", self.explosion_sound))
        // }
        if config == "-cExplosionSoundAtten" {
            self.explosion_sound_atten = value.parse::<i32>().unwrap();
            Ok(format!("Set Explosion Sound Atten to {}", self.explosion_sound_atten))
        } else {
            self.set_config_inner(config, value)
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

impl BFUnitType {
    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            self.set_config_inner(config, value)
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
    pub swims: bool,                // 0x17C
    pub surface: bool,              // 0x17D
    pub underwater: bool,           // 0x17E
    pub only_underwater: bool,      // 0x17F
    pub skip_trick_happiness: bool, // 0x180
    pub skip_trick_chance: bool,    // 0x184
}

impl ZTUnitType {
    pub fn get_list_name(&self) -> String {
        let obj_ptr = self as *const ZTUnitType as u32;
        get_string_from_memory(get_from_memory::<u32>(obj_ptr + 0x168))
    }

    pub fn set_config(&mut self, config: &str, value: &str) -> Result<String, String> {
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
            self.set_config_inner(config, value)
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

// ------------ Custom Command Implementation ------------ //

fn command_sel_type(args: Vec<&str>) -> Result<String, CommandError> {
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let entity_type_print = get_from_memory::<u32>(entity_type_address); // convert the address to a u32 ptr for printing
    if entity_type_address == 0 {
        return Err("No entity selected").map_err(Into::into);
    }

    let entity_type = BFEntityType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
                                                                       // if -v flag is used, print the entity type configuration and other details
    if args.is_empty() {
        Ok(entity_type.print_details())
    } else if args[0] == "-v" {
        info!("Printing configuration for entity type at address {:#x}", entity_type_print);

        // print the entity type configuration for the selected entity type
        Ok(print_config_for_type())
    } else if args.len() == 2 {
        // parse the subargs for the entity type
        Ok(parse_subargs_for_type(args)?)
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

// prints the configuration for the selected entity type
fn print_config_for_type() -> String {
    let entity_type_address = get_selected_entity_type(); // grab the address of the selected entity type
    let entity_type = BFEntityType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
    let mut config: String = String::new();

    info!("Printing configuration for entity type at address {:#x}", entity_type_address);

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
    } else if class_type == "Animal"
        || class_type == "Guest"
        || class_type == "Keeper"
        || class_type == "MaintenanceWorker"
        || class_type == "TourGuide"
        || class_type == "DRT"
    {
        info!("Entity type is a ZTUnit. Printing ZTUnit type configuration.");
        let ztunit_type = ZTUnitType::new(entity_type_address).unwrap(); // create a copied instance of the entity type
        config.push_str(&ztunit_type.bfunit_type.bfentitytype.print_config_integers());
        config.push_str(&ztunit_type.bfunit_type.print_config_integers());
        config.push_str(&ztunit_type.print_config_integers());
        // config.push_str(&ztunit_type.get_list_name());

        // print_info_image_name(entity_type, &mut config);
    } else {
        info!(
            "Entity type is not a known entity type. Printing base entity type configuration only."
        );
    }

    // print [colorrep] section of the configuration - available in all entity types
    // config.push_str(&entity_type.print_colorrep());
    // info!("Colorrep printed successfully.");

    info!("Configuration printed successfully.");
    config
}

// parses the subargs for the entity type
fn parse_subargs_for_type(_args: Vec<&str>) -> Result<String, String> {
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
    } else {
        Err("Invalid configuration option".to_string())
    }
}

// initializes the custom command
pub fn init() {
    add_to_command_register("sel_type".to_string(), command_sel_type);
}
