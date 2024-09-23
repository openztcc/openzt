use super::util::{Address, Setting, GettableSettable};

const SHOW_BUILDING_AI_INFO_ADDRESS: u32 = 0x00638fc8;

const ZTAIMGR_ADDRESS_PTR: u32 = 0x00638098;

const SHOW_AI_INFO_OFFSET: u32 = 0xf4;
const SHOW_NAME_OFFSET: u32 = 0xf8;
const SHOW_POSITION_OFFSET: u32 = 0xfc;
const SHOW_STATUS_VARS_OFFSET: u32 = 0x100;
const SHOW_FUNCTION_CALL_OFFSET: u32 = 0x108;
const SHOW_EVENTS_OFFSET: u32 = 0x10c;
const SHOW_SELECTED_OFFSET: u32 = 0x104;
const SHOW_FRAME_OFFSET: u32 = 0x114;
const SHOW_GOAL_OFFSET: u32 = 0x118;
const AI_INFO_NTH_OFFSET: u32 = 0x110;

pub fn get_settings() -> Vec<Box<dyn GettableSettable>> {
    vec![
        Box::new(SHOW_BUILDING_AI_INFO),
        Box::new(SHOW_AI_INFO),
        Box::new(SHOW_NAME),
        Box::new(SHOW_POSITION),
        Box::new(SHOW_STATUS_VARS),
        Box::new(SHOW_FUNCTION_CALL),
        Box::new(SHOW_EVENTS),
        Box::new(SHOW_SELECTED),
        Box::new(SHOW_FRAME),
        Box::new(SHOW_GOAL),
        Box::new(AI_INFO_NTH),
    ]
}

// TODO: Fill in all the keys, inline the addresses
const SHOW_BUILDING_AI_INFO: Setting<bool> = Setting {
    header: "AI",
    key: "ShowBuildingAIInfo",
    address: Address::Global(SHOW_BUILDING_AI_INFO_ADDRESS),
    default: false,
};
const SHOW_AI_INFO: Setting<bool> = Setting {
    header: "AI",
    key: "ShowAIInfo",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_AI_INFO_OFFSET),
    default: false,
};
const SHOW_NAME: Setting<bool> = Setting {
    header: "AI",
    key: "ShowName",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_NAME_OFFSET),
    default: false,
};
const SHOW_POSITION: Setting<bool> = Setting {
    header: "AI",
    key: "ShowPosition",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_POSITION_OFFSET),
    default: false,
};
const SHOW_STATUS_VARS: Setting<bool> = Setting {
    header: "AI",
    key: "ShowStatusVars",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_STATUS_VARS_OFFSET),
    default: false,
};
const SHOW_FUNCTION_CALL: Setting<bool> = Setting {
    header: "AI",
    key: "ShowFunctionCall",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_FUNCTION_CALL_OFFSET),
    default: false,
};
const SHOW_EVENTS: Setting<bool> = Setting {
    header: "AI",
    key: "ShowEvents",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_EVENTS_OFFSET),
    default: false,
};
const SHOW_SELECTED: Setting<bool> = Setting {
    header: "AI",
    key: "ShowSelected",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_SELECTED_OFFSET),
    default: false,
};
const SHOW_FRAME: Setting<bool> = Setting {
    header: "AI",
    key: "ShowFrame",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_FRAME_OFFSET),
    default: false,
};
const SHOW_GOAL: Setting<bool> = Setting {
    header: "AI",
    key: "ShowGoal",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, SHOW_GOAL_OFFSET),
    default: false,
};
const AI_INFO_NTH: Setting<bool> = Setting {
    header: "AI",
    key: "AIInfoNth",
    address: Address::Indirect(ZTAIMGR_ADDRESS_PTR, AI_INFO_NTH_OFFSET),
    default: false,
};
