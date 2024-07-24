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
