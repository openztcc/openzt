use super::util::{GlobalSetting, MgrSetting};

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

const SHOW_BUILDING_AI_INFO: GlobalSetting<bool> = GlobalSetting {
    header: "AI",
    key: "",
    address: SHOW_BUILDING_AI_INFO_ADDRESS,
    default: false,
};
const SHOW_AI_INFO: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_AI_INFO_OFFSET,
    default: false,
};
const SHOW_NAME: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_NAME_OFFSET,
    default: false,
};
const SHOW_POSITION: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_POSITION_OFFSET,
    default: false,
};
const SHOW_STATUS_VARS: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_STATUS_VARS_OFFSET,
    default: false,
};
const SHOW_FUNCTION_CALL: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_FUNCTION_CALL_OFFSET,
    default: false,
};
const SHOW_EVENTS: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_EVENTS_OFFSET,
    default: false,
};
const SHOW_SELECTED: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_SELECTED_OFFSET,
    default: false,
};
const SHOW_FRAME: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_FRAME_OFFSET,
    default: false,
};
const SHOW_GOAL: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: SHOW_GOAL_OFFSET,
    default: false,
};
const AI_INFO_NTH: MgrSetting<bool> = MgrSetting {
    header: "AI",
    key: "",
    address: ZTAIMGR_ADDRESS_PTR,
    offset: AI_INFO_NTH_OFFSET,
    default: false,
};
