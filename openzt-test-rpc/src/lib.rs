#![allow(dead_code)]

use tracing::{error, info};
#[cfg(target_os = "windows")]
use windows::Win32::System::{SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH}, Console::{AllocConsole, FreeConsole}};

use lrpc::*;

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
    use tracing::info;

    #[hook(unsafe extern "stdcall" WinMain, offset = 0x0001a8bc)]
    fn win_main(hInstance: u32, hPrevInstance: u32, lpCmdLine: u32, nShowCm: u32) -> u32 {
        info!("###### WinMain: {} {} {} {}", hInstance, hPrevInstance, lpCmdLine, nShowCm);
        let result = unsafe { WinMain.call(hInstance, hPrevInstance, lpCmdLine, nShowCm) };
        result
    }

    #[hook(unsafe extern "thiscall" LoadLangDLLs, offset = 0x00137333)]
    fn load_lang_dlls(this: u32) -> u32 {
        info!("LoadLangDLLs called with this: {}", this);
        // Call the original function
        let result = unsafe { LoadLangDLLs.call(this) };
        // Do any additional processing if needed
        result
    }
}