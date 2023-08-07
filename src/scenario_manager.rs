use retour_utils::hook_module;

pub fn init() {
    unsafe { zoo_scenario_mgr::init_detours().unwrap() };
}

#[hook_module("zoo.exe")]
pub mod zoo_scenario_mgr {
    use tracing::info;

    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    #[hook(unsafe extern "thiscall" ZTScenarioTimer_ZTScenarioTimer, offset = 0x0018e218)]
    fn zt_scenario_timer(this_ptr: u32, param_1: u32) -> u32 {
        let return_value = unsafe { ZTScenarioTimer_ZTScenarioTimer.call(this_ptr, param_1) };
        info!("ZTScenarioTimer::ZTScenarioTimer({:X}, {:X}) -> {:X}", this_ptr, param_1, return_value);
        return_value
    }
}