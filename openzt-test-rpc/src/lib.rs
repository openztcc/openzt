#![allow(dead_code)]
#![feature(lazy_cell)]

mod rpc_hooks;

use openzt_detour_macro::detour_mod;
use tracing::{error, info};

#[cfg(target_os = "windows")]
use windows::Win32::System::{SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH}, Console::{AllocConsole, FreeConsole}};

#[no_mangle]
extern "system" fn DllMain(_module: u8, reason: u32, _reserved: u8) -> i32 {
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

            unsafe { detour_zoo_main::init_detours() }.is_err().then(|| {
                error!("Error initialising zoo_main detours");
            });
        }
        DLL_PROCESS_DETACH => {}
        DLL_THREAD_ATTACH => {}
        DLL_THREAD_DETACH => {}
        _ => {}
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

#[detour_mod]
mod detour_zoo_main {
    use tracing::info;
    use openzt_detour::LOAD_LANG_DLLS;
    use crate::rpc_hooks::{show_int, hello_world};

    // TODO: Fix this so it works with a crate/mod prefix
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(this: u32) -> u32 {
        info!("Detour success");

        let _result = unsafe { LOAD_LANG_DLLS_DETOUR.call(this) };

        let mut srv_fun = lrpc::Fun::new();

        srv_fun.regist("show_int", show_int);
        srv_fun.regist("hello_world", hello_world);

        lrpc::service(srv_fun, "0.0.0.0:9009");

        std::process::exit(1);
    }
}
