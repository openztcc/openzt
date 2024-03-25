// ztgamemgr module has functions to interact with the live zoo stats such as cash, num animals, species, guests, etc.

use crate::add_to_command_register;
use crate::debug_dll::{get_from_memory, get_string_from_memory};

use tracing::info;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use num_enum::FromPrimitive;

const GLOBAL_ZTGAMEMGR_ADDRESS: u32 = 0x00638048;

// ZTGameMgr struct
#[derive(Debug)]
#[repr(C)]
struct ZTGameMgr {
    instance: u32, // 0x0
    get_date: String, // 0x1194 (8 bytes)
    get_cash: f32, // 0x0C
    set_cash: f32, // 0x0C
    add_cash: f32, // 0x0C
    sub_cash: f32, // 0x0C
    num_animals: u32, // 0x30
    num_species: u32, // 0x38
    num_guests: u32, // 0x54
    num_tired_guests: u32, // 0x3C
    num_hungry_guests: u32, // 0x40
    num_thirst_guests: u32, // 0x44
    num_guests_restroom_need: u32, // 0x48
    num_guests_in_filter: u32, // 0x54
    zoo_admission_cost: f32, // 0x1160
    enable_dev_mode: bool, // 0x63858A
}

// SYSTEMTIME struct from Windows API
#[derive(Debug)]
#[repr(C)]
struct SYSTEMTIME {
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
    // returns the address of the ZTGameMgr instance in memory
    fn instance() -> u32 {
        get_from_memory::<u32>(GLOBAL_ZTGAMEMGR_ADDRESS)
    }

    // returns the SYSTEMTIME struct in memory
    fn get_date() -> SYSTEMTIME {
        let date = get_from_memory::<SYSTEMTIME>(Self::instance() + 0x1194);
        date
    }

    // returns the cash in the player's account
    fn get_cash() -> f32 {
        get_from_memory::<f32>(Self::instance() + 0x0C)
    }

    // sets the cash in the player's account
    fn set_cash(cash: f32) {
        let cash_address = Self::instance() + 0x0C;
        unsafe {
            *(cash_address as *mut f32) = cash;
        }
    }

    // adds cash to the player's account
    fn add_cash(cash: f32) {
        let current_cash = Self::get_cash();
        Self::set_cash(current_cash + cash);
    }

    // subtracts cash from the player's account
    fn sub_cash(cash: f32) {
        let current_cash = Self::get_cash();
        Self::set_cash(current_cash - cash);
    }

    // returns the number of animals in the zoo
    fn num_animals() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x30)
    }

    // returns the number of species in the zoo
    fn num_species() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x38)
    }

    // returns the number of guests in the zoo
    fn num_guests() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x54)
    }

    // returns the number of tired guests in the zoo
    fn num_tired_guests() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x3C)
    }

    // returns the number of hungry guests in the zoo
    fn num_hungry_guests() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x40)
    }

    // returns the number of thirsty guests in the zoo
    fn num_thirst_guests() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x44)
    }

    // returns the number of guests that need to use the restroom
    fn num_guests_restroom_need() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x48)
    }

    // returns the number of guests in the filter
    fn num_guests_in_filter() -> u32 {
        get_from_memory::<u32>(Self::instance() + 0x54)
    }

    // returns the zoo admission cost
    fn zoo_admission_cost() -> f32 {
        get_from_memory::<f32>(Self::instance() + 0x1160)
    }

    // enables or disables dev mode
    fn enable_dev_mode(enable: bool) {
        let enable_dev_mode_address = 0x63858A;
        unsafe {
            *(enable_dev_mode_address as *mut bool) = enable;
        }
    }
}

// prints the SYSTEMTIME struct in memory in a human-readable format
// usage: get_date
pub fn command_get_date_str(_args: Vec<&str>) -> Result<String, &'static str> {
    let date = get_from_memory::<SYSTEMTIME>(ZTGameMgr::instance() + 0x1194);

    info!("Date: {:#?}", date);

    Ok(format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", date.w_year, date.w_month, date.w_day, date.w_hour, date.w_minute, date.w_second))
}

// adds cash to the player's account
// usage: add_cash <amount>
pub fn command_add_cash(_args: Vec<&str>) -> Result<String, &'static str> {
    let cash = _args[0].parse::<f32>().unwrap();
    ZTGameMgr::add_cash(cash);
    Ok(format!("Added ${}", cash))
}

// enables or disables dev mode
// usage: enable_dev_mode <true/false>
pub fn command_enable_dev_mode(_args: Vec<&str>) -> Result<String, &'static str> {
    let enable = _args[0].parse::<bool>().unwrap();
    ZTGameMgr::enable_dev_mode(enable);
    Ok(format!("Dev mode enabled: {}", enable))
}

// prints various stats about the zoo
// usage: zoostats
pub fn command_zoostats(_args: Vec<&str>) -> Result<String, &'static str> {
    Ok(format!("\nBudget: {}\nAnimals: {}\nSpecies: {}\nGuests: {}\nTired Guests: {}\nHungry Guests: {}\nThirsty Guests: {}\nGuests Need Restroom: {}\nGuests in Filter: {}\nZoo Admission Cost: ${}", ZTGameMgr::get_cash(), ZTGameMgr::num_animals(), ZTGameMgr::num_species(), ZTGameMgr::num_guests(), ZTGameMgr::num_tired_guests(), ZTGameMgr::num_hungry_guests(), ZTGameMgr::num_thirst_guests(), ZTGameMgr::num_guests_restroom_need(), ZTGameMgr::num_guests_in_filter(), ZTGameMgr::zoo_admission_cost()))
}

// registers the commands with the command register
pub fn init() {
    add_to_command_register("get_date".to_string(), command_get_date_str);
    add_to_command_register("add_cash".to_string(), command_add_cash);
    add_to_command_register("enable_dev_mode".to_string(), command_enable_dev_mode);
    add_to_command_register("zoostats".to_string(), command_zoostats);
}    

