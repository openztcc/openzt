use configparser::ini::Ini;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::path::Path;
use std::env;

const ZOO_INI: &str = "zoo.ini";

static INI_FILE: Lazy<Mutex<Ini>> = Lazy::new(|| {
    let mut config = Ini::new();
    match env::current_exe() {
        Ok(exe_path) => {
            let exe_path = exe_path.to_str().unwrap();
            let exe_path = Path::new(exe_path);
            let exe_path = exe_path.parent().unwrap();
            let exe_path = exe_path.join(ZOO_INI);
            config.load(exe_path).unwrap();
        }
        Err(e) => println!("failed to get current exe path: {e}"),
    };
    Mutex::new(config)
});

pub fn load_from_zoo_ini<T: std::str::FromStr>(section: &str, key: &str, default: T) -> T {
    let ini_file = INI_FILE.lock().unwrap();
    let value = ini_file.get(section, key);
    match value {
        Some(value) => {
            match value.parse::<T>() {
                Ok(value) => value,
                Err(e) => default
            }
        }
        None => default
    }

}

#[derive(Debug)]
pub struct DebugSettings {
    pub log_cutoff: i32,
    pub send_log_file: i32,
    pub send_message_box: i32,
    pub send_debugger: i32,
    pub delta_log_1: i32,
    pub delta_log_0: i32,
}

pub fn load_debug_settings(ini_path: &Path) -> DebugSettings {
    let mut debug_settings = DebugSettings {
        log_cutoff: 9,
        send_log_file: 0,
        send_message_box: 0,
        send_debugger: 0,
        delta_log_0: 0,
        delta_log_1: 0,
    };
    debug_settings = load_debug_settings_from_ini(debug_settings, ini_path);
    return debug_settings;
}

fn load_debug_settings_from_ini(mut debug_settings: DebugSettings, ini_path: &Path) -> DebugSettings {
    let mut zoo_ini = Ini::new();
    
    zoo_ini.load(ini_path).unwrap();

    let debug_header = "Debug";

    debug_settings.log_cutoff = load_int_with_default(&zoo_ini, debug_header, "LogCutoff", debug_settings.log_cutoff);

    debug_settings.send_log_file = load_int_with_default(&zoo_ini, debug_header, "SendLogfile", debug_settings.send_log_file);

    debug_settings.send_message_box = load_int_with_default(&zoo_ini, debug_header, "sendMessageBox", debug_settings.send_message_box);

    debug_settings.send_debugger = load_int_with_default(&zoo_ini, debug_header, "sendDebugger", debug_settings.send_debugger);

    debug_settings.delta_log_0 = load_int_with_default(&zoo_ini, debug_header, "deltaLog0", debug_settings.delta_log_0);

    debug_settings.delta_log_1 = load_int_with_default(&zoo_ini, debug_header, "deltaLog1", debug_settings.delta_log_1);

    return debug_settings;
}

pub fn load_int_with_default(ini_file: &Ini, section: &str, key: &str, default: i32) -> i32 {
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

pub fn load_string_with_default(ini_file: &Ini, section: &str, key: &str, default: &str) -> String {
    let value = ini_file.get(section, key);
    match value {
        Some(parsed_value) => parsed_value,
        None => default.to_string(),
    }
}