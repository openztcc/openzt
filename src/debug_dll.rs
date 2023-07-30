use std::path::PathBuf;
use std::ptr;

use tracing::{info, debug};

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

const SHOW_BUILDING_AI_INFO: u32 = 0x00638fc8;

const ZTAIMGR_ADDRESS_PTR: u32 = 0x00638098;

const SHOW_AI_INFO_OFFSET: u32 = 0xf4;
const SHOW_NAME_OFFSET: u32 = 0xf8;
const SHOW_POSITION_OFFSET: u32 = 0xfc;
const SHOW_STATUS_VARS_OFFSET: u32 = 0x100;
const SHOW_FUNCTION_CALL_OFFSET: u32 = 0x108;
const SHOW_EVENTS_OFFSET: u32 = 0x10c;
const SHOW_SELECTED_OFFSET: u32 = 0x104;
const SHOW_FRAME_OFFSET: u32 = 0x114;
const SHOW_GOAL_OFFSET: u32 = 0x118;
const AI_INFO_NTH_OFFSET: u32 = 0x110;

pub const DEBUG_INI_LOAD_CALL_ADDRESS: u32 = 0x0057a218;
pub const DEBUG_INI_LOAD_FUNCTION_ADDRESS: u32 = 0x00579f4c; 


const DEBUG_INI_LOAD_FUNCTION_RETURN_ADDRESS: u32 = 0x0057a217;

pub const EXE_LOCATION_ADDRESS: u32 = 0x0064BEDC;
pub const EXE_LOCATION_ADDRESS_2: u32 = 0x0064BED8;
pub const EXE_LOCATION_ADDRESS_3: u32 = 0x0064A800;
pub const LOAD_INT_FROM_INI_ADDRESS_ARRAY: [u32; 146] = [0x0041b1cd, 0x00442dc1, 0x00453449, 0x0046154c, 0x00462825, 0x00463935, 0x004639e2, 0x0047eaa1, 0x0048b03d, 0x0048b1b9, 0x004bc271, 0x004bd23f, 0x004c2778, 0x004c2848, 0x004c2987, 0x004ca3ea, 0x004cabe6, 0x004cb55d, 0x004cb629, 0x004d6948, 0x004d8305, 0x004d83b2, 0x004ebc8e, 0x0050efc1, 0x0051319a, 0x0051324a, 0x0051841a, 0x005184fb, 0x00518621, 0x0051872a, 0x00518842, 0x00518942, 0x00518a65, 0x00518b4f, 0x00518c3f, 0x00518e08, 0x00518eab, 0x00518f4e, 0x00518ff1, 0x0051909a, 0x0051915b, 0x0051e375, 0x0051e489, 0x0051e52c, 0x0051e5cf, 0x0051e670, 0x0051e71e, 0x0051e7c7, 0x0051e870, 0x0051e914, 0x0051fb81, 0x0051fc28, 0x0051fccf, 0x0051fd76, 0x0051fe1d, 0x0051febf, 0x0051ff66, 0x00520012, 0x005200b4, 0x00520160, 0x0052020c, 0x00520b26, 0x00521ed5, 0x00525a42, 0x00525aeb, 0x00525bbf, 0x00525c68, 0x00525d31, 0x00525df1, 0x00526125, 0x005261c5, 0x00526269, 0x0052630d, 0x005263ae, 0x005264f5, 0x00526544, 0x005268a0, 0x00526949, 0x005269f2, 0x00526a9b, 0x00526b7e, 0x00526c31, 0x00526cd8, 0x00526d7b, 0x00526e2a, 0x00526ed9, 0x00526f88, 0x00527037, 0x005270e6, 0x00527195, 0x00527248, 0x005272f7, 0x005273a6, 0x00527455, 0x00527504, 0x005275a7, 0x00527627, 0x00527678, 0x00527d83, 0x00527eb0, 0x005281c1, 0x0052826b, 0x00528315, 0x005283bf, 0x00528590, 0x0052aea3, 0x0052af49, 0x0052afef, 0x0052b095, 0x0052b13b, 0x0052b1e1, 0x0052b284, 0x0052b327, 0x0052b3ca, 0x0052b46d, 0x0052b510, 0x0052b58a, 0x0052b5de, 0x0052bd95, 0x0052be3a, 0x0052bee1, 0x0052c0b4, 0x00533edb, 0x00533f90, 0x00534208, 0x00534406, 0x00536340, 0x005363ed, 0x00536666, 0x005367d2, 0x00579fda, 0x0057a044, 0x0057a0b5, 0x0057a123, 0x0057a18e, 0x0057a1f1, 0x00598daf, 0x005b1558, 0x005b15fd, 0x005b257e, 0x005c181c, 0x005c1ea7, 0x005d6efa, 0x005d7df9, 0x005e5eab, 0x00606980];
pub const LOAD_INT_FROM_INI_ADDRESS_ARRAY_FAILED: [u32; 13] = [0x004bc32b, 0x004bc3d0, 0x004bc475, 0x004bc51a, 0x004bc5c4, 0x004bc667, 0x004bccce, 0x004bc854, 0x004bc7b0, 0x004bc707, 0x004bc8f8, 0x00533b1b, 0x005956e1];

