use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    /// Detour error encountered within the [`retour`] crate
    DetourError(retour::Error),
    /// Module trying to be hooked is not currently loaded in memory
    ModuleNotLoaded,
}

impl From<retour::Error> for Error {
    fn from(value: retour::Error) -> Self {
        Error::DetourError(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DetourError(e) => f.write_fmt(format_args!("Detour Error: {e:?}")),
            Error::ModuleNotLoaded => f.write_str("Module trying to be hooked is not currently loaded in memory"),
        }
    }
}
impl std::error::Error for Error {}
