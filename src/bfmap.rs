use tracing::info;

use crate::console::add_to_command_register;

use crate::ztworldmgr::read_zt_world_mgr_ptr;

use retour_utils::hook_module;

use strum_macros::EnumString;
use std::str::FromStr;

use num_enum::FromPrimitive;

#[derive(Debug, PartialEq, Eq, Clone, EnumString)]
#[repr(u32)]
enum ZoomLevel {
    #[strum(serialize = "one", serialize = "1")]
    One = 0x2,
    #[strum(serialize = "two", serialize = "2")]
    Two = 0x0,
    #[strum(serialize = "three", serialize = "3")]
    Three = 0xfffffffe,
    #[strum(serialize = "four", serialize = "4")]
    Four = 0xfffffffc,
}


#[hook_module("zoo.exe")]
pub mod zoo_zoom {
    use tracing::info;

    #[hook(unsafe extern "thiscall" BFMap_setZoom, offset = 0x000af19c)]
    fn zoo_bf_map_set_zoom(_this_ptr: u32, param_1: i32) {
        info!("BFMap::setZoom this_ptr: {:x}, zoom: {:x}", _this_ptr, param_1);
        unsafe { BFMap_setZoom.call(_this_ptr, param_1) }
    }

    #[hook(unsafe extern "thiscall" BFTile_validatePositions, offset = 0x0004a0bf)]
    fn zoo_bf_tile_validate_positions(_this_ptr: u32, param_1: u32, param_2: bool) {
        info!("BFTile::validatePositions this_ptr: {:x}, bfmap: {:x}, param_bool: {}", _this_ptr, param_1, param_2);
        unsafe { BFTile_validatePositions.call(_this_ptr, param_1, param_2) }
    }

    #[hook(unsafe extern "thiscall" BFTile_calculateShape, offset = 0x000aea12)]
    fn zoo_bf_tile_calculate_shape(_this_ptr: u32, param_1: u32) {
        info!("BFTile::calculateShape this_ptr: {:x}, bfmap: {:x}", _this_ptr, param_1);
        unsafe { BFTile_calculateShape.call(_this_ptr, param_1) }
    }

    // #[hook(unsafe extern "cdecl" scaleRect, offset = 0x0000f33d)]
    // fn zoo_scale_rect(param_1: u32, param_2: u32, param_3: u32, param_4: u32, param_5: u32, param_6: u32, param_7: u32, param_8: u32) {
    //     info!("scaleRect param_1: {:x}, param_2: {:x}, param_3: {:x}, param_4: {:x}, param_5: {:x}, param_6: {:x}, param_7: {:x}, param_8: {:x}", param_1, param_2, param_3, param_4, param_5, param_6, param_7, param_8);
    //     let return_value = unsafe { scaleRect.call(param_1, param_2, param_3, param_4, param_5, param_6, param_7, param_8) };
    //     info!("scaleRect return_value: {:x}", return_value);
    //     return_value
    // }
}

pub fn init() {
    unsafe { zoo_zoom::init_detours().unwrap() };
    add_to_command_register("set_zoom".to_string(), command_set_zoom)
}

fn command_set_zoom(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() != 1 {
        // return Err(strf!("Invalid number of arguments, expected 1 got {}", args.len()));
        return Err("Invalid number of arguments");
    }
    match ZoomLevel::from_str(args[0]) {
        Ok(zoom_level) => {
            let ptr = 0x004af19c as * const ();
            let code: extern "thiscall" fn(u32, u32) = unsafe { std::mem::transmute(ptr) };
            (code)(read_zt_world_mgr_ptr() + 8, zoom_level.clone() as u32);
            Ok(format!("Set zoom to {}", zoom_level as u32))
        },
        Err(_) => {
            // Err(strf!("Invalid zoom level: {}", args[0]))
            Err("Invalid zoom level")
        }
    }
}