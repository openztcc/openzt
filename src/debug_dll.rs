use std::fs;
use std::path::Path;
use std::io::Write;
use std::ptr;

use chrono::Local;

use directories::ProjectDirs;

// pub unsafe fn print_debug_globals() {
    
// }

const SEND_DEBUGGER_ADDRESS: u32 = 0x00643e44;
const SEND_LOG_FILE_ADDRESS: u32 = 0x00643e48;
const SEND_MESSAGE_ADDRESS: u32 = 0x00643e4a;
const DELTA_LOG_0_ADDRESS: u32 = 0x00638054;
const DELTA_LOG_1_ADDRESS: u32 = 0x0064bd7c;
const LOG_CUTOFF_ADDRESS: u32 = 0x0063804c;

pub const DEBUG_INI_LOAD_CALL_ADDRESS: u32 = 0x0057a218;
pub const DEBUG_INI_LOAD_FUNCTION_ADDRESS: u32 = 0x00579f4c;
const DEBUG_INI_LOAD_FUNCTION_RETURN_ADDRESS: u32 = 0x0057a217;

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

pub fn log_debug_ini_memory_values() {
    let send_debugger: bool = get_from_memory::<bool>(SEND_DEBUGGER_ADDRESS);
    debug_logger(&format!("send_debugger: {}", send_debugger));
    let send_log_file: bool = get_from_memory::<bool>(SEND_LOG_FILE_ADDRESS);
    debug_logger(&format!("send_log_file: {}", send_log_file));
    let send_message: bool = get_from_memory::<bool>(SEND_MESSAGE_ADDRESS);
    debug_logger(&format!("send_message: {}", send_message));
    let delta_log_0: bool = get_from_memory::<bool>(DELTA_LOG_0_ADDRESS);
    debug_logger(&format!("delta_log_0: {}", delta_log_0));
    let delta_log_1: bool = get_from_memory::<bool>(DELTA_LOG_1_ADDRESS);
    debug_logger(&format!("delta_log_1: {}", delta_log_1));
    let log_cutoff: u32 = get_from_memory::<u32>(LOG_CUTOFF_ADDRESS);
    debug_logger(&format!("log_cutoff: {}", log_cutoff));
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
    let old_address: u32 = get_from_memory::<u32>(address + 1);
    debug_logger(&format!("opcode: {:02x}", opcode));
    debug_logger(&format!("current address: {:02x}", old_address));
    assert!(opcode == 0xe8);
    // unsafe {
    //     ptr::write((address + 1) as *mut u32, new_address);
    // }
}