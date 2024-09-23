use crate::command_console::{CommandError, add_to_command_register};
use tracing::error;
use retour_utils::hook_module;

mod ai;
mod debug;
mod util;

fn command_get_setting(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() != 2 {
        return Err(format!("Got {} arguments, expected 2", args.len()).into());
    }
    if args[0].is_empty() || args[1].is_empty() {
        return Err("Empty arguments".into());
    }
    ai::get_settings().iter().chain(debug::get_settings().iter())
        .find_map(|setting| {
            if setting.check(args[0], args[1]) {
                Some(setting.get())
            } else {
                None
            }
        })
        .ok_or(format!("Setting {} {} not found", args[0], args[1]).into())
}

fn command_set_setting(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() != 3 {
        return Err(format!("Got {} arguments, expected 3", args.len()).into());
    }
    if args[0].is_empty() || args[1].is_empty() || args[2].is_empty() {
        return Err("Empty arguments".into());
    }
    let Some(setting) = ai::get_settings().into_iter().chain(debug::get_settings().into_iter())
        .find_map(|setting| {
            if setting.check(args[0], args[1]) {
                Some(setting)
            } else {
                None
            }
        }) else {
            return Err(format!("Setting {} {} not found", args[0], args[1]).into());
    };
    
    setting.set(args[2])?;
    Ok("Success".to_string())
}

fn command_list_settings(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() > 1 {
        return Err(format!("Got {} arguments, expected at most 1", args.len()).into());
    }
    ai::get_settings().iter().chain(debug::get_settings().iter())
        .filter(|setting| args.is_empty() || setting.check(args[0], ""))
        .find_map(|setting| {
            if setting.check(args[0], args[1]) {
                Some(setting.get())
            } else {
                None
            }
        })
        .ok_or(format!("Setting {} {} not found", args[0], args[1]).into())
}

#[hook_module("zoo.exe")]
mod zoo_ini_loading {
    use tracing::info;

    #[hook(unsafe extern "cdecl" LoadDebugSettingsFromIniHook, offset = 0x00179f4c)]
    fn load_debug_settings_from_ini_detour() -> u32 {
        let result = unsafe { LoadDebugSettingsFromIniHook.call() };
        info!("######################LoadDebugSettingsFromIniHook: {}", result);
        result
    }
}


pub fn init() {
    add_to_command_register("get_setting".to_string(), command_get_setting);
    add_to_command_register("set_setting".to_string(), command_set_setting);
    add_to_command_register("list_settings".to_string(), command_list_settings);
    if unsafe { zoo_ini_loading::init_detours() }.is_err() {
        error!("Error initialising load ini detours");
    };
}