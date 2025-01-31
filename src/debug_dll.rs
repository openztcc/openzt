use tracing::info;

use crate::{console::CommandError, load_ini::DebugSettings, util::{get_from_memory, save_to_memory}};


pub fn log_debug_ini_memory_values() {
    let send_debugger: bool = get_from_memory::<bool>(SEND_DEBUGGER_ADDRESS);
    info!("send_debugger: {}", send_debugger);
    let send_log_file: bool = get_from_memory::<bool>(SEND_LOG_FILE_ADDRESS);
    info!("send_log_file: {}", send_log_file);
    let send_message: bool = get_from_memory::<bool>(SEND_MESSAGE_BOX_ADDRESS);
    info!("send_message: {}", send_message);
    let delta_log_0: bool = get_from_memory::<bool>(DELTA_LOG_0_ADDRESS);
    info!("delta_log_0: {}", delta_log_0);
    let delta_log_1: bool = get_from_memory::<bool>(DELTA_LOG_1_ADDRESS);
    info!("delta_log_1: {}", delta_log_1);
    let log_cutoff: u32 = get_from_memory::<u32>(LOG_CUTOFF_ADDRESS);
    info!("log_cutoff: {}", log_cutoff);
}

pub fn exit_program(code: i32) {
    std::process::exit(code);
}

pub fn save_debug_settings(settings: DebugSettings) {
    save_to_memory::<bool>(SEND_DEBUGGER_ADDRESS, settings.send_debugger != 0);
    save_to_memory::<bool>(SEND_LOG_FILE_ADDRESS, settings.send_log_file != 0);
    save_to_memory::<bool>(SEND_MESSAGE_BOX_ADDRESS, settings.send_message_box != 0);
    save_to_memory::<bool>(DELTA_LOG_0_ADDRESS, settings.delta_log_0 != 0);
    save_to_memory::<bool>(DELTA_LOG_1_ADDRESS, settings.delta_log_1 != 0);
    save_to_memory::<u32>(LOG_CUTOFF_ADDRESS, settings.log_cutoff as u32);
}

pub fn command_set_setting(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() != 2 {
        return Err(Into::into("Invalid number of arguments"));
    }
    let setting = args[0].to_string();
    let value = args[1].to_string();
    Ok(set_setting(setting, value))
}

fn set_setting(setting: String, value: String) -> String {
    match setting.as_str() {
        "sendDebugger" | "sendLogFile" | "sendMessageBox" | "deltaLog0" | "deltaLog1" | "ShowBuildingAIInfo" => handle_bool_setting(setting.as_str(), value),
        "logCutoff" => handle_u32_setting(setting.as_str(), value),
        "ShowGoal" | "ShowFrame" | "ShowSelected" | "ShowEvents" | "ShowFunctionCall" | "ShowStatusVars" | "ShowPosition" | "ShowName" | "ShowAIInfo" => {
            info!("handle_get_bool_zt_ai_mgr_setting");
            handle_set_bool_zt_ai_mgr_setting(setting.as_str(), value)
        }
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
        }
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
        }
        Err(_) => {
            format!("invalid value: {}", value)
        }
    }
}

pub fn command_get_setting(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() != 1 {
        return Err(Into::into("Invalid number of arguments"));
    }
    let setting = args[0].to_string();
    Ok(get_setting(setting))
}

fn get_setting(setting: String) -> String {
    match setting.as_str() {
        "sendDebugger" | "sendLogFile" | "sendMessageBox" | "deltaLog0" | "deltaLog1" | "ShowBuildingAIInfo" => handle_get_bool_setting(setting.as_str()),
        "logCutoff" => handle_get_u32_setting(setting.as_str()),
        "ShowGoal" | "ShowFrame" | "ShowSelected" | "ShowEvents" | "ShowFunctionCall" | "ShowStatusVars" | "ShowPosition" | "ShowName" | "ShowAIInfo" => {
            handle_get_bool_zt_ai_mgr_setting(setting.as_str())
        }
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

pub fn command_show_settings(args: Vec<&str>) -> Result<String, CommandError> {
    if !args.is_empty() {
        return Err(Into::into("Invalid number of arguments"));
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
    let show_goal: String = handle_get_bool_zt_ai_mgr_setting("ShowGoal");
    let show_frame: String = handle_get_bool_zt_ai_mgr_setting("ShowFrame");
    let show_selected: String = handle_get_bool_zt_ai_mgr_setting("ShowSelected");
    let show_events: String = handle_get_bool_zt_ai_mgr_setting("ShowEvents");
    let show_function_call: String = handle_get_bool_zt_ai_mgr_setting("ShowFunctionCall");
    let show_status_vars: String = handle_get_bool_zt_ai_mgr_setting("ShowStatusVars");
    let show_position: String = handle_get_bool_zt_ai_mgr_setting("ShowPosition");
    let show_name: String = handle_get_bool_zt_ai_mgr_setting("ShowName");
    let show_ai_info: String = handle_get_bool_zt_ai_mgr_setting("ShowAIInfo");
    format!(
        "sendDebugger: {}\nsendLogFile: {}\nsendMessage: {}\ndeltaLog0: {}\ndeltaLog1: {}\nlogCutoff: {}\nShowBuildingAIInfo: {}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        send_debugger,
        send_log_file,
        send_message,
        delta_log_0,
        delta_log_1,
        log_cutoff,
        show_building_ai_info,
        show_goal,
        show_frame,
        show_selected,
        show_events,
        show_function_call,
        show_status_vars,
        show_position,
        show_name,
        show_ai_info
    )
}

fn handle_get_bool_zt_ai_mgr_setting(setting: &str) -> String {
    let address = get_from_memory::<u32>(ZTAIMGR_ADDRESS_PTR);
    let offset = setting_to_address(setting);
    let value: bool = get_from_memory::<bool>(address + offset);
    format!("{}: {}", setting, value)
}

fn handle_set_bool_zt_ai_mgr_setting(setting: &str, value: String) -> String {
    let address = get_from_memory::<u32>(ZTAIMGR_ADDRESS_PTR);
    let offset = setting_to_address(setting);
    match parse_bool(&value) {
        Ok(setting_value) => {
            save_to_memory::<bool>(address + offset, setting_value);
            format!("{} set to {}", setting, setting_value)
        }
        Err(_) => {
            format!("invalid value: {}", value)
        }
    }
}
