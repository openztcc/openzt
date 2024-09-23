use tracing::info;

use crate::{
    command_console::{add_to_command_register, CommandError},
    util::get_from_memory,
};

const GLOBAL_ZTGAMEMGR_ADDRESS: u32 = 0x00638048;

/// ZTGameMgr struct
#[derive(Debug)]
#[repr(C)]
struct ZTGameMgr {
    pad1: [u8; 0x0C],
    cash: f32,                     // 0x0C
    pad2: [u8; 0x30 - 0x10],       // 0x0C
    num_animals: u16,              // 0x30
    pad3: [u8; 0x38 - 0x32],       // 0x30
    num_species: u16,              // 0x38
    pad4: [u8; 0x3C - 0x3A],       // 0x38
    num_tired_guests: u16,         // 0x3C
    pad5: [u8; 0x40 - 0x3E],       // 0x3C
    num_hungry_guests: u16,        // 0x40
    pad6: [u8; 0x44 - 0x42],       // 0x40
    num_thirst_guests: u16,        // 0x44
    pad7: [u8; 0x48 - 0x46],       // 0x44
    num_guests_restroom_need: u16, // 0x48
    pad8: [u8; 0x54 - 0x4A],       // 0x48
    num_guests: u16,               // 0x54
    pad9: [u8; 0x1160 - 0x56],     // 0x54
    zoo_admission_cost: f32,       // 0x1160
    pad10: [u8; 0x1194 - 0x1164],  // 0x1160
    date: Systemtime,              // 0x1194
    pad11: [u8; 0x1400],           // 0x1194
}

/// SYSTEMTIME struct from Windows API
/// TODO: Replace this with the actual SYSTEMTIME struct from the Windows API
#[derive(Debug, Clone)]
#[repr(C)]
struct Systemtime {
    w_year: u16,
    w_month: u16,
    w_day_of_week: u16,
    w_day: u16,
    w_hour: u16,
    w_minute: u16,
    w_second: u16,
    w_milliseconds: u16,
}

impl ZTGameMgr {
    /// enables or disables dev mode
    fn enable_dev_mode(enable: bool) {
        let enable_dev_mode_address = 0x63858A;
        unsafe {
            *(enable_dev_mode_address as *mut bool) = enable;
        }
    }

    /// returns the instance of the ZTGameMgr struct
    fn instance() -> Option<&'static mut ZTGameMgr> {
        unsafe {
            // get the pointer to the ZTGameMgr instance
            let ptr = get_from_memory::<*mut ZTGameMgr>(GLOBAL_ZTGAMEMGR_ADDRESS);

            // is pointer null
            if !ptr.is_null() {
                Some(&mut *ptr)
            } else {
                // pointer is null
                None
            }
        }
    }
}

/// a command that prints the SYSTEMTIME struct in memory in a human-readable format
/// usage: `get_date`
pub fn command_get_date_str(_args: Vec<&str>) -> Result<String, CommandError> {
    let ztgamemgr = ZTGameMgr::instance().ok_or("Failed to get ZTGameMgr instance")?;
    let date = ztgamemgr.date.clone();
    info!("Date: {:#?}", date);

    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        date.w_year, date.w_month, date.w_day, date.w_hour, date.w_minute, date.w_second
    ))
}

/// a command that adds cash to the player's account
/// usage: `add_cash <amount>`
pub fn command_add_cash(args: Vec<&str>) -> Result<String, CommandError> {
    let ztgamemgr = ZTGameMgr::instance().ok_or("Failed to get ZTGameMgr instance")?;
    ztgamemgr.cash += args[0].parse::<f32>()?;
    Ok(format!("Added ${}", args[0]))
}

/// a command that enables or disables dev mode
/// usage: `enable_dev_mode <true/false>`
pub fn command_enable_dev_mode(args: Vec<&str>) -> Result<String, CommandError> {
    let enable = args[0].parse()?;
    ZTGameMgr::enable_dev_mode(enable);
    Ok(format!("Dev mode enabled: {}", enable))
}

/// a command that prints various stats about the zoo
/// usage: `zoostats`
pub fn command_zoostats(_args: Vec<&str>) -> Result<String, CommandError> {
    let ztgamemgr = ZTGameMgr::instance().ok_or("Failed to get ZTGameMgr instance")?;
    Ok(format!("\nBudget: {}\nAnimals: {}\nSpecies: {}\nTired Guests: {}\nHungry Guests: {}\nThirsty Guests: {}\nGuests Need Restroom: {}\nNum Guests: {}\nZoo Admission Cost: ${}", ztgamemgr.cash, ztgamemgr.num_animals, ztgamemgr.num_species, ztgamemgr.num_tired_guests, ztgamemgr.num_hungry_guests, ztgamemgr.num_thirst_guests, ztgamemgr.num_guests_restroom_need, ztgamemgr.num_guests, ztgamemgr.zoo_admission_cost))
}

/// registers the commands with the command register
pub fn init() {
    add_to_command_register("get_date".to_string(), command_get_date_str);
    add_to_command_register("add_cash".to_string(), command_add_cash);
    add_to_command_register("enable_dev_mode".to_string(), command_enable_dev_mode);
    add_to_command_register("zoostats".to_string(), command_zoostats);
}
