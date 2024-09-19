use bf_configparser::ini::Ini;

use std::fmt;

use crate::util::{get_from_memory, save_to_memory};

/// A setting that is stored at a specific memory address
pub struct GlobalSetting<T> {
    pub header: &'static str,
    pub key: &'static str,
    pub address: u32,
    pub default: T,
}

/// A setting stored inside a ZT*Mgr class
pub struct MgrSetting<T> {
    pub header: &'static str,
    pub key: &'static str,
    pub address: u32,
    pub offset: u32,
    pub default: T,
}

impl<T> GlobalSetting<T> {
    fn write(&self, value: T) {
        save_to_memory::<T>(self.address, value);
    }

    fn read(&self) -> T {
        get_from_memory::<T>(self.address)
    }
}

impl GlobalSetting<i32> {
    fn load_from_ini(&self, ini: &Ini) -> i32 {
        value_or_default(ini.get_parse(self.header, self.key), self.default)
    }
}

impl GlobalSetting<bool> {
    fn load_from_ini(&self, ini: &Ini) -> bool {
        match ini.get(self.header, self.key) {
            Some(value) => match value.as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => self.default,
            },
            None => self.default,
        }
    }
}

impl<T: std::fmt::Display> std::fmt::Display for GlobalSetting<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GlobalSetting: header: {}, key: {}, address: {}, default: {}", self.header, self.key, self.address, self.default)
    }
}

impl<T> MgrSetting<T> {
    fn write(&self, value: T) {
        let address = get_from_memory::<u32>(self.address);
        save_to_memory::<T>(address + self.offset, value);
    }

    fn read(&self) -> T {
        let address = get_from_memory::<u32>(self.address);
        get_from_memory::<T>(address + self.offset)
    }
}

impl MgrSetting<i32> {
    fn load_from_ini(&self, ini: &Ini) -> i32 {
        value_or_default(ini.get_parse(self.header, self.key), self.default)
    }
}


impl MgrSetting<bool> {
    fn load_from_ini(&self, ini: &Ini) -> bool {
        match ini.get(self.header, self.key) {
            Some(value) => match value.as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => self.default,
            },
            None => self.default,
        }
    }
}

pub fn value_or_default<T>(value: Result<Option<T>, String>, default: T) -> T {
    match value {
        Ok(Some(v)) => v,
        Ok(None) => default,
        Err(_) => default,
    }
}