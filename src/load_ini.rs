use configparser::ini::Ini;

#[derive(Debug)]
pub struct DebugSettings {
    log_cutoff: i32,
    send_logfile: i32,
    send_message_box: i32,
    send_debugger: i32,
    delta_log1: i32,
    delta_log0: i32,
}

pub fn load_debug_settings() -> DebugSettings {
    let mut debug_settings = DebugSettings {
        log_cutoff: 9,
        send_logfile: 0,
        send_message_box: 0,
        send_debugger: 0,
        delta_log0: 0,
        delta_log1: 0,
    };
    debug_settings = load_debug_settings_from_ini(debug_settings);
    return debug_settings;
}

// pub unsafe fn zt_load_debug_settings() -> i32 {
//     let 
//     let debug_settings = load_debug_settings();

//     return 1;
// }

fn load_debug_settings_from_ini(mut debug_settings: DebugSettings) -> DebugSettings {
    let mut zoo_ini = Ini::new();
    
    zoo_ini.load("zoo.ini").unwrap();

    let debug_header = "Debug";

    debug_settings.log_cutoff = load_int_with_default(&zoo_ini, debug_header, "LogCutoff", debug_settings.log_cutoff);

    debug_settings.send_logfile = load_int_with_default(&zoo_ini, debug_header, "SendLogfile", debug_settings.send_logfile);

    debug_settings.send_message_box = load_int_with_default(&zoo_ini, debug_header, "sendMessageBox", debug_settings.send_message_box);

    debug_settings.send_debugger = load_int_with_default(&zoo_ini, debug_header, "sendDebugger", debug_settings.send_debugger);

    debug_settings.delta_log0 = load_int_with_default(&zoo_ini, debug_header, "deltaLog0", debug_settings.delta_log0);

    debug_settings.delta_log1 = load_int_with_default(&zoo_ini, debug_header, "deltaLog1", debug_settings.delta_log1);

    return debug_settings;
}

fn load_int_with_default(ini_file: &Ini, section: &str, key: &str, default: i32) -> i32 {
    let value = ini_file.getint(section, key);

    match value {
        Ok(inner_value) => {
            match inner_value {
                Some(parsed_value) => parsed_value as i32,
                None => default,
            }
        },
        Err(_) => default,
    }
}