#![allow(dead_code)]
#![feature(lazy_cell)]

mod rpc_hooks;

use retour::GenericDetour;
use tracing::{error, info};
#[cfg(target_os = "windows")]
use windows::Win32::System::{SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH}, Console::{AllocConsole, FreeConsole}};

use std::sync::LazyLock;
use std::collections::HashSet;

// pub static LOCATIONS_HABITATS_RESOURCE_MAP: LazyLock<HashSet<GenericDetour<Any>>> = LazyLock::new(|| HashSet::new());

// use retour_utils::hook_module;

#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
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

            unsafe { zoo_main::init_detours() }.is_err().then(|| {
                error!("Error initialising zoo_main detours");
            });

            // info!("DllMain: DLL_PROCESS_ATTACH: {}, {} {}", module, reason, _reserved);
        }
        DLL_PROCESS_DETACH => {
            // info!("DllMain: DLL_PROCESS_DETACH: {}, {} {}", module, reason, _reserved);
        }
        DLL_THREAD_ATTACH => {
            // info!("DllMain: DLL_THREAD_ATTACH: {}, {} {}", module, reason, _reserved);
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


fn init_console() -> windows::core::Result<()> {
        // Free the current console
        unsafe { FreeConsole()? };

        // Allocate a new console
        unsafe { AllocConsole()? };

        Ok(())
}

// #[hook_module("zoo.exe")]
// mod zoo_main {

//     // use crate::rpc_hooks::{show_int, hello_world};

//     // We do our initialization slightly later than openzt because we pause here, so we want dll resources to already be initialized.
//     #[hook(unsafe extern "thiscall" LoadLangDLLs, offset = 0x00137333)]
//     fn load_lang_dlls(this: u32) -> u32 {
//         // info!("LoadLangDLLs called with this: {}", this);
//         // Call the original function
//         let _result = unsafe { LoadLangDLLs.call(this) };
//         // Do any additional processing if needed

//         // let mut srv_fun = lrpc::Fun::new();

//         // srv_fun.regist("show_int", show_int);
//         // srv_fun.regist("hello_world", hello_world);


//         // lrpc::service(srv_fun, "0.0.0.0:9009");
//         std::process::exit(1);
//     }
// }

mod zoo_main {
    use tracing::info;
    use crate::rpc_hooks::{show_int, hello_world};
    use retour::static_detour;

    fn load_lang_dlls(this: u32) -> u32 {
        info!("LoadLangDLLs called with this: {}", this);
        // Call the original function
        let _result = unsafe { LoadLangDLLs.call(this) };

        let mut srv_fun = lrpc::Fun::new();

        srv_fun.regist("show_int", show_int);
        srv_fun.regist("hello_world", hello_world);

        lrpc::service(srv_fun, "0.0.0.0:9009");

        std::process::exit(1);
    }

    // #[allow(unused)]
    // pub const MODULE_NAME: &str = "zoo.exe";

    static_detour! {
        static LoadLangDLLs: unsafe extern "thiscall" fn(u32) -> u32;
    }

    // #[allow(non_upper_case_globals)]
    // static LoadLangDLLs: ::retour::StaticDetour<
    //     unsafe extern "thiscall" fn(u32) -> u32,
    // > = {
    //         #[inline(never)]
    //         #[allow(unused_unsafe)]
    //         unsafe extern "thiscall" fn __ffi_detour(this: u32) -> u32 {
    //             #[allow(unused_unsafe)] (LoadLangDLLs.__detour())(this)
    //     }
    //     ::retour::StaticDetour::__new(__ffi_detour)
    // };

    pub unsafe fn init_detours() -> Result<(), retour_utils::Error> {
            // ::retour_utils::init_detour(
                // ::retour_utils::LookupData::from_offset("zoo.exe", 0x00137333),
            // |addr| {
                    LoadLangDLLs
                        .initialize(::retour::Function::from_ptr(0x00537333 as *const ()), load_lang_dlls)?
                        .enable()?;
                    // Ok(())
            // },
        // )?;
        Ok(())
    }
}


// static_detour! {
//     static Test: fn(i32) -> i32;
// }

//   #[allow(non_upper_case_globals)]
// static Test: ::retour::StaticDetour<fn(i32) -> i32> = {
//         #[inline(never)]
//         #[allow(unused_unsafe)]
//         fn __ffi_detour(__arg_0: i32) -> i32 {
//             #[allow(unused_unsafe)] (Test.__detour())(__arg_0)
//     }
//     ::retour::StaticDetour::__new(__ffi_detour)
// };

struct FunctionDef<T> {
    pub address: u32,
    pub function: T,
}

struct DetourWrapper<T> where T: retour::Function{
    pub detour: GenericDetour<T>,
}

trait Tobble {
    fn enable(&mut self) -> Result<(), retour::Error>;
    fn disable(&mut self) -> Result<(), retour::Error>;
    fn is_enabled(&self) -> bool;
}

impl<T> Tobble for FunctionDef<T> {
    fn enable(&mut self) -> Result<(), retour::Error> {
        self.detour.enable()
    }

    fn disable(&mut self) -> Result<(), retour::Error> {
        self.detour.disable()
    }

    fn is_enabled(&self) -> bool {
        self.detour.is_enabled()
    }
}