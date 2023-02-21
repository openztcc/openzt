use std::mem;

use configparser::ini::Ini;

use std::net::TcpStream;

use retour::static_detour;

use std::sync::Mutex;

use tracing::{info, error};

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
    debug_dll::debug_logger("Remote debug Test");
    debug_dll::log_debug_ini_memory_values();
    patch_load_debug_ini_call();
    // patch_load_int_from_ini_call();
    // patch_load_value_from_ini_call();
}

#[no_mangle]
pub fn dll_first_load() {

    let stream = TcpStream::connect("127.0.0.1:1492").unwrap();

    tracing_subscriber::fmt()
        .with_writer(Mutex::new(stream))
        // .with_writer(stream)
        //.with_max_level
        .init();

    info!("Remote Test");

    debug_dll::debug_logger("Loaded");
}

// static mut LOAD_DEBUG_SETTINGS_FROM_INI: Option<GenericDetour<extern "cdecl" fn()>> = None;

// //DEBUG_INI_LOAD_FUNCTION_ADDRESS


// // load_debug_settings_from_ini

static_detour! {
    static LoadDebugSettingsFromIniHook: unsafe extern "cdecl" fn();
}

type FnLoadDebugSettingsFromIni = unsafe extern "cdecl" fn();


#[no_mangle]
pub fn detour_logging_test() {
    debug_dll::debug_logger("Attempting detour");

    unsafe {
        let target: FnLoadDebugSettingsFromIni = mem::transmute(debug_dll::DEBUG_INI_LOAD_FUNCTION_ADDRESS);

        LoadDebugSettingsFromIniHook
            .initialize(target, load_debug_settings_from_ini_log).expect("Failed to initialise detour")
            .enable().expect("Failed to enable detour");
    }
}

fn load_debug_settings_from_ini_log() {
    info!("Calling load_debug_settings_from_ini_log");
    unsafe { LoadDebugSettingsFromIniHook.call() }
}

#[no_mangle]
pub fn detour_detour_test() {
    debug_dll::debug_logger("Attempting detour");

    unsafe {
        let target: FnLoadDebugSettingsFromIni = mem::transmute(debug_dll::DEBUG_INI_LOAD_FUNCTION_ADDRESS);

        LoadDebugSettingsFromIniHook
            .initialize(target, load_debug_settings_from_ini_detour).expect("Failed to initialise detour")
            .enable().expect("Failed to enable detour");
    }
}

fn load_debug_settings_from_ini_detour() {
    info!("Calling load_debug_settings_from_ini_detour");
    load_debug_settings_from_ini();
}


#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            dll_first_load();
            detour_detour_test();
            debug_dll::debug_logger(&format!("DllMain: DLL_PROCESS_ATTACH: {}, {} {}", module, reason, _reserved));
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
extern "C" fn DllMainCRTStartup() -> i32 {
    debug_dll::debug_logger("DllMainCRTStartup");
    1
}


#[no_mangle]
extern "C" fn dll_ini_debug_log() {
    debug_dll::log_debug_ini_memory_values();
}

fn load_debug_settings_from_ini() {
    debug_dll::debug_logger("load_debug_settings_from_ini");
    debug_dll::log_exe_location_memory_value();
    debug_dll::log_debug_ini_memory_values();
    let mut base_path = debug_dll::get_base_path();
    base_path.push("zoo.ini");
    let debug_settings = load_ini::load_debug_settings(base_path.as_path());
    debug_dll::debug_logger("Saving debug ini settings");
    debug_dll::save_debug_settings(debug_settings);
    debug_dll::log_debug_ini_memory_values();
}

#[no_mangle]
pub fn patch_load_debug_ini_call() {
    debug_dll::debug_logger(&format!("load_debug_settings_from_ini {:p}", load_debug_settings_from_ini as *const ()));
    debug_dll::debug_logger(&format!("load_debug_settings_from_ini (u32) {}", load_debug_settings_from_ini as u32));
    debug_dll::get_code_from_memory(debug_dll::DEBUG_INI_LOAD_CALL_ADDRESS, 0x10);
    debug_dll::patch_call(debug_dll::DEBUG_INI_LOAD_CALL_ADDRESS, load_debug_settings_from_ini as u32);
}

#[no_mangle]
extern "C" fn patch_load_int_from_ini_call() {
    debug_dll::debug_logger(&format!("load_int_from_ini {:p}", load_int_from_ini as *const ()));
    debug_dll::patch_calls(debug_dll::LOAD_INT_FROM_INI_ADDRESS_ARRAY_SUBSET.to_vec(), load_int_from_ini as u32);
    debug_dll::patch_nops_series(debug_dll::LOAD_INT_FROM_INI_ADDRESS_ARRAY_SUBSET_NOP.to_vec());
}

#[no_mangle]
extern "C" fn patch_load_value_from_ini_call() {
    debug_dll::debug_logger(&format!("load_value_from_ini {:p}", load_value_from_ini as *const ()));
    debug_dll::patch_calls(debug_dll::LOAD_VALUE_FROM_INI_ADDRESS_ARRAY.to_vec(), load_value_from_ini as u32);
}


#[no_mangle]
extern "cdecl" fn load_int_from_ini(section_address: &u32, header_address: &u32, default: i32) -> u32 {
    debug_dll::debug_logger(&format!("load_int_from_ini {:p} {:p} default: {}", *section_address as *const (), *header_address as *const (), default));
    let section = debug_dll::get_string_from_memory(*section_address);
    let header = debug_dll::get_string_from_memory(*header_address);
    let mut zoo_ini = Ini::new();
    zoo_ini.load(get_ini_path()).unwrap();
    let result = load_ini::load_int_with_default(&zoo_ini, &section, &header, default) as u32;
    debug_dll::debug_logger(&format!("load_int_from_ini {} {} result: {}", section, header, result));
    return result;
}

#[no_mangle]
extern "cdecl" fn load_value_from_ini<'a>(result_address: &'a u32, section_address: &u32, header_address: &u32, default_address: &u32) -> &'a u32{
    debug_dll::debug_logger(&format!("load_value_from_ini {:p} {:p} default: {:p}", *section_address as *const (), *header_address as *const (), *default_address as *const ()));
    let section = debug_dll::get_string_from_memory(*section_address);
    let header = debug_dll::get_string_from_memory(*header_address);
    let default = debug_dll::get_string_from_memory(*default_address);
    let mut zoo_ini = Ini::new();
    zoo_ini.load(get_ini_path()).unwrap();
    let result = load_ini::load_string_with_default(&zoo_ini, &section, &header, &default);

    debug_dll::debug_logger(&format!("load_value_from_ini {} {} result: {}", section, header, result));
    debug_dll::debug_logger(&format!("encoding string at address: {:p}", *result_address as *const ()));
    debug_dll::save_string_to_memory(*(&result_address as &u32), &result);
    // ptr::write(result_address as *mut _, result);
    return result_address;
}

fn get_ini_path() -> String {
    let mut base_path = debug_dll::get_base_path();
    base_path.push("zoo.ini");
    base_path.to_str().unwrap().to_string()
}
