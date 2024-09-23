#![feature(let_chains)]
#![allow(dead_code)]

/// Reimplementation of the BFRegistry, a vanilla system used to store pointers to the ZT*Mgr classes. In theory this
/// allowed customization via zoo.ini, but in practice it appears unused.
mod bfregistry;

/// Hooks into the vanilla game's logging system to re-log messages with the default OpenZT logger.
mod capture_ztlog;

/// Basic development console, includes a server that listens for a client connection to recieve commands from,
/// functions for registering commands with a function callback and hooks so that a command is run every game update
mod command_console;

/// Commands and functions for reading entities and entity types from the ZTWorldMgr class
mod ztworldmgr;

mod resource_manager;

/// Reading and changing the state of the UI, contains hooks for UI elements and some basic UI manipulation functions.
mod ztui;

/// Assembly patches and functions to fix bugs in the vanilla game.
///
/// Currently fixes a crash when a maintenance worker tries to fix a
/// fence 1 tile away from the edge of the map, and a bug where the
/// game crashes if a zoo wall that is one tile away from the edge
/// of the map is deleted.
mod bugfix;

/// Methods for reading the vanilla ZTAdvTerrainMgr class, which contains information about terrain types.
mod ztadvterrainmgr;

/// Reimplementation of vanilla handling of Expansion Packs, including the ability to define custom expansions.
///
/// Default behaviour adds in an expansion called "Custom Content" which includes all non-vanilla entities.
/// Expanding the Expansion dropdown is also handled here.
mod expansions;

/// Reimplementation of the vanilla BFApp::loadString, has functions to add a string to the OpenZT string registry,
/// will fallback to the vanilla BFApp::loadString if the string is not found in the registry.
mod string_registry;

/// Helper methods for parsing binary data, including reading and writing binary data to and from buffers.
mod binary_parsing;

/// ZTAF Animation file format parsing, writing and some modification methods.
///
/// Based on documentation at <https://github.com/jbostoen/ZTStudio/wiki/ZT1-Graphics-Explained>
mod animation;

/// Structs that mirror ZT Entity types and their properties. Currently there are many missing fields.
mod bfentitytype;

/// ztgamemgr module has commands to interact with the live zoo stats such as cash, num animals, species, guests, etc. via the vanilla ZTGameMgr class.
mod ztgamemgr;

/// Patches in the current OpenZT build version into the game's version string.
mod version;

/// OpenZT mod structs
mod mods;

/// Utility functions for working with the game's memory, including reading and writing memory, and patching the game's assembly.
/// Common structs like ZTString are also defined here
mod util;

/// Loads settings from the zoo.ini file and commands/functions for reading and writing settings during runtime
#[cfg(feature = "ini")]
mod settings;

use tracing::info;
#[cfg(target_os = "windows")]
use windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH};

#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            // We init this first so we have a console to log to
            let console_created = command_console::init().is_ok();

            if console_created {
                let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();

                tracing_subscriber::fmt().with_ansi(enable_ansi).init();
            }

            // dll_first_load();
            info!("DllMain: DLL_PROCESS_ATTACH: {}, {} {}", module, reason, _reserved);

            // Initialize stable modules
            resource_manager::init();
            expansions::init();
            string_registry::init();
            bugfix::init();
            version::init();
            ztui::init();
            ztworldmgr::init();
            bfentitytype::init();
            settings::init();


            if cfg!(feature = "capture_ztlog") {
                use crate::capture_ztlog;
                info!("Feature 'capture_ztlog' enabled");
                capture_ztlog::init();
            }

            if cfg!(feature = "experimental") {
                info!("Feature 'experimental' enabled");
                ztadvterrainmgr::init();
                ztgamemgr::init();
            }
        }
        DLL_PROCESS_DETACH => {
            // info!("DllMain: DLL_PROCESS_DETACH: {}, {} {}", module, reason, _reserved);
        }
        DLL_THREAD_ATTACH => {
            info!("DllMain: DLL_THREAD_ATTACH: {}, {} {}", module, reason, _reserved);
        }
        DLL_THREAD_DETACH => {
            // info!("DllMain: DLL_THREAD_DETACH: {}, {} {}", module, reason, _reserved);
        }
        _ => {
            // info!("DllMain: Unknown: {}, {} {}", module, reason, _reserved);
        }
    }
    1
}

// TODO: Make sure there isn't anything useful here before removing
// #[no_mangle]
// extern "cdecl" fn load_int_from_ini(section_address: &u32, header_address: &u32, default: i32) -> u32 {
//     util::debug_logger(&format!(
//         "load_int_from_ini {:p} {:p} default: {}",
//         *section_address as *const (), *header_address as *const (), default
//     ));
//     let section = util::get_string_from_memory(*section_address);
//     let header = util::get_string_from_memory(*header_address);
//     let mut zoo_ini = Ini::new();
//     zoo_ini.load(get_ini_path()).unwrap();
//     let result = load_ini::load_int_with_default(&zoo_ini, &section, &header, default) as u32;
//     util::debug_logger(&format!("load_int_from_ini {} {} result: {}", section, header, result));
//     result
// }

// #[no_mangle]
// extern "cdecl" fn load_value_from_ini<'a>(result_address: &'a u32, section_address: &u32, header_address: &u32, default_address: &u32) -> &'a u32 {
//     util::debug_logger(&format!(
//         "load_value_from_ini {:p} {:p} default: {:p}",
//         *section_address as *const (), *header_address as *const (), *default_address as *const ()
//     ));
//     let section = util::get_string_from_memory(*section_address);
//     let header = util::get_string_from_memory(*header_address);
//     let default = util::get_string_from_memory(*default_address);
//     let mut zoo_ini = Ini::new();
//     zoo_ini.load(get_ini_path()).unwrap();
//     let result = load_ini::load_string_with_default(&zoo_ini, &section, &header, &default);

//     util::debug_logger(&format!("load_value_from_ini {} {} result: {}", section, header, result));
//     util::debug_logger(&format!("encoding string at address: {:p}", *result_address as *const ()));
//     util::save_string_to_memory(*result_address, &result);
//     result_address
// }

// fn get_ini_path() -> String {
//     let mut base_path = util::get_base_path();
//     base_path.push("zoo.ini");
//     base_path.to_str().unwrap().to_string()
// }