pub const LOAD_INT_FROM_INI_ADDRESS_ARRAY_SUBSET: [u32; 2] = [0x004bc271, 0x004bc32b];
pub const LOAD_INT_FROM_INI_ADDRESS_ARRAY_SUBSET_NOP: [u32; 8] = [0x004bc224, 0x004bc260, 0x004bc27f, 0x004bc288, 0x004bc2de, 0x004bc31a, 0x004bc33e, 0x004bc347];

pub const LOAD_VALUE_FROM_INI_ADDRESS_ARRAY: [u32; 1] = [0x005221a8];

pub fn debug_logger(message: &str) {
    info!(message);
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
    let exe_location: String = get_string_from_memory(EXE_LOCATION_ADDRESS);
    debug_logger(&format!("exe location from memory: {}", exe_location));
    debug_logger(&format!("exe location from rust: {}", std::env::current_exe().unwrap().to_str().unwrap()));
}

pub fn get_string_from_memory(address: u32) -> String {
    debug!("decoding string at address: {:p}", address as *const ());
    let mut string = String::new();
    let mut char_address = address;
    while { let byte = get_from_memory::<u8>(char_address); byte != 0 } {
        string.push(get_from_memory::<u8>(char_address) as char);
        char_address += 1;
    }
    debug!("decoded: {}", string);
    return string;
}

pub fn save_string_to_memory(address: u32, string: &str) {
    debug_logger(&format!("encoding string at address: {:p}", address as *const ()));
    let mut char_address = address;
    for c in string.chars() {
        save_to_memory::<u8>(char_address, c as u8);
        char_address += 1;
    }
    save_to_memory::<u8>(char_address, 0);
    debug_logger(&format!("encoded: {}", string));
}

pub fn get_code_from_memory(address: u32, size: u32) {
    let mut code = String::new();
    for i in 0..size {
        let byte = get_from_memory::<u8>(address + i);
        code.push_str(&format!("{:02x} ", byte));
    }
    debug_logger(&code);
}

