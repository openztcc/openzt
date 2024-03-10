use std::fmt;

use tracing::info;

use crate::console::add_to_command_register;

use crate::ztworldmgr::read_zt_entity_from_memory;

use crate::debug_dll::{get_from_memory, get_string_from_memory_bounded};

use crate::common::ZTString;

const BFUIMGR_PTR: u32 = 0x00638de0;

#[derive(Debug)]
pub enum UIElementId {
    AnimalScrollingRegion = 2019,
    AnimalTab = 2075,
    ShelterTab = 3255,
    ToysTab = 3254,
    ShowToysTab = 2076,
    BuyObjectScrollingRegion = 3019,
    BuildingTab = 3050,
    SceneryTab = 3053,
    BuildHabitatScrollingRegion = 3208,
    FenceTab = 3251,
    PathTab = 3056,
    FoliageTab = 3252,
    RocksTab = 3256,
    TerraformScrollingRegion = 3350,
    PaintTerrainTab = 3362,
    TerraformTab = 3361,
    StaffScrollingRegion = 3602,
    MaleButton = 2000,
    FemaleButton = 2001,
    DeveloperScrollingRegion = 5003,
}

#[derive(Debug, PartialEq)]
pub enum BuyTab {
    AnimalTab,
    ShelterTab,
    ToysTab,
    ShowToysTab,
    BuildingTab,
    SceneryTab,
    FenceTab,
    PathTab,
    FoliageTab,
    RocksTab,
    PaintTerrainTab,
    TerraformTab,
    StaffTab,
    DeveloperTab,
}

//TODO: Add support for the buy young hack
pub enum Sex {
    Male,
    Female,
}

impl fmt::Display for Sex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let char = match self {
            Sex::Male => 'm',
            Sex::Female => 'f',
        };
        write!(f, "{}", char)
    }
}

const RANDOM_SEX_STRING_PTR: u32 = 0x0063e420;

pub fn init() {
    add_to_command_register("get_selected_entity".to_owned(), command_get_selected_entity);
    add_to_command_register("get_element".to_owned(), command_get_element);
}

fn command_get_selected_entity(_args: Vec<&str>) -> Result<String, &'static str> {
    let get_selected_entity_fn: fn() -> u32 = unsafe { std::mem::transmute(0x00410f84) }; //TODO: Move type to variable declaration rather than turbofish
    let entity_address = get_selected_entity_fn();
    if entity_address == 0 {
        return Ok("No entity selected".to_string());
    }
    let entity = read_zt_entity_from_memory(entity_address);
    Ok(format!("{:#?}", entity))
}

fn command_get_element(args: Vec<&str>) -> Result<String, &'static str> {
    if args.len() != 1 {
        return Err("Expected 1 argument");
    }
    let address = args[0].parse::<u32>().unwrap();
    let get_element_fn: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(0x0040157d) };
    let ui_element_addr = get_element_fn(BFUIMGR_PTR, address);
    if ui_element_addr == 0 {
        return Err("No element found");
    }
    let element: UIElement = get_from_memory(ui_element_addr);
    info!("{:#x} {:#x}", address, ui_element_addr);
    Ok(format!("{}", element))
}

fn get_element(id: UIElementId) -> Option<UIElement> {
    let get_element_fn: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(0x0040157d) };
    let ui_element_addr = get_element_fn(BFUIMGR_PTR, id as u32);
    if ui_element_addr == 0 {
        return None;
    }
    Some(get_from_memory(ui_element_addr))
}

fn command_get_current_buy_tab(_args: Vec<&str>) -> Result<String, &'static str> {
    let tab = get_current_buy_tab();
    Ok(format!("{:?}", tab))
}

