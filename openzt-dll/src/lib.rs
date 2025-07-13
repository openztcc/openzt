use tracing::info;
#[cfg(target_os = "windows")]
use windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH};

use openztlib::zoo_init;

#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {

            // Initialize a hook into the WinMain function, where we can perform further initialization
            // We just unwrap here as this is a critical initialization step, and we want to panic if it fails.
            unsafe { zoo_init::init_detours().unwrap() }
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