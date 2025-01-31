use retour_utils::hook_module;
use tracing::info;

enum ZTLogLevel {
    Trace,
    Info,
    Note,
    Error,
    Fatal,
    BFLog,
}

impl ZTLogLevel {
    fn from_u32(value: u32) -> ZTLogLevel {
        match value {
            3 => ZTLogLevel::Trace,
            6 => ZTLogLevel::Info,
            9 => ZTLogLevel::Note,
            12 => ZTLogLevel::Error,
            15 => ZTLogLevel::Fatal,
            _ => ZTLogLevel::BFLog,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            ZTLogLevel::Trace => "TRACE",
            ZTLogLevel::Info => "INFO",
            ZTLogLevel::Note => "NOTE",
            ZTLogLevel::Error => "ERROR",
            ZTLogLevel::Fatal => "FATAL",
            ZTLogLevel::BFLog => "BFLOG",
        }
    }
}

pub fn log_from_zt(source_file: &String, line_number: u32, level: u32, message: &String) {
    let level = ZTLogLevel::from_u32(level);
    info!("{}({}) : {} : {}", source_file, line_number, level.as_str(), message);
}

#[hook_module("zoo.exe")]
mod zoo_logging {
    use crate::{capture_ztlog::log_from_zt, util::get_string_from_memory};

    #[hook(unsafe extern "cdecl" ZooLogging_LogHook, offset = 0x00001363)]
    fn zoo_log_func(source_file: u32, param_2: u32, param_3: u32, _param_4: u8, _param_5: u32, _param_6: u32, log_message: u32) {
        let source_file_string = get_string_from_memory(source_file);
        let log_message_string = get_string_from_memory(log_message);
        log_from_zt(&source_file_string, param_2, param_3, &log_message_string);
    }
}

pub fn init() {
    if let Err(e) = unsafe { zoo_logging::init_detours() } {
        info!("Error initialising zt logging detours: {}", e);
    };
}
