use std::fmt;

use retour_utils::hook_module;

// use crate::num;
// #[macro_use]
// use num::FromPrimitive;
use num_enum::FromPrimitive;

use crate::debug_dll::{get_string_from_memory, get_from_memory};
    
use crate::console::add_to_command_register;

const GLOBAL_ZTSCENARIOMGR_ADDRESS: u32 = 0x00638FF8;

// #[derive(FromPrimitive)]
#[derive(Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u32)]
enum ZtScenarioGoalType {
    Timer = 0x6354e4,
    Simple = 0x635510,
    #[num_enum(default)]
    Unknown = 0x0,
}

pub fn init() {
    unsafe { zoo_scenario_mgr::init_detours().unwrap() };
    add_to_command_register("get_ztscenariomgr".to_owned(), command_get_zt_scenario_mgr);
    add_to_command_register("list_goals".to_owned(), command_list_goals)
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

#[repr(C)]
struct ZTScenarioSimpleGoal {
    vftable: u32,
    param_1: u32,
    param_2: u32,
    rulea: u32,
    ruleb: u32,
    s_type: u32,
    value: u32,
    arga: u32,
    argb: u32,
    sticky: u32,
    hidden: u32,
    optional: u32,
    trulea: u32,
    truleb: u32,
    targa: u32,
    targb: u32,
}

#[repr(C)]
struct ZTScenarioMgr {
    vftable: u32,
    _padding1: [u8; 0x24 - 0x4],
    scenario_filename_str_start: u32,
    scenario_filename_str_end: u32,
    scenario_filename_str_buffer_end: u32,
    unkn_0x2c: u32,
    goal_array_start: u32,
    goal_array_end: u32,
    goal_array_buffer_end: u32,
}

impl fmt::Display for ZTScenarioMgr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scenario_filename_str = get_string_from_memory(self.scenario_filename_str_start);
        let goal_array_len = (self.goal_array_end - self.goal_array_start) / 0x4;
        write!(f, "ZTScenarioMgr; vftable: {:X},\n scenario_filename_str: {},\n goal_array_len: {}\n", self.vftable, scenario_filename_str, goal_array_len)
    }
}

impl fmt::Display for ZTScenarioTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_string = get_string_from_memory(self.display_string_start);
        let icon_uri = get_string_from_memory(self.icon_uri_start);
        write!(f, "ZTScenarioTimer; vftable: {:X},\n time_passed: {},\n num_months: {},\n display_flag: {},\n display_string: {},\n icon_uri: {}\n", self.vftable, self.state, self.num_months, self.display_flag, display_string, icon_uri)
    }
}

impl fmt::Display for ZTScenarioSimpleGoal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let param_2_string = get_string_from_memory(self.param_2);
        write!(f, "ZTScenarioSimpleGoal; vftable: {:X},\n param_1: {:X},\n param_2: {:X},\n param_2_string: {},\n rulea: {},\n ruleb: {},\n s_type: {},\n value: {},\n arga: {},\n argb: {},\n sticky: {},\n hidden: {},\n optional: {},\n trulea: {},\n truleb: {},\n targa: {},\n targb: {}\n", self.vftable, self.param_1, self.param_2, param_2_string, self.rulea, self.ruleb, self.s_type, self.value, self.arga, self.argb, self.sticky, self.hidden, self.optional, self.trulea, self.truleb, self.targa, self.targb)
        // write!(f, "ZTScenarioSimpleGoal; vftable: {:X},\n param_1: {:X},\n param_2: {:X},\n rulea: {},\n ruleb: {},\n s_type: {},\n value: {},\n arga: {},\n argb: {},\n sticky: {},\n hidden: {},\n optional: {},\n trulea: {},\n truleb: {},\n targa: {},\n targb: {}\n", self.vftable, self.param_1, self.param_2, self.rulea, self.ruleb, self.s_type, self.value, self.arga, self.argb, self.sticky, self.hidden, self.optional, self.trulea, self.truleb, self.targa, self.targb)

    }
}

fn read_zt_scenario_mgr_from_memory() -> ZTScenarioMgr {
    get_from_memory::<ZTScenarioMgr>(get_from_memory::<u32>(GLOBAL_ZTSCENARIOMGR_ADDRESS))
}