pub fn exit_program(code: i32) {
    std::process::exit(code);
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

pub fn command_set_setting(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() != 2 {
        return Err("Invalid number of arguments");
    }
    let setting = args[0].to_string();
    let value = args[1].to_string();
    Ok(set_setting(setting, value))
}

fn set_setting(setting: String, value: String) -> String {
    match setting.as_str() {
        "sendDebugger" | "sendLogFile" | "sendMessageBox" | "deltaLog0" | "deltaLog1" | "ShowBuildingAIInfo" => {
            handle_bool_setting(setting.as_str(), value)
        },
        "logCutoff" => {
            handle_u32_setting(setting.as_str(), value)
        },
        "ShowGoal" | "ShowFrame" | "ShowSelected" | "ShowEvents" | "ShowFunctionCall" | "ShowStatusVars" | "ShowPosition" | "ShowName" | "ShowAIInfo" => {
            info!("handle_get_bool_zt_ai_mgr_setting");
            handle_set_bool_zt_ai_mgr_setting(setting.as_str(), value)
        },
        _ => {
            format!("unknown setting: {}", setting)
        }
    }
}

fn handle_bool_setting(setting: &str, value: String) -> String {
    match parse_bool(&value) {
        Ok(setting_value) => {
            let address = match setting {
                "sendDebugger" => SEND_DEBUGGER_ADDRESS,
                "sendLogFile" => SEND_LOG_FILE_ADDRESS,
                "sendMessageBox" => SEND_MESSAGE_BOX_ADDRESS,
                "deltaLog0" => DELTA_LOG_0_ADDRESS,
                "deltaLog1" => DELTA_LOG_1_ADDRESS,
                _ => unreachable!(), // This should never happen due to outer match
            };
            save_to_memory::<bool>(address, setting_value);
            format!("{} set to {}", setting, setting_value)
        },
        Err(_) => {
            format!("invalid value: {}", value)
        }
    }
}

fn handle_u32_setting(setting: &str, value: String) -> String {
    match value.parse::<u32>() {
        Ok(setting_value) => {
            let address: u32 = match setting {
                "logCutoff" => LOG_CUTOFF_ADDRESS,
                _ => unreachable!(), // This should never happen due to outer match
            };
            save_to_memory::<u32>(address, setting_value);
            format!("{} set to {}", setting, setting_value)
        },
        Err(_) => {
            format!("invalid value: {}", value)
        }
    }
}

pub fn command_get_setting(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() != 1 {
        return Err("Invalid number of arguments");
    }
    let setting = args[0].to_string();
    Ok(get_setting(setting))
}

fn get_setting(setting: String) -> String {
    match setting.as_str() {
        "sendDebugger" | "sendLogFile" | "sendMessageBox" | "deltaLog0" | "deltaLog1" | "ShowBuildingAIInfo" => {
            handle_get_bool_setting(setting.as_str())
        },
        "logCutoff" => {
            handle_get_u32_setting(setting.as_str())
        },
        "ShowGoal" | "ShowFrame" | "ShowSelected" | "ShowEvents" | "ShowFunctionCall" | "ShowStatusVars" | "ShowPosition" | "ShowName" | "ShowAIInfo" => {
            handle_get_bool_zt_ai_mgr_setting(setting.as_str())
        },
        _ => {
            format!("unknown setting: {}", setting)
        }
    }
}

fn setting_to_address(setting: &str) -> u32 {
    match setting {
        "sendDebugger" => SEND_DEBUGGER_ADDRESS,
        "sendLogFile" => SEND_LOG_FILE_ADDRESS,
        "sendMessageBox" => SEND_MESSAGE_BOX_ADDRESS,
        "deltaLog0" => DELTA_LOG_0_ADDRESS,
        "deltaLog1" => DELTA_LOG_1_ADDRESS,
        "logCutoff" => LOG_CUTOFF_ADDRESS,
        "ShowBuildingAIInfo" => SHOW_BUILDING_AI_INFO,
        "ShowGoal" => SHOW_GOAL_OFFSET,
        "ShowFrame" => SHOW_FRAME_OFFSET,
        "ShowSelected" => SHOW_SELECTED_OFFSET,
        "ShowEvents" => SHOW_EVENTS_OFFSET,
        "ShowFunctionCall" => SHOW_FUNCTION_CALL_OFFSET,
        "ShowStatusVars" => SHOW_STATUS_VARS_OFFSET,
        "ShowPosition" => SHOW_POSITION_OFFSET,
        "ShowName" => SHOW_NAME_OFFSET,
        "ShowAIInfo" => SHOW_AI_INFO_OFFSET,
        _ => unreachable!(), // This should never happen due to outer match
    }
}

fn handle_get_bool_setting(setting: &str) -> String {
    let address = setting_to_address(setting);
    let value: bool = get_from_memory::<bool>(address);
    format!("{}: {}", setting, value)
}

fn handle_get_u32_setting(setting: &str) -> String {
    let address = setting_to_address(setting);
    let value: u32 = get_from_memory::<u32>(address);
    format!("{}: {}", setting, value)
}

pub fn command_show_settings(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() != 0 {
        return Err("Invalid number of arguments");
    }
    Ok(show_settings())
}

pub fn show_settings() -> String {
    let send_debugger: bool = get_from_memory::<bool>(SEND_DEBUGGER_ADDRESS);
    let send_log_file: bool = get_from_memory::<bool>(SEND_LOG_FILE_ADDRESS);
    let send_message: bool = get_from_memory::<bool>(SEND_MESSAGE_BOX_ADDRESS);
    let delta_log_0: bool = get_from_memory::<bool>(DELTA_LOG_0_ADDRESS);
    let delta_log_1: bool = get_from_memory::<bool>(DELTA_LOG_1_ADDRESS);
    let log_cutoff: u32 = get_from_memory::<u32>(LOG_CUTOFF_ADDRESS);
    let show_building_ai_info: bool = get_from_memory::<bool>(SHOW_BUILDING_AI_INFO);
    return format!("send_debugger: {}\nsend_log_file: {}\nsend_message: {}\ndelta_log_0: {}\ndelta_log_1: {}\nlog_cutoff: {}", send_debugger, send_log_file, send_message, delta_log_0, delta_log_1, log_cutoff);
}

pub fn parse_bool(string: &String) -> Result<bool, String> {
    match string.trim() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err("Invalid input".to_string()),
    }
}

