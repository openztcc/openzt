use core::fmt;
use num_enum::FromPrimitive;
use tracing::info;
use retour_utils::hook_module;

use crate::{bfentitytype::ZTEntityTypeClass, util::get_from_memory};
use crate::util::save_to_memory;
use crate::zthabitatmgr::read_zt_habitat_mgr_from_memory;
use crate::ztworldmgr::{BFEntity, ZTEntity};
// use crate::{
//     util::get_from_memory,
// };


// 0049ccc3
// void __thiscall BFUIMgr::displayMessage(void *this,uint param_1,int param_2,BFTile *param_3,BFEntity *param_4,bool param_5, bool param_6)

// #[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
// #[repr(u32)]
// pub enum ZTEntityTypeClass {
//     Animal = 0x630268,
//     Ambient = 0x62e1e8,
//     Guest = 0x62e330,
//     Fences = 0x63034c,
//     TourGuide = 0x62e8ac,
//     Building = 0x6307e4,
//     Scenery = 0x6303f4,
//     Food = 0x630544,
//     TankFilter = 0x630694,
//     Path = 0x63049c,
//     Rubble = 0x63073c,
//     TankWall = 0x6305ec,
//     Keeper = 0x62e7d8,
//     MaintenanceWorker = 0x62e704,
//     Drt = 0x62e980,
//     #[num_enum(default)]
//     Unknown = 0x0,
// }


#[derive(Debug)]
#[repr(C)]
pub struct BFTile {
    padding: [u8; 0x34],
    pub x: u32, // 0x034
    pub y: u32, // 0x038
    padding_2: [u8; 0x50], // Full size 0x8c
}

impl PartialEq for BFTile {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl fmt::Display for BFTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BFTile {{ x: {}, y: {} }}", self.x, self.y)
    }
}

#[hook_module("zoo.exe")]
pub mod zoo_ztmapview {
    use tracing::info;
    use crate::zthabitatmgr::{ZTHabitatMgr, read_zt_habitat_mgr_from_memory};
    use crate::ztworldmgr::read_zt_entity_from_memory;
    use crate::util::get_from_memory;
    use crate::ztmapview::{BFTile, ZTMapView};

    // use crate::{
    //     bfregistry::{add_to_registry, get_from_registry},
    //     util::{get_from_memory, get_string_from_memory},
    // };

    //004df688
    #[hook(unsafe extern "thiscall" ZTMapView_check_tank_placement, offset = 0x000df688)]
    // fn check_tank_placement(ZTMapView *other_this, BFEntity *param_2, BFTile *param_3, int *param_4)
    fn check_tank_placement(_this: u32, temp_entity: u32, tile: u32, response_ptr: *mut u32) -> u32 {
        let entity = get_from_memory(temp_entity);

        let bf_tile = get_from_memory::<BFTile>(tile);
        
        let zt_map_view = get_from_memory::<ZTMapView>(_this);
        
        if let Err(reimplemented_result) = zt_map_view.check_tank_placement(&entity, &bf_tile) {
            info!("ZTMapView::checkTankPlacement 1 -> {:?}", reimplemented_result);
        } else {
            info!("ZTMapView::checkTankPlacement 0 -> 0");
        }

        let result = unsafe { ZTMapView_check_tank_placement.call(_this, temp_entity, tile, response_ptr) };
        info!("ZTMapView::checkTankPlacement {}, {:p} -> {}", result, response_ptr, unsafe{*response_ptr});
        result
        // 1
    }

    // #[hook(unsafe extern "thiscall" BFUIMgr_display_message, offset = 0x0009ccc3)]
    // fn prt_get(_this_prt: u32, param_1: u32, param_2: i32, param_3: u32, param_4: u32, param_5: bool, param_6: bool) {
    //     info!("BFUIMgr::displayMessage called with params: {}, {}, {}, {}, {}, {}", param_1, param_2, param_3, param_4, param_5, param_6);
    //     unsafe { BFUIMgr_display_message.call(_this_prt, param_1, param_2, param_3, param_4, param_5, param_6) };
    // }
}

pub fn init() {
    if let Err(e) = unsafe { zoo_ztmapview::init_detours() } {
        info!("Error initialising zoo_ztmapview detours: {}", e);
    };
}

pub struct ZTMapView {
    pad: [u8; 0x5ec] // Not currently using this struct, so just padding it out to the size of the class
}

#[derive(Debug, PartialEq, Eq, FromPrimitive, Clone)]
#[repr(u32)]
pub enum ErrorStringId {
    #[default]
    UnknownError = 0x0000, // Unknown error
    ObjectTooCloseToLadderOrPlatform = 0x2942, // Objects cannot be placed too close to the ladder or platform of the tank.
    ObjectCannotBePlacedInTank = 0x293f, // This object can not be placed in a tank.
    ObjectMustBePlacedInADeeperTank = 0x2940, // This object must be placed in a deeper tank.
    EggsMustBePlacedOnLand = 0x294a, // Eggs must be placed on land.
    AnimalMustBePlacedInATankWithWater = 0x293a, // This animal must be placed in a tank with water in it.
    AnimalMustBePlacedOnLand = 0x2939, // This animal doesn't swim in tanks and must be placed in a land exhibit.
    StaffCannotBePlacedInTank = 0x2943, // This staff member can not be placed in a tank.
    ShowObjectMustBePlacedInShowTank = 0x293e, // Show objects can only be placed in show exhibits.
    ObjectCannotBePlacedAgainstTankWall = 0x2967, // This object cannot be placed next to a tank wall.
    ObjectCannotBePlacedInShowTank = 0x2941, // Only show toys and animals that can do tricks can be placed in show tanks.
}

impl ZTMapView {
    pub fn check_tank_placement(&self, temp_entity: &BFEntity, tile: &BFTile) -> Result<(), ErrorStringId> {
        let habitat_mgr = read_zt_habitat_mgr_from_memory();
        let Some(habitat) = habitat_mgr.get_habitat(tile.x, tile.y) else {
            return Ok(());
        };
        let entity_class = temp_entity.entity_type_class();
        if entity_class != ZTEntityTypeClass::Keeper {
            if let Some(t) = habitat.get_gate_tile_in() {
                if temp_entity.is_on_tile(&t) {
                    return Err(ErrorStringId::ObjectTooCloseToLadderOrPlatform);
                }
            }
        }
        // match  {
        //     ZTEntityTypeClass::ZTKeeper
        // }

        Ok(())
    }
}