fn command_get_zt_scenario_mgr(_args: Vec<&str>) -> Result<String, &'static str> {
    let zt_scenario_mgr = read_zt_scenario_mgr_from_memory();
    Ok(format!("{}", zt_scenario_mgr))
}

fn command_list_goals(_args: Vec<&str>) -> Result<String, &'static str> {
    let zt_scenario_mgr = read_zt_scenario_mgr_from_memory();
    let goal_array_len = (zt_scenario_mgr.goal_array_end - zt_scenario_mgr.goal_array_start) / 0x4;
    let mut goals = String::new();
    for i in 0..goal_array_len {
        let goal_ptr = get_from_memory::<u32>(zt_scenario_mgr.goal_array_start + i * 0x4);
        match ZtScenarioGoalType::from(get_from_memory::<u32>(goal_ptr)) {
            ZtScenarioGoalType::Timer => {
                let goal = get_from_memory::<ZTScenarioTimer>(goal_ptr);
                goals.push_str(&format!("{:X} -> {}\n", goal_ptr, goal));
            },
            ZtScenarioGoalType::Simple => {
                let goal = get_from_memory::<ZTScenarioSimpleGoal>(goal_ptr);
                goals.push_str(&format!("{:X} -> {}\n", goal_ptr, goal));
            },
            ZtScenarioGoalType::Unknown => {
                goals.push_str(&format!("{:X} -> Unknown goal type\n", goal_ptr));
            }
        }
    }
    Ok(goals)
}

#[hook_module("zoo.exe")]
pub mod zoo_scenario_mgr {
    use tracing::info;

    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    use super::ZTScenarioTimer;

    #[hook(unsafe extern "thiscall" ZTScenarioTimer_ZTScenarioTimer, offset = 0x0018e218)]
    fn zt_scenario_timer(this_ptr: u32, param_1: u32) -> u32 {
        let return_value = unsafe { ZTScenarioTimer_ZTScenarioTimer.call(this_ptr, param_1) };
        info!("ZTScenarioTimer::ZTScenarioTimer({:X}, {:X}) -> {:X} -> {:X}", this_ptr, param_1, return_value, get_from_memory::<u32>(return_value));
        return_value
    }

    //this call no params offset 2425d
    #[hook(unsafe extern "thiscall" ZTScenarioTimer_update, offset = 0x0002425d)]
    fn zt_scenario_mgr(this_ptr: u32, param_1: u32) {
        let before_timer = get_from_memory::<ZTScenarioTimer>(this_ptr);
        unsafe { ZTScenarioTimer_update.call(this_ptr, param_1) };
        info!("ZTScenarioTimer::update({:X}, {})", this_ptr, param_1);
        let after_timer = get_from_memory::<ZTScenarioTimer>(this_ptr);
        info!("Before: {}", before_timer);
        info!("After: {}", after_timer);
    }
    //this call 4 params offset 1950ab
    #[hook(unsafe extern "thiscall" ZTScenarioTimer_load, offset = 0x0001950ab)]
    fn zt_scenario_mgr_2(this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) {
        let before_timer = get_from_memory::<ZTScenarioTimer>(this_ptr);
        unsafe { ZTScenarioTimer_load.call(this_ptr, param_1, param_2, param_3) };
        let after_timer = get_from_memory::<ZTScenarioTimer>(this_ptr);
        info!("ZTScenarioTimer::load({:X}, {:X}, {:X}, {:X})", this_ptr, param_1, param_2, param_3);
        info!("Before: {}", before_timer);
        info!("After: {}", after_timer);
    }

    //this call 1 param offset 7a04c
    #[hook(unsafe extern "thiscall" ZTScenarioTimer_save, offset = 0x0007a04c)]
    fn zt_scenario_mgr_3(this_ptr: u32, param_1: u32) {
        let before_timer = get_from_memory::<ZTScenarioTimer>(this_ptr);
        unsafe { ZTScenarioTimer_save.call(this_ptr, param_1) };
        let after_timer = get_from_memory::<ZTScenarioTimer>(this_ptr);
        info!("ZTScenarioTimer::save({:X}, {:X})", this_ptr, param_1);
        info!("Before: {}", before_timer);
        info!("After: {}", after_timer);
    }
}