use std::fmt;

use retour_utils::hook_module;

use crate::debug_dll::get_string_from_memory;

pub fn init() {
    unsafe { zoo_scenario_mgr::init_detours().unwrap() };
}

#[repr(C)]
#[derive(Debug)]
struct ZTScenarioTimer {
    vftable: u32,
    state: u32,
    num_months: u32,
    display_flag: u32,
    display_string_start: u32,
    display_string_end: u32,
    display_string_buffer_end: u32,
    icon_uri_start: u32,
    icon_uri_end: u32,
    icon_uri_buffer_end: u32,
}

impl fmt::Display for ZTScenarioTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_string = get_string_from_memory(self.display_string_start);
        let icon_uri = get_string_from_memory(self.icon_uri_start);
        write!(f, "ZTScenarioTimer; vftable: {:X}\n, time_passed: {}\n, num_months: {}\n, display_flag: {}\n, display_string: {}\n, icon_uri: {}\n", self.vftable, self.state, self.num_months, self.display_flag, display_string, icon_uri)
    }
}

#[hook_module("zoo.exe")]
pub mod zoo_scenario_mgr {
    use tracing::info;

    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    #[hook(unsafe extern "thiscall" ZTScenarioTimer_ZTScenarioTimer, offset = 0x0018e218)]
    fn zt_scenario_timer(this_ptr: u32, param_1: u32) -> u32 {
        let return_value = unsafe { ZTScenarioTimer_ZTScenarioTimer.call(this_ptr, param_1) };
        info!("ZTScenarioTimer::ZTScenarioTimer({:X}, {:X}) -> {:X} -> {:X}", this_ptr, param_1, return_value, get_from_memory::<u32>(return_value));
        return_value
    }

    //this call no params offset 2425d
    #[hook(unsafe extern "thiscall" ZTScenarioTimer_update, offset = 0x0002425d)]
    fn zt_scenario_mgr(this_ptr: u32, param_1: u32) {
        unsafe { ZTScenarioTimer_update.call(this_ptr, param_1) };
        info!("ZTScenarioTimer::update({:X}, {})", this_ptr, param_1);
    }
    //this call 4 params offset 1950ab
    #[hook(unsafe extern "thiscall" ZTScenarioTimer_load, offset = 0x0001950ab)]
    fn zt_scenario_mgr_2(this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) {
        unsafe { ZTScenarioTimer_load.call(this_ptr, param_1, param_2, param_3) };
        info!("ZTScenarioTimer::load({:X}, {:X}, {:X}, {:X})", this_ptr, param_1, param_2, param_3);
    }

    //this call 1 param offset 7a04c
    #[hook(unsafe extern "thiscall" ZTScenarioTimer_save, offset = 0x0007a04c)]
    fn zt_scenario_mgr_3(this_ptr: u32, param_1: u32) {
        unsafe { ZTScenarioTimer_save.call(this_ptr, param_1) };
        info!("ZTScenarioTimer::save({:X}, {:X})", this_ptr, param_1);
    }
}