#![feature(let_chains)] // Remove when upgrading to Rust 1.88 2024 edition
#![feature(lazy_cell)] // Remove when upgrading to Rust 1.88 2024 edition (need to figure out what is causing the crash when creating a thread in dll)
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
pub mod ztworldmgr;

mod resource_manager;

/// Reading and changing the state of the UI, contains hooks for UI elements and some basic UI manipulation functions.
mod ztui;

/// Assembly patches and functions to fix bugs in the vanilla game.
///
/// Currently fixes a crash when a maintenance worker tries to fix a
/// fence 1 tile away from the edge of the map, and a bug where the
/// game crashes if a zoo wall that is one tile away from the edge
/// of the map is deleted.
#[cfg(target_os = "windows")]
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

/// ztmapview is the main view in zoo tycoon, all map interaction is done through this class.
pub mod ztmapview;

/// zthabitatmgr module has commands to interact with habitats/exhibits/tanks via the vanilla ZTHabitatMgr class.
mod zthabitatmgr;

mod experimental;

/// Patches in the current OpenZT build version into the game's version string.
mod version;

// TODO: Move this to resource_manager/openzt_mods
/// OpenZT mod structs
mod mods;

/// Utility functions for working with the game's memory, including reading and writing memory, and patching the game's assembly.
/// Common structs like ZTString are also defined here
mod util;

/// Loads settings from the zoo.ini file and commands/functions for reading and writing settings during runtime
mod settings;

#[cfg(target_os = "windows")]
use windows::Win32::System::{SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH}, Console::{AllocConsole, FreeConsole}};

use openzt_detour_macro::detour_mod;

use tracing::info;

#[cfg(target_os = "windows")]
#[detour_mod]
mod zoo_init {
    use super::*;
    use openzt_detour::LOAD_LANG_DLLS;

    // Note(finn): We hook the LoadRes function to perform some later initialization steps. Starting
    //  the console starts a new thead which is not recommended in the DllMain function.
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn load_res_dlls(this: u32) -> u32 {
        match init_console() {
            Ok(_) => {
                let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();
                tracing_subscriber::fmt().with_ansi(enable_ansi).init();
            },
            Err(e) => {
                info!("Failed to initialize console: {}", e);
                return 0; // Return 0 to indicate failure
            }
        }


        // Initialize stable modules
        command_console::init();
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
            experimental::init();
            ztmapview::init();
            zthabitatmgr::init();
        }
        unsafe { LOAD_LANG_DLLS_DETOUR.call(this) }
    }
}

#[cfg(target_os = "windows")]
pub fn init() {
    // Initialize the detours
    unsafe {
        zoo_init::init_detours().expect("Failed to initialize detours");
    }
}

#[cfg(target_os = "windows")]
fn init_console() -> windows::core::Result<()> {
        // Free the current console
        unsafe { FreeConsole()? };

        // Allocate a new console
        unsafe { AllocConsole()? };

        Ok(())
}


#[cfg(test)]
mod tests {
    use lrpc::*;
    use tracing::info;
    use std::sync::LazyLock;
    // Use parking_lot for a Mutex that recovers from panics
    use parking_lot::Mutex;

    static GRPC_CONNECTION: LazyLock<Mutex<Connection>> = LazyLock::new(|| Mutex::new(
        {
            let port = std::env::var("OPENZT_RPC_PORT").unwrap_or_else(|_| "9009".to_string());
            let addr = format!("127.0.0.1:{}", port);
    
            info!("Connecting to RPC server at {}", addr);
            Connection::new(&addr)
        }
    ));
   
    macro_rules! rpc_test {
        (fn $name: ident($arg:ident : $arg_type:ty) {
            $( $body:stmt );* $(;)?
        }) => {
            #[test]
            fn $name() {
                let mut conn = GRPC_CONNECTION.lock();
                (|$arg: $arg_type| {
                    info!("Running test case: {}", stringify!($name));
                    $(
                        $body
                    )*
                })(&mut conn);
            }
        };
    }

    rpc_test! {
        fn hello_world_test(conn: &mut Connection) {
            let response: String = conn.invoke(fun!("hello_world", "World".to_string())).unwrap();
            assert_eq!(response, "Hello, World!");
        }
    }
}