fn handle_get_bool_zt_ai_mgr_setting(setting: &str) -> String {
    let address = get_from_memory::<u32>(ZTAIMGR_ADDRESS_PTR);
    let offset = setting_to_address(setting);
    let value: bool = get_from_memory::<bool>(address + offset);
    return format!("{}: {}", setting, value);
}

fn handle_set_bool_zt_ai_mgr_setting(setting: &str, value: String) -> String {
    let address = get_from_memory::<u32>(ZTAIMGR_ADDRESS_PTR);
    let offset = setting_to_address(setting);
    match parse_bool(&value) {
        Ok(setting_value) => {
            save_to_memory::<bool>(address + offset, setting_value);
            return format!("{} set to {}", setting, setting_value);
        },
        Err(_) => {
            return format!("invalid value: {}", value);
        }
    }
    return format!("{}: {}", setting, value);
}

pub fn get_base_path() -> PathBuf {
    let mut exe_location = std::env::current_exe().unwrap();
    exe_location.pop();
    return exe_location;
}

pub fn patch_calls(addresses: Vec<u32>, new_address: u32) {
    for address in addresses {
        patch_call(address, new_address);
    }
}

pub fn patch_nop(address: u32) {
    let opcode: u8 = get_from_memory::<u8>(address);
    info!("Nop opcode: {:02x}", opcode);
    unsafe {
        #[cfg(target_os = "windows")]
        {
            let mut old_protect: u32 = 0;
            VirtualProtect(address as *mut _, 1, PAGE_EXECUTE_READWRITE, &mut old_protect);
            ptr::write(address as *mut _, 0x90);
            VirtualProtect(address as *mut _, 1, old_protect, &mut old_protect);
        }
    }
}

pub fn patch_nops(address: u32, size: u32) {
    for i in 0..size {
        patch_nop(address + i);
    }
}

pub fn patch_nops_series(patches: Vec<u32>) {
    for i in (0..patches.len()).step_by(2) {
        patch_nops(patches[i], patches[i + 1]);
    }
}

pub fn read_string_array_from_memory(address: u32, size: u32) -> Vec<String> {
    let mut strings: Vec<String> = Vec::new();
    let mut string_address = address;
    for _ in 0..size {
        let string = get_string_from_memory(string_address);
        strings.push(string);
        string_address += 0x100;
    }
    return strings;
}

pub fn get_zt_string_array_from_memory(address: u32, end_address: u32) -> Vec<String> {
    let mut strings: Vec<String> = Vec::new();
    let array_length = (end_address - address) / 0xc;
    let mut string_address = address;
    for _ in 0..array_length {
        let string = get_string_from_memory(get_from_memory::<u32>(string_address));
        strings.push(string);
        string_address += 0xc;
    }
    return strings;
}