#![allow(dead_code)]

mod rpc_hooks;

use tracing::{error, info};
#[cfg(target_os = "windows")]
use windows::Win32::System::{SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH}, Console::{AllocConsole, FreeConsole}};


use retour_utils::hook_module;

#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            // TODO:  Hook 0x41a8bc (0x1a8bc) and do all init in there

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

            // Hook 0x41a8bc (0x1a8bc) and do all init in there
            unsafe { zoo_main::init_detours() }.is_err().then(|| {
                error!("Error initialising zoo_main detours");
            });


            // dll_first_load();
            info!("DllMain: DLL_PROCESS_ATTACH: {}, {} {}", module, reason, _reserved);
            info!("Testing dll injected!");
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


fn init_console() -> windows::core::Result<()> {
        // Free the current console
        unsafe { FreeConsole()? };

        // Allocate a new console
        unsafe { AllocConsole()? };

        Ok(())
}

#[hook_module("zoo.exe")]
mod zoo_main {

    use crate::rpc_hooks::{show_int, hello_world};

    // We do our initialization slightly later than openzt because we pause here, so we want dll resources to already be initialized.
    #[hook(unsafe extern "thiscall" LoadLangDLLs, offset = 0x00137333)]
    fn load_lang_dlls(this: u32) -> u32 {
        // info!("LoadLangDLLs called with this: {}", this);
        // Call the original function
        let result = unsafe { LoadLangDLLs.call(this) };
        // Do any additional processing if needed

        let mut srv_fun = lrpc::Fun::new();

        // srv_fun.regist("hello_world", show_int as fn(i32));
        srv_fun.regist("show_int", show_int);
        srv_fun.regist("hello_world", hello_world);


        lrpc::service(srv_fun, "0.0.0.0:9009");
        std::process::exit(0);
    }
}