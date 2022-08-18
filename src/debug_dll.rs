use std::fs;
use std::path::{Path,PathBuf};
use std::io::Write;
use std::ptr;

use chrono::Local;

use directories::ProjectDirs;

use crate::load_ini::DebugSettings;

#[cfg(target_os = "windows")]
use winapi::um::memoryapi::VirtualProtect;
#[cfg(target_os = "windows")]
use winapi::um::winnt::{PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_READ};

const SEND_DEBUGGER_ADDRESS: u32 = 0x00643e44;
const SEND_LOG_FILE_ADDRESS: u32 = 0x00643e48;
const SEND_MESSAGE_BOX_ADDRESS: u32 = 0x00643e4a;
const DELTA_LOG_0_ADDRESS: u32 = 0x00638054;
const DELTA_LOG_1_ADDRESS: u32 = 0x0064bd7c;
const LOG_CUTOFF_ADDRESS: u32 = 0x0063804c;

pub const DEBUG_INI_LOAD_CALL_ADDRESS: u32 = 0x0057a218;
pub const DEBUG_INI_LOAD_FUNCTION_ADDRESS: u32 = 0x00579f4c;
const DEBUG_INI_LOAD_FUNCTION_RETURN_ADDRESS: u32 = 0x0057a217;

pub const EXE_LOCATION_ADDRESS: u32 = 0x0064BEDC;
pub const EXE_LOCATION_ADDRESS_2: u32 = 0x0064BED8;
pub const EXE_LOCATION_ADDRESS_3: u32 = 0x0064A800;

pub fn debug_logger(message: &str) {
    let log_file_path = Path::new("openzt.log");
    let timestamp = Local::now().format("%F %T%.3f");
    if let Some(proj_dirs) = ProjectDirs::from("com", "fh",  "openzt") {
        let config_dir = proj_dirs.config_dir();
        let _ = fs::create_dir_all(config_dir);

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(config_dir.join(log_file_path))
            .unwrap();
        file.write_all(format!("{timestamp} - {message}\n").as_bytes()).unwrap();    
    }
}

pub fn get_from_memory<T>(address: u32) -> T {
    return unsafe { ptr::read(address as *const T)};
}

pub fn save_to_memory<T>(address: u32, value: T) {
    unsafe { ptr::write(address as *mut T, value)};
}

pub fn log_debug_ini_memory_values() {
    let send_debugger: bool = get_from_memory::<bool>(SEND_DEBUGGER_ADDRESS);
    debug_logger(&format!("send_debugger: {}", send_debugger));
    let send_log_file: bool = get_from_memory::<bool>(SEND_LOG_FILE_ADDRESS);
    debug_logger(&format!("send_log_file: {}", send_log_file));
    let send_message: bool = get_from_memory::<bool>(SEND_MESSAGE_BOX_ADDRESS);
    debug_logger(&format!("send_message: {}", send_message));
    let delta_log_0: bool = get_from_memory::<bool>(DELTA_LOG_0_ADDRESS);
    debug_logger(&format!("delta_log_0: {}", delta_log_0));
    let delta_log_1: bool = get_from_memory::<bool>(DELTA_LOG_1_ADDRESS);
    debug_logger(&format!("delta_log_1: {}", delta_log_1));
    let log_cutoff: u32 = get_from_memory::<u32>(LOG_CUTOFF_ADDRESS);
    debug_logger(&format!("log_cutoff: {}", log_cutoff));
}

pub fn log_exe_location_memory_value() {
    let exe_location: String = decode_string(EXE_LOCATION_ADDRESS);
    debug_logger(&format!("exe location from memory: {}", exe_location));
    debug_logger(&format!("exe location from rust: {}", std::env::current_exe().unwrap().to_str().unwrap()));
}

pub fn decode_string(address: u32) -> String {
    debug_logger(&format!("decoding string at address: {}", address));
    let mut string = String::new();
    let mut char_address = address;
    while { let byte = get_from_memory::<u8>(char_address); byte != 0 } {
        string.push(get_from_memory::<u8>(char_address) as char);
        char_address += 1;
    }
    debug_logger(&format!("decoded: {}", string));
    return string;
}

pub fn decode_code(address: u32, size: u32) {
    let mut code = String::new();
    for i in 0..size {
        let byte = get_from_memory::<u8>(address + i);
        code.push_str(&format!("{:02x} ", byte));
    }
    debug_logger(&code);
}

pub fn exit_program(code: i32) {
    unsafe {
        std::process::exit(code);
    }
}

pub fn patch_call(address: u32, new_address: u32) {
    let opcode: u8 = get_from_memory::<u8>(address);
    let old_offset: u32 = get_from_memory::<u32>(address + 1);
    debug_logger(&format!("opcode: {:02x}", opcode));
    debug_logger(&format!("current address: {:02x} current offset: {:02x} {}", address, old_offset, old_offset as i32));
    debug_logger(&format!("new address: {:02x}", new_address));
    let address_offset: i32 = (new_address - address - 5) as i32;
    debug_logger(&format!("new address offset: {:02x} ", address_offset));
    assert!(opcode == 0xe8);
    unsafe {
        #[cfg(target_os = "windows")]
        {
            let mut old_protect: u32 = 0;
            VirtualProtect(address as *mut _, 5, PAGE_EXECUTE_READWRITE, &mut old_protect);
            ptr::write((address + 1) as *mut _, address_offset);
            VirtualProtect(address as *mut _, 5, old_protect, &mut old_protect);
        }
    }
}

pub fn save_debug_settings(settings: DebugSettings) {
    save_to_memory::<bool>(SEND_DEBUGGER_ADDRESS, settings.send_debugger != 0);
    save_to_memory::<bool>(SEND_LOG_FILE_ADDRESS, settings.send_log_file != 0);
    save_to_memory::<bool>(SEND_MESSAGE_BOX_ADDRESS, settings.send_message_box != 0);
    save_to_memory::<bool>(DELTA_LOG_0_ADDRESS, settings.delta_log_0 != 0);
    save_to_memory::<bool>(DELTA_LOG_1_ADDRESS, settings.delta_log_1 != 0);
    save_to_memory::<u32>(LOG_CUTOFF_ADDRESS, settings.log_cutoff as u32);
}
pub fn get_base_path() -> PathBuf {
    let mut exe_location = std::env::current_exe().unwrap();
    exe_location.pop();
    return exe_location;
}
