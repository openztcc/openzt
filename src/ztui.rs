use tracing::info;

use crate::console::add_to_command_register;
use crate::ztgamemgr::ZTGameMgr;
use crate::ztworldmgr::read_zt_entity_from_memory;

mod ZTUI {
    pub struct gameopts {

    }

    pub struct main {
        pub setMoneyText: fn(),
    }
}

void ZTGameMgr::ZTUIMainSetMoneyText() {
    // set money text show in the UI
    float money = this->zoo_budget;
    DWORD uVar1; // local variable

    // void* pBFUIMgr = *(void**)0x00638de0;
    BFUIMgr *pBFUIMgr = reinterpret_cast<BFUIMgr*>(0x00638de0);

    // GXRGB color = {0, 0, 0}; // set color to black

    // float money_to_display = (float)((int)money); // round down to nearest integer
    BFUIMgrSetControlForeColor(pBFUIMgr, 0x3f8, 0xf44bda); // set control forecolor   
    BFInternatSetMoneyText(0x3f8, (int)(this->zoo_budget), '\x01'); // set money text
}
DWORD bfuimgr = *(DWORD*)((LPVOID)0x00638de0);

impl ZTUI {
    pub fn set_money_text() {
        let money = ZTGameMgr.zoo_budget;
        let pbfuimgr: u32 = 0x00638de0;
        let set_money_text_fn = unsafe { std::mem::transmute::<u32, fn() -> u32>(0x00410f84) };
        set_money_text_fn();
    }
}

pub fn init() {
    add_to_command_register("get_selected_entity".to_owned(), command_get_selected_entity);
}

fn command_get_selected_entity(_args: Vec<&str>) -> Result<String, &'static str> {
    let get_selected_entity_fn = unsafe { std::mem::transmute::<u32, fn() -> u32>(0x00410f84) }; //TODO: Move type to variable declaration rather than turbofish
    let entity_address = get_selected_entity_fn();
    if entity_address == 0 {
        return Ok("No entity selected".to_string());
    }
    let entity = read_zt_entity_from_memory(entity_address);
    Ok(format!("{:#?}", entity))
}


