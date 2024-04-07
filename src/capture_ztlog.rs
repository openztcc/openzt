use tracing::info;

enum ZTLogLevel {
    ZTLogLevelTrace,
    ZTLogLevelInfo,
    ZTLogLevelNote,
    ZTLogLevelError,
    ZTLogLevelFatal,
    ZTLogLevelBFLog,
}

impl ZTLogLevel {
    fn from_u32(value: u32) -> ZTLogLevel {
        match value {
            3 => ZTLogLevel::ZTLogLevelTrace,
            6 => ZTLogLevel::ZTLogLevelInfo,
            9 => ZTLogLevel::ZTLogLevelNote,
            12 => ZTLogLevel::ZTLogLevelError,
            15 => ZTLogLevel::ZTLogLevelFatal,
            _ => ZTLogLevel::ZTLogLevelBFLog,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            ZTLogLevel::ZTLogLevelTrace => "TRACE",
            ZTLogLevel::ZTLogLevelInfo => "INFO",
            ZTLogLevel::ZTLogLevelNote => "NOTE",
            ZTLogLevel::ZTLogLevelError => "ERROR",
            ZTLogLevel::ZTLogLevelFatal => "FATAL",
            ZTLogLevel::ZTLogLevelBFLog => "BFLOG",
        }
    }
}

pub fn log_from_zt(source_file: &String, line_number: u32, level: u32, message: &String) {
    let level = ZTLogLevel::from_u32(level);
    info!(
        "{}({}) : {} : {}",
        source_file,
        line_number,
        level.as_str(),
        message
    );
}