pub fn get_current_buy_tab() -> Option<BuyTab> {
    if let Some(asr) = get_element(UIElementId::AnimalScrollingRegion) && !asr.state.is_hidden() {
        if get_element(UIElementId::AnimalTab)?.state.is_selected() {
            return Some(BuyTab::AnimalTab);
        }
        if get_element(UIElementId::ShelterTab)?.state.is_selected() {
            return Some(BuyTab::ShelterTab);
        }
        if get_element(UIElementId::ToysTab)?.state.is_selected() {
            return Some(BuyTab::ToysTab);
        }
        if get_element(UIElementId::ShowToysTab)?.state.is_selected() {
            return Some(BuyTab::ShowToysTab);
        }
    }
    if let Some(osr) = get_element(UIElementId::BuyObjectScrollingRegion) && !osr.state.is_hidden() {
        if get_element(UIElementId::BuildingTab)?.state.is_selected() {
            return Some(BuyTab::BuildingTab);
        }
        if get_element(UIElementId::SceneryTab)?.state.is_selected() {
            return Some(BuyTab::SceneryTab);
        }
    }
    if let Some(hsr) = get_element(UIElementId::BuildHabitatScrollingRegion) && !hsr.state.is_hidden() {
        if get_element(UIElementId::FenceTab)?.state.is_selected() {
            return Some(BuyTab::FenceTab);
        }
        if get_element(UIElementId::PathTab)?.state.is_selected() {
            return Some(BuyTab::PathTab);
        }
        if get_element(UIElementId::FoliageTab)?.state.is_selected() {
            return Some(BuyTab::FoliageTab);
        }
        if get_element(UIElementId::RocksTab)?.state.is_selected() {
            return Some(BuyTab::RocksTab);
        }
    }
    if let Some(tsr) = get_element(UIElementId::TerraformScrollingRegion) && !tsr.state.is_hidden() {
        if get_element(UIElementId::PaintTerrainTab)?.state.is_selected() {
            return Some(BuyTab::PaintTerrainTab);
        }
        if get_element(UIElementId::TerraformTab)?.state.is_selected() {
            return Some(BuyTab::TerraformTab);
        }
    }
    if let Some(ssr) = get_element(UIElementId::StaffScrollingRegion) && !ssr.state.is_hidden() {
        return Some(BuyTab::StaffTab);
    }
    if let Some(developer_tab) = get_element(UIElementId::DeveloperScrollingRegion) && !developer_tab.state.is_hidden() {
        return Some(BuyTab::DeveloperTab);
    }
    None
}

pub fn get_selected_sex() -> Option<Sex> {
    if get_element(UIElementId::MaleButton)?.state.is_selected() {
        return Some(Sex::Male);
    }
    if get_element(UIElementId::FemaleButton)?.state.is_selected() {
        return Some(Sex::Female);
    }
    None
}

pub fn get_random_sex() -> Option<Sex> {
    let string_address = get_from_memory::<u32>(RANDOM_SEX_STRING_PTR);
    match get_string_from_memory_bounded(string_address, string_address + 4, string_address + 8).as_str() {
        "m" => Some(Sex::Male),
        "f" => Some(Sex::Female),
        _ => None
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct UIElement {
    vftable: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_string_1: ZTString,
    string_content: ZTString,
    element_name: ZTString,
    // 25 unknown u32s
    padding: [u8; 76],
    state: UIState,
}

impl fmt::Display for UIElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UIElement {{ unknown_u32_1: {:#x}, unknown_u32_2: {:#x}, unknown_string_1: {}, string_content: {}, element_name: {}, state: {} }}",
               self.unknown_u32_1, self.unknown_u32_2, self.unknown_string_1.to_string(), self.string_content.to_string(), self.element_name.to_string(), self.state)
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct UIState {
    state: u16
}

impl UIState {
    fn is_hidden(&self) -> bool {
        0b1 & self.state != 0
    }
    fn is_disabled(&self) -> bool {
        0b10 & self.state != 0
    }
    fn is_highlighted(&self) -> bool {
        0b100 & self.state != 0
    }
    fn is_selected(&self) -> bool {
        0b1000 & self.state != 0
    }
    fn is_extra_hidden(&self) -> bool {
        0b1_0000 & self.state != 0
    }
    fn is_focused(&self) -> bool {
        0b10_0000_0000 & self.state != 0
    }
}

impl fmt::Display for UIState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UIState {{ hidden: {}, disabled: {}, highlighted: {}, selected: {}, focused: {} }}",
               self.is_hidden(), self.is_disabled(), self.is_highlighted(), self.is_selected(), self.is_focused())
    }
}
