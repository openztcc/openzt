use std::path::Path;

use bf_configparser::ini::Ini;

use super::util::{value_or_default, GlobalSetting, MgrSetting};

// TODO: Move all these inline
const SEND_DEBUGGER_ADDRESS: u32 = 0x00643e44;
const SEND_LOG_FILE_ADDRESS: u32 = 0x00643e48;
const SEND_MESSAGE_BOX_ADDRESS: u32 = 0x00643e4a;
const DELTA_LOG_0_ADDRESS: u32 = 0x00638054;
const DELTA_LOG_1_ADDRESS: u32 = 0x0064bd7c;
const LOG_CUTOFF_ADDRESS: u32 = 0x0063804c;

const SHOW_BUILDING_AI_INFO_ADDRESS: u32 = 0x00638fc8;

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

pub const EXE_LOCATION_ADDRESS: u32 = 0x0064BEDC;
pub const EXE_LOCATION_ADDRESS_2: u32 = 0x0064BED8;
pub const EXE_LOCATION_ADDRESS_3: u32 = 0x0064A800;

const LOG_CUTTOFF: GlobalSetting<u32> = GlobalSetting {
    header: "Debug",
    key: "LogCutoff",
    address: LOG_CUTOFF_ADDRESS,
    default: 9,
};
const SEND_DEBUGGER: GlobalSetting<bool> = GlobalSetting {
    header: "Debug",
    key: "sendDebugger",
    address: SEND_DEBUGGER_ADDRESS,
    default: false,
};
const SEND_LOG_FILE: GlobalSetting<bool> = GlobalSetting {
    header: "Debug",
    key: "sendLogFile",
    address: SEND_LOG_FILE_ADDRESS,
    default: false,
};
const SEND_MESSAGE_BOX: GlobalSetting<bool> = GlobalSetting {
    header: "Debug",
    key: "sendMessageBox",
    address: SEND_MESSAGE_BOX_ADDRESS,
    default: false,
};
const DELTA_LOG_0: GlobalSetting<bool> = GlobalSetting {
    header: "Debug",
    key: "deltaLog0",
    address: DELTA_LOG_0_ADDRESS,
    default: false,
};
const DELTA_LOG_1: GlobalSetting<bool> = GlobalSetting {
    header: "Debug",
    key: "deltaLog1",
    address: DELTA_LOG_1_ADDRESS,
    default: false,
};

//TODO: Find these debug settings? Technically they are in
// drawfps=1           // ZTApp + 0x509  // 004bc707
// drawfpsx=720        // ZTApp + 0x50c  // 004bc7b0
// drawfpsy=20         // ZTApp + 0x510  // 004bc854

const SHOW_BUILDING_AI_INFO: GlobalSetting<bool> = GlobalSetting {
    header: "AI",
    key: "deltaLog1",
    address: SHOW_BUILDING_AI_INFO_ADDRESS,
    default: false,
};
const SHOW_AI_INFO: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_AI_INFO_OFFSET,
    default: false,
};
const SHOW_NAME: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_NAME_OFFSET,
    default: false,
};
const SHOW_POSITION: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_POSITION_OFFSET,
    default: false,
};
const SHOW_STATUS_VARS: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_STATUS_VARS_OFFSET,
    default: false,
};
const SHOW_FUNCTION_CALL: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_FUNCTION_CALL_OFFSET,
    default: false,
};
const SHOW_EVENTS: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_EVENTS_OFFSET,
    default: false,
};
const SHOW_SELECTED: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_SELECTED_OFFSET,
    default: false,
};
const SHOW_FRAME: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_FRAME_OFFSET,
    default: false,
};
const SHOW_GOAL: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_GOAL_OFFSET,
    default: false,
};
const AI_INFO_NTH: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: AI_INFO_NTH_OFFSET,
    default: false,
};

// }, "LogCutoff", "Debug", LOG_CUTOFF_ADDRESS, 9> = GlobalSetting{ default: 2 };

// impl _DebugSettings {
//     pub fn new() -> Self {
//         Self {
//             log_cutoff: GlobalSetting::new("Debug", "LogCutoff", LOG_CUTOFF_ADDRESS, 9),
//             send_log_file: GlobalSetting::new("Debug", "SendLogfile", SEND_LOG_FILE_ADDRESS, false),
//             send_message_box: GlobalSetting::new("Debug", "sendMessageBox", SEND_MESSAGE_BOX_ADDRESS, false),
//             send_debugger: GlobalSetting::new("Debug", "sendDebugger", SEND_DEBUGGER_ADDRESS, false),
//             delta_log_0: GlobalSetting::new("Debug", "deltaLog0", DELTA_LOG_0_ADDRESS, false),
//             delta_log_1: GlobalSetting::new("Debug", "deltaLog1", DELTA_LOG_1_ADDRESS, false),
//         }
//     }
// }

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
    let debug_settings = DebugSettings {
        log_cutoff: 9,
        send_log_file: 0,
        send_message_box: 0,
        send_debugger: 0,
        delta_log_0: 0,
        delta_log_1: 0,
    };
    load_debug_settings_from_ini(debug_settings, ini_path)
}

fn load_debug_settings_from_ini(mut debug_settings: DebugSettings, ini_path: &Path) -> DebugSettings {
    let mut zoo_ini = Ini::new();

    zoo_ini.load(ini_path).unwrap();

    let debug_header = "Debug";

    debug_settings.log_cutoff = value_or_default(zoo_ini.get_parse(debug_header, "LogCutoff"), debug_settings.log_cutoff);

    debug_settings.send_log_file = value_or_default(zoo_ini.get_parse(debug_header, "SendLogfile"), debug_settings.send_log_file);

    debug_settings.send_message_box = value_or_default(zoo_ini.get_parse(debug_header, "sendMessageBox"), debug_settings.send_message_box);

    debug_settings.send_debugger = value_or_default(zoo_ini.get_parse(debug_header, "sendDebugger"), debug_settings.send_debugger);

    debug_settings.delta_log_0 = value_or_default(zoo_ini.get_parse(debug_header, "deltaLog0"), debug_settings.delta_log_0);

    debug_settings.delta_log_1 = value_or_default(zoo_ini.get_parse(debug_header, "deltaLog1"), debug_settings.delta_log_1);

    debug_settings
}

// fn load_debug_settings_from_zoo_ini() {
//     util::debug_logger("load_debug_settings_from_ini");
//     util::log_exe_location_memory_value();
//     util::log_debug_ini_memory_values();
//     let mut base_path = util::get_base_path();
//     base_path.push("zoo.ini");
//     let debug_settings = load_ini::load_debug_settings(base_path.as_path());
//     util::debug_logger("Saving debug ini settings");
//     util::save_debug_settings(debug_settings);
//     util::log_debug_ini_memory_values();
// }