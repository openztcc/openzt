#[cfg(target_os = "windows")]
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH};

#[cfg(not(target_os = "windows"))]
mod linux {
    const DLL_PROCESS_DETACH: u32 = 0;
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_THREAD_ATTACH: u32 = 2;
    const DLL_THREAD_DETACH: u32 = 3;
}

mod debug_dll;
mod load_ini;

#[no_mangle]
pub extern fn lib_test() {
    debug_dll::debug_logger("Test");
    debug_dll::log_debug_ini_memory_values();
    patch_load_debug_ini_call();
}

#[no_mangle]
pub fn dll_first_load() {
    debug_dll::debug_logger("Loaded")
}


#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            dll_first_load();
            lib_test();
            debug_dll::debug_logger(&format!("DllMain: DLL_PROCESS_ATTACH: {}, {} {}", module, reason, _reserved));
            // debug_dll::exit_program(0);
        }
        DLL_PROCESS_DETACH => {
            debug_dll::debug_logger(&format!("DllMain: DLL_PROCESS_DETACH: {}, {} {}", module, reason, _reserved));
        }
        DLL_THREAD_ATTACH => {
            debug_dll::debug_logger(&format!("DllMain: DLL_THREAD_ATTACH: {}, {} {}", module, reason, _reserved));
        }
        DLL_THREAD_DETACH => {
            debug_dll::debug_logger(&format!("DllMain: DLL_THREAD_DETACH: {}, {} {}", module, reason, _reserved));
        }
        _ => {
            debug_dll::debug_logger(&format!("DllMain: Unknown: {}, {} {}", module, reason, _reserved));
        }
    }
    1
}

#[no_mangle]
extern "system" fn DllMainCRTStartup() -> i32 {
    debug_dll::debug_logger("DllMainCRTStartup");
    1
}


#[no_mangle]
extern "system" fn dll_ini_debug_log() {
    debug_dll::log_debug_ini_memory_values();
}

#[no_mangle]
pub fn load_debug_settings_from_ini() {
    debug_dll::debug_logger("load_debug_settings_from_ini");
    debug_dll::log_exe_location_memory_value();
    debug_dll::log_debug_ini_memory_values();
    let mut base_path = debug_dll::get_base_path();
    base_path.push("zoo.ini");
    let debug_settings = load_ini::load_debug_settings(base_path.as_path());
    debug_dll::debug_logger("Saving debug ini settings");
    debug_dll::save_debug_settings(debug_settings);
    debug_dll::log_debug_ini_memory_values();
    // debug_dll::exit_program(1);
}

#[no_mangle]
pub fn patch_load_debug_ini_call() {
    debug_dll::debug_logger(&format!("load_debug_settings_from_ini {:p}", load_debug_settings_from_ini as *const ()));
    debug_dll::debug_logger(&format!("load_debug_settings_from_ini (u32) {}", load_debug_settings_from_ini as u32));
    debug_dll::decode_code(debug_dll::DEBUG_INI_LOAD_CALL_ADDRESS, 0x10);
    debug_dll::patch_call(debug_dll::DEBUG_INI_LOAD_CALL_ADDRESS, load_debug_settings_from_ini as u32);
}


// #[no_mangle]
// extern "system" fn load_init_from_ini() -> i32 {
    
// }
