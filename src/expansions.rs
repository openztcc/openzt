use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fmt,
    fmt::Display,
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use anyhow::Context;
use bf_configparser::ini::Ini;
use maplit::hashset;
use once_cell::sync::Lazy;
use retour_utils::hook_module;
use tracing::{error, info};
use zip::read::ZipFile;

use crate::{
    add_to_command_register,
    debug_dll::{
        get_from_memory, get_string_from_memory, get_string_from_memory_bounded, save_to_memory,
    },
    resource_manager::{
        add_handler, add_raw_bytes_to_map_with_path_override,
        add_txt_file_to_map_with_path_override, modify_ztfile_as_animation, modify_ztfile_as_ini,
        Handler,
    },
    string_registry::add_string_to_registry,
    ztui::{get_random_sex, get_selected_sex, BuyTab},
    ztworldmgr::{ZTEntityType, ZTEntityTypeClass},
};

static OFFICIAL_FILESET: Lazy<HashSet<&str>> = Lazy::new(|| {
    hashset! {"animals8.ztd",
        "awards5.ztd",
        "config5.ztd",
        "global03.ztd",
        "items5.ztd",
        "scenari5.ztd",
        "scenery5.ztd",
        "staff5.ztd",
        "ui5.ztd",
        "animals7.ztd",
        "scn40.ztd",
        "scn41.ztd",
        "scn42.ztd",
        "swordfsh.ztd",
        "swordfsh01.ztd",
        "ui7.ztd",
        "wilddog.ztd",
        "yeti.ztd",
        "animalsa.ztd",
        "antelope.ztd",
        "aqtheme1.ztd",
        "Asian Black Bear.ztd",
        "Asian Elephant.ztd",
        "asian_black_bear.ztd",
        "asian_elephant.ztd",
        "asian_elephant01.ztd",
        "asian_elephant02.ztd",
        "baracuda.ztd",
        "blackbuck.ztd",
        "bongo.ztd",
        "config4.ztd",
        "fancybld.ztd",
        "gallim.ztd",
        "gallim01.ztd",
        "global06.ztd",
        "guests7.ztd",
        "items7.ztd",
        "kidsdino.ztd",
        "komodo.ztd",
        "llama.ztd",
        "magnet.ztd",
        "magnet01.ztd",
        "magnet02.ztd",
        "megathrm.ztd",
        "mexipack.ztd",
        "mountainlion.ztd",
        "mountainlion01.ztd",
        "mt_lion.ztd",
        "na_theme.ztd",
        "object06.ztd",
        "plateo.ztd",
        "plateo01.ztd",
        "reindeer.ztd",
        "researm.ztd",
        "scenari7.ztd",
        "scenery7.ztd",
        "scn16.ztd",
        "scn17.ztd",
        "scn18.ztd",
        "ai.ztd",
        "ambient.ztd",
        "animals.ztd",
        "animals2.ztd",
        "awards.ztd",
        "config.ztd",
        "fences.ztd",
        "freeform.ztd",
        "fringe.ztd",
        "global.ztd",
        "guests.ztd",
        "items.ztd",
        "objects.ztd",
        "paths.ztd",
        "paths2.ztd",
        "research.ztd",
        "scenario.ztd",
        "scenery.ztd",
        "select.ztd",
        "sounds.ztd",
        "staff.ztd",
        "terrain.ztd",
        "tiles.ztd",
        "ui.ztd",
        "ztatb00.ztd",
        "ztatb0a.ztd",
        "ztatb0b.ztd",
        "ztatb0d.ztd",
        "ztatb01.ztd",
        "ztatb02.ztd",
        "ztatb03.ztd",
        "ztatb04.ztd",
        "ztatb05.ztd",
        "ztatb06.ztd",
        "ztatb07.ztd",
        "ztatb08.ztd",
        "ztatb09.ztd",
        "ztatb10.ztd",
        "zts.ztd",
    }
});

const CUSTOM_CONTENT_EXPANSION_STRING_PREFIX: &str = "openzt_";
const CUSTOM_CONTENT_EXPANSION_STRING_ALL: &str = "all";
const CUSTOM_CONTENT_EXPANSION_STRING_SUBDIR: &str = "subdir_";

const EXPANSION_LIST_START: u32 = 0x00639030;
const EXPANSION_SIZE: u32 = 0x14;
const EXPANSION_CURRENT: u32 = 0x00638d4c;

const MAX_EXPANSION_SIZE: usize = 14;

//TODO: Should point to openzt/openzt/expansion_dropdown/
const EXPANSION_ZT_RESOURCE_PREFIX: &str = "ui/sharedui/listbk/";
const EXPANSION_OPENZT_RESOURCE_PREFIX: &str = "openzt/openzt/expansion_dropdown/";
const EXPANSION_RESOURCE_ANI: &str = "listbk.ani";
const EXPANSION_RESOURCE_LYT: &str = "ui/xpac.lyt";
const EXPANSION_RESOURCE_PAL: &str = "listbk.pal";
const EXPANSION_RESOURCE_ANIMATION: &str = "N";

static MEMBER_SETS: Lazy<Mutex<HashMap<String, HashSet<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn add_member(entity_name: String, member: String) {
    let mut data_mutex = MEMBER_SETS.lock().unwrap();
    let set = data_mutex.entry(member).or_insert(HashSet::new());
    set.insert(entity_name);
}

pub fn is_member(entity_name: &str, member: &str) -> bool {
    let data_mutex = MEMBER_SETS.lock().unwrap();
    match data_mutex.get(member) {
        Some(set) => set.contains(entity_name),
        None => false,
    }
}

pub fn get_members(member: &str) -> Option<HashSet<String>> {
    let data_mutex = MEMBER_SETS.lock().unwrap();
    data_mutex.get(member).cloned()
}

// pub fn get_cc_members() -> Option<HashSet<String>> {
//     get_members(&(CUSTOM_CONTENT_EXPANSION_STRING_PREFIX.to_string() + CUSTOM_CONTENT_EXPANSION_STRING_ALL))
// }

fn get_cc_expansion_name_all() -> String {
    CUSTOM_CONTENT_EXPANSION_STRING_PREFIX.to_string() + CUSTOM_CONTENT_EXPANSION_STRING_ALL
}

fn get_cc_expansion_name(subdir: &str) -> String {
    CUSTOM_CONTENT_EXPANSION_STRING_PREFIX.to_string()
        + CUSTOM_CONTENT_EXPANSION_STRING_SUBDIR
        + subdir
}

fn command_get_members(_: Vec<&str>) -> Result<String, &'static str> {
    let data_mutex = MEMBER_SETS.lock().unwrap();
    let mut result = String::new();

    for (set_name, members) in data_mutex.iter() {
        let members_as_string: Vec<String> = members.iter().cloned().collect();
        result.push_str(&format!(
            "Set: {} -> Members: {}\n",
            set_name,
            members_as_string.join(", ")
        ));
    }

    Ok(result)
}

// There are no accessors for Expansions, ZT accesses expansions by directly iterating over the array, adding to the array also saves ptrs to ZT's memory keeping things in sync
static EXPANSION_ARRAY: Lazy<Mutex<Vec<Expansion>>> = Lazy::new(|| Mutex::new(Vec::new()));

fn add_expansion(expansion: Expansion, save_to_memory: bool) -> Result<(), &'static str> {
    let mut data_mutex = EXPANSION_ARRAY.lock().unwrap();
    if data_mutex.len() >= MAX_EXPANSION_SIZE {
        return Err("Max expansion size reached");
    }
    data_mutex.push(expansion);

    data_mutex.sort_by_key(|k| k.expansion_id);

    if save_to_memory {
        inner_save_mutex(data_mutex);
    }

    Ok(())
}

fn get_expansion(expansion_id: u32) -> Option<Expansion> {
    let data_mutex = EXPANSION_ARRAY.lock().unwrap();
    data_mutex
        .iter()
        .find(|expansion| expansion.expansion_id == expansion_id)
        .cloned()
}

fn expansion_number() -> usize {
    EXPANSION_ARRAY.lock().unwrap().len()
}

fn save_mutex() {
    inner_save_mutex(EXPANSION_ARRAY.lock().unwrap());
}

fn inner_save_mutex(mut mutex_guard: MutexGuard<Vec<Expansion>>) {
    let array_ptr = mutex_guard.as_mut_ptr();
    let array_end_ptr = unsafe { array_ptr.offset(mutex_guard.len() as isize) };
    let array_buffer_end_ptr = unsafe { array_ptr.offset(mutex_guard.capacity() as isize) };
    info!(
        "Saving expansions to {:#x} to {:#x}; {:#x}",
        array_ptr as u32, array_end_ptr as u32, array_buffer_end_ptr as u32
    );

    save_expansion_list_to_memory(ExpansionList {
        array_start: array_ptr as u32,
        array_end: array_end_ptr as u32,
        buffer_end: array_end_ptr as u32,
    });
}

fn get_expansions() -> Vec<Expansion> {
    EXPANSION_ARRAY.lock().unwrap().clone()
}

#[derive(Debug)]
#[repr(C)]
struct ExpansionList {
    array_start: u32,
    array_end: u32,
    buffer_end: u32,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Expansion {
    expansion_id: u32,
    name_id: u32,
    name_string_start_ptr: u32,
    name_string_end_ptr: u32,
    name_string_buffer_end_ptr: u32,
}

impl Expansion {
    fn name_string(&self) -> String {
        get_string_from_memory_bounded(
            self.name_string_start_ptr,
            self.name_string_end_ptr,
            self.name_string_buffer_end_ptr,
        )
    }
}

fn read_expansion_list_from_memory() -> ExpansionList {
    get_from_memory(EXPANSION_LIST_START)
}

fn read_expansion_from_memory(address: u32) -> Expansion {
    get_from_memory(address)
}

fn read_expansions_from_memory() -> Vec<Expansion> {
    let expansion_list = read_expansion_list_from_memory();
    info!(
        "Reading expansions from {:#x} to {:#x}, len {}",
        expansion_list.array_start,
        expansion_list.array_end,
        (expansion_list.array_end - expansion_list.array_start) / EXPANSION_SIZE
    );
    let mut expansions = Vec::new();
    let mut current_expansion_address = expansion_list.array_start;
    while current_expansion_address < expansion_list.array_end {
        expansions.push(read_expansion_from_memory(current_expansion_address));
        current_expansion_address += EXPANSION_SIZE;
    }
    expansions
}

fn read_current_expansion() -> Expansion {
    let current_expansion_id = get_from_memory(EXPANSION_CURRENT);
    get_expansion(current_expansion_id).unwrap()
}

fn save_current_expansion(expansion_id: u32) {
    save_to_memory(EXPANSION_CURRENT, expansion_id);
}

fn save_expansion_list_to_memory(expansion_list: ExpansionList) {
    save_to_memory(EXPANSION_LIST_START, expansion_list);
}

impl Display for Expansion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Expansion {{ expansion_id: {:#x} name_id: {:#x} name_string: {} }}",
            self.expansion_id,
            self.name_id,
            get_string_from_memory(self.name_string_start_ptr)
        )
    }
}

impl Display for ExpansionList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ExpansionList {{ array_start: {:#x} array_end: {:#x} buffer_end: {:#x} }}",
            self.array_start, self.array_end, self.buffer_end
        )
    }
}

fn command_get_expansions(_args: Vec<&str>) -> Result<String, &'static str> {
    let mut string_array = Vec::new();
    for expansion in read_expansions_from_memory() {
        string_array.push(expansion.to_string());
    }

    Ok(string_array.join("\n"))
}

fn command_get_current_expansion(_args: Vec<&str>) -> Result<String, &'static str> {
    Ok(read_current_expansion().to_string())
}

#[hook_module("zoo.exe")]
pub mod custom_expansion {
    use tracing::info;

    use super::{initialise_expansions, read_current_expansion};
    use crate::{
        debug_dll::get_from_memory, ztui::get_current_buy_tab,
        ztworldmgr::read_zt_entity_type_from_memory,
    };

    #[hook(unsafe extern "cdecl" ZTUI_general_entityTypeIsDisplayed, offset=0x000e8cc8)]
    pub fn ztui_general_entity_type_is_displayed(bf_entity: u32, param_1: u32, param_2: u32) -> u8 {
        // TODO: Put this call and subsequent log behind OpenZT debug flag
        let result =
            unsafe { ZTUI_general_entityTypeIsDisplayed.call(bf_entity, param_1, param_2) };

        let current_expansion = read_current_expansion();

        let entity = read_zt_entity_type_from_memory(bf_entity);

        let Some(current_buy_tab) = get_current_buy_tab() else {
            return 0;
        };

        let reimplemented_result =
            match super::filter_entity_type(&current_buy_tab, &current_expansion, &entity) {
                true => 1,
                false => 0,
            };

        if result != reimplemented_result {
            info!(
                "Filtering mismatch {} {} ({:#x} vs {:#x})",
                entity, current_buy_tab, result, reimplemented_result
            );
        }

        reimplemented_result
    }

    #[hook(unsafe extern "stdcall" ZTUI_expansionselect_setup, offset=0x001291fb)]
    pub fn ztui_expansionselect_setup() {
        unsafe { ZTUI_expansionselect_setup.call() }; //TODO: Remove this call once all functionality has been replicated

        initialise_expansions();
    }
}

fn initialise_expansions() {
    add_expansion_with_string_id(0x0, "all".to_string(), 0x5974, false);
    if let Some(member_hash) = get_members(&get_cc_expansion_name_all())
        && member_hash.len() > 0
    {
        add_expansion_with_string_value(
            0x4000,
            get_cc_expansion_name_all(),
            "Custom Content".to_string(),
            true,
        );

        save_mutex();
    }

    let number_of_expansions = get_expansions().len();

    if number_of_expansions > 4 {
        resize_expansion_dropdown(number_of_expansions as u32);
    }

    save_current_expansion(0x0);
}

fn resize_expansion_dropdown(number_of_expansions: u32) {
    let number_of_additional_expansions = number_of_expansions as i32 - 4;
    info!(
        "Resizing expansion dropdown to fit {} extra expansions",
        number_of_additional_expansions
    );
    if let Err(err) = modify_ztfile_as_ini(EXPANSION_RESOURCE_LYT, |cfg| {
        let old_y = cfg.get_parse::<i32>("list", "dy").unwrap().unwrap();
        let new_y = old_y + (number_of_additional_expansions * 30);
        cfg.set("list", "dy", Some(new_y.to_string()));
        info!("Resized expansion dropdown to {}", new_y);
    }) {
        info!("Error resizing expansion dropdown 'ani' file: {}", err);
    }

    if let Err(err) = modify_ztfile_as_ini(
        &(EXPANSION_OPENZT_RESOURCE_PREFIX.to_string() + EXPANSION_RESOURCE_ANI),
        |cfg| {
            let old_y0 = cfg.get_parse::<i32>("animation", "y0").unwrap().unwrap();
            let new_y0 = old_y0 - (number_of_additional_expansions * 10);
            let old_y1 = cfg.get_parse::<i32>("animation", "y1").unwrap().unwrap();
            let new_y1 = old_y1 + (number_of_additional_expansions * 10);
            cfg.set("animation", "y0", Some(new_y0.to_string()));
            cfg.set("animation", "y1", Some(new_y1.to_string()));
            cfg.set("animation", "dir0", Some("openzt".to_string()));
            cfg.set("animation", "dir1", Some("openzt".to_string()));
            cfg.set("animation", "dir2", Some("expansion_dropdown".to_string()));
            info!("Resized expansion dropdown to {} {}", new_y0, new_y1);
        },
    ) {
        info!("Error resizing expansion dropdown 'ani' file: {}", err);
    }
    info!("Check");
    match modify_ztfile_as_animation(
        &(EXPANSION_OPENZT_RESOURCE_PREFIX.to_string() + EXPANSION_RESOURCE_ANIMATION),
        |animation| {
            info!(
                "Reading expansion dropdown animation {} {} {}",
                animation.num_frames,
                animation.frames[0].lines.len(),
                animation.frames[0].vertical_offset_y
            );
            info!(
                "Animation points to pal file: {}",
                animation.pallette_filename
            );
            animation.frames[0].vertical_offset_y += number_of_additional_expansions as u16 * 10;
            for _ in 0..number_of_additional_expansions {
                animation.duplicate_pixel_rows(0, 10, 31).unwrap();
            }
            animation.set_pallette_filename(
                EXPANSION_OPENZT_RESOURCE_PREFIX.to_string() + EXPANSION_RESOURCE_PAL,
            );
            info!(
                "Resized expansion dropdown animation to {} {}",
                animation.frames[0].lines.len(),
                animation.frames[0].vertical_offset_y
            );
            info!(
                "Animation pal file changed to: {}",
                animation.pallette_filename
            );
        },
    ) {
        Ok(_) => info!("Resized expansion dropdown animation"),
        Err(e) => info!("Error resizing expansion dropdown animation: {}", e),
    }
}

fn filter_entity_type(
    buy_tab: &BuyTab,
    current_expansion: &Expansion,
    entity: &ZTEntityType,
) -> bool {
    match buy_tab {
        BuyTab::AnimalTab => {
            if !entity.is_member("animals".to_string()) {
                return false;
            }
            match get_selected_sex() {
                Some(sex) => {
                    if &sex.to_string() != entity.zt_sub_type() {
                        return false;
                    }
                }
                None => return false,
            }
        }
        BuyTab::ShelterTab => {
            if !entity.is_member("shelters".to_string()) {
                return false;
            }
        }
        BuyTab::ToysTab => {
            if !entity.is_member("toys".to_string()) {
                return false;
            }
        }
        BuyTab::ShowToysTab => {
            if !entity.is_member("showtoys".to_string()) {
                return false;
            }
        }
        BuyTab::BuildingTab => {
            if !entity.is_member("structures".to_string()) {
                return false;
            }
        }
        BuyTab::SceneryTab => {
            if !entity.is_member("scenery".to_string()) {
                return false;
            }
            // TODO: Make member name a combination of name and class so name double-ups don't cause this issue
            if entity.class() == &ZTEntityTypeClass::Scenery
                && entity.zt_type() == "other"
                && entity.zt_sub_type() == "fountain"
            {
                return false;
            }
        }
        BuyTab::FenceTab => {
            if !entity.is_member("fence".to_string()) {
                return false;
            }
            if entity.zt_sub_type() == "g" {
                return false;
            }
        }
        BuyTab::PathTab => {
            if !entity.is_member("paths".to_string()) {
                return false;
            }
        }
        BuyTab::FoliageTab => {
            if !entity.is_member("foliage".to_string()) {
                return false;
            }
        }
        BuyTab::RocksTab => {
            if !entity.is_member("rocks".to_string()) {
                return false;
            }
        }
        BuyTab::StaffTab => {
            if !entity.is_member("staff".to_string()) {
                return false;
            }
            if (matches!(entity.zt_sub_type().as_str(), "m" | "f")
                && entity.zt_sub_type() != &get_random_sex().unwrap().to_string())
            {
                return false;
            }
        }
        BuyTab::DeveloperTab => {
            if !entity.is_member("developer".to_string()) {
                return false;
            }
        }
        BuyTab::PaintTerrainTab | BuyTab::TerraformTab => return false,
    }

    if buy_tab != &BuyTab::PathTab {
        if current_expansion.expansion_id == 0x1 {
            for expansion in get_expansions() {
                if expansion.expansion_id > 0x1
                    && entity.is_member(expansion.name_string())
                    && !entity.is_member("zoo".to_string())
                {
                    return false;
                }
            }
        }
        if current_expansion.expansion_id > 0x1
            && !entity.is_member(current_expansion.name_string())
        {
            return false;
        }
    }

    return true;
}

fn add_expansion_with_string_id(id: u32, name: String, string_id: u32, save_to_memory: bool) {
    // TODO: get len first to avoid needing to clone?
    let name_len = name.len();
    let name_ptr = CString::new(name).unwrap().into_raw() as u32;
    let name_ptr_end = name_ptr + name_len as u32 + 1;
    if let Err(err) = add_expansion(
        Expansion {
            expansion_id: id,
            name_id: string_id,
            name_string_start_ptr: name_ptr,
            name_string_end_ptr: name_ptr_end,
            name_string_buffer_end_ptr: name_ptr_end,
        },
        save_to_memory,
    ) {
        error!("Error adding expansion: {}", err);
    }
}

fn add_expansion_with_string_value(
    id: u32,
    name: String,
    string_value: String,
    save_to_memory: bool,
) {
    let name_len = name.len();
    let name_ptr = CString::new(name).unwrap().into_raw() as u32;
    let name_ptr_end = name_ptr + name_len as u32 + 1;
    let name_id = add_string_to_registry(string_value);
    if let Err(err) = add_expansion(
        Expansion {
            expansion_id: id,
            name_id: name_id,
            name_string_start_ptr: name_ptr,
            name_string_end_ptr: name_ptr_end,
            name_string_buffer_end_ptr: name_ptr_end,
        },
        save_to_memory,
    ) {
        error!("Error adding expansion: {}", err);
    }
}

fn handle_expansion_config(path: &PathBuf, file: &mut ZipFile) {
    if let Err(e) = parse_expansion_config(file) {
        info!(
            "Error parsing expansion config: {} {} {}",
            path.display(),
            file.name(),
            e
        )
    }
}

fn handle_member_parsing(path: &PathBuf, file: &mut ZipFile) {
    if let Err(e) = parse_member_config(path, file) {
        error!(
            "Error parsing member config: {} {} {}",
            path.display(),
            file.name(),
            e
        )
    }
}

static FILE_NAME_OVERRIDES: Lazy<HashMap<String, String>> = Lazy::new(|| {
    vec![
        (
            "fences/tankwall.ai".to_string(),
            "fences/tankwal1.ai".to_string(),
        ), // Assumed spelling mistake
        (
            "fences/hedge.ai".to_string(),
            "fences/not_hedge.ai".to_string(),
        ), // Duplicates, this one isn't loaded
        // TODO: Below might not be needed?
        (
            "scenery/other/fountain.ai".to_string(),
            "scenery/other/other_fountain.ai".to_string(),
        ), // Duplicates, this one isn't loaded
    ]
    .into_iter()
    .collect()
});

fn parse_member_config(path: &PathBuf, file: &mut ZipFile) -> anyhow::Result<()> {
    let mut buffer = vec![0; file.size() as usize];
    if let Err(error) = file.read(&mut buffer[..]) {
        info!("Error reading member config {}: {}", file.name(), error);
        return Ok(());
    }
    let string_buffer = String::from_utf8_lossy(&buffer[..]).to_string(); //TODO: Investigate parsing ANSI files

    let mut member_cfg = Ini::new();
    member_cfg.set_comment_symbols(&[';', '#', ':']);
    member_cfg.read(string_buffer).map_err(anyhow::Error::msg)?;

    let filepath = match FILE_NAME_OVERRIDES.contains_key(file.name()) {
        true => FILE_NAME_OVERRIDES.get(file.name()).unwrap().to_string(),
        false => file.name().to_ascii_lowercase(),
    };

    let filename = Path::new(&filepath)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    if let Some(keys) = member_cfg.get_keys("member") {
        for key in keys {
            add_member(filename.clone(), key);
        }
    }

    if is_cc(path) {
        add_member(
            filename,
            CUSTOM_CONTENT_EXPANSION_STRING_PREFIX.to_string()
                + CUSTOM_CONTENT_EXPANSION_STRING_ALL,
        );
    }

    Ok(())
}

fn is_cc(path: &PathBuf) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };

    match parent
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "zupdate" | "xpack1" | "zupdate1" | "xpack2" => return false,
        "dlupdate" | "dupdate" | "updates" | "" => {
            match path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
            {
                "" => return false,
                file_name => return !OFFICIAL_FILESET.contains(file_name),
            }
        }
        _ => return true,
    }
}

fn parse_expansion_config(file: &mut ZipFile) -> anyhow::Result<()> {
    let mut string_buffer = String::with_capacity(file.size() as usize);
    file.read_to_string(&mut string_buffer)?;
    info!("{}", string_buffer);

    let mut expansion_cfg = Ini::new();
    expansion_cfg
        .read(string_buffer)
        .map_err(anyhow::Error::msg)?;

    // TODO: bf-configparser should return a custom error so we can use ? rather than map_err
    let mut id: u32 = expansion_cfg
        .get_parse("expansion", "id")
        .map_err(anyhow::Error::msg)?
        .context("No id found in expansion config")?;
    id += 1;
    let name = expansion_cfg
        .get("expansion", "name")
        .context("No name found in expansion config")?
        .to_ascii_lowercase();
    let name_ptr = CString::new(name.clone()).unwrap().into_raw() as u32;
    let listid: u32 = expansion_cfg
        .get_parse("expansion", "listid")
        .map_err(anyhow::Error::msg)?
        .context("No listid found in expansion config")?;

    info!("Adding expansion: {}", name);
    add_expansion(
        Expansion {
            expansion_id: id,
            name_id: listid,
            name_string_start_ptr: name_ptr,
            name_string_end_ptr: name_ptr + name.len() as u32 + 1,
            name_string_buffer_end_ptr: name_ptr + name.len() as u32 + 1,
        },
        false,
    )
    .map_err(anyhow::Error::msg)?;

    Ok(())
}

fn handle_expansion_dropdown(entry: &PathBuf, file: &mut ZipFile) {
    let file_name = file.enclosed_name().unwrap().file_name().unwrap();
    let file_path = Path::new(EXPANSION_OPENZT_RESOURCE_PREFIX).join(file_name);
    let Ok(file_path_string) = file_path.clone().into_os_string().into_string() else {
        error!("Error converting file path to string");
        return;
    };
    info!("Handling expansion dropdown: {}", file_path_string);
    match Path::new(&file_path_string)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
    {
        "ani" => {
            info!("Handling ani file");
            add_txt_file_to_map_with_path_override(entry, file, file_path_string);
        }
        "pal" | "" => {
            info!("Handling pal file");
            add_raw_bytes_to_map_with_path_override(entry, file, file_path_string);
        }
        _ => return,
    }
}

pub fn init() {
    add_to_command_register("list_expansion".to_string(), command_get_expansions);
    add_to_command_register(
        "get_current_expansion".to_string(),
        command_get_current_expansion,
    );
    add_to_command_register("get_members".to_string(), command_get_members);
    add_handler(
        Handler::new(
            Some("xpac".to_string()),
            Some("cfg".to_string()),
            handle_expansion_config,
        )
        .unwrap(),
    );
    add_handler(Handler::new(None, Some("uca".to_string()), handle_member_parsing).unwrap());
    add_handler(Handler::new(None, Some("ucs".to_string()), handle_member_parsing).unwrap());
    add_handler(Handler::new(None, Some("ucb".to_string()), handle_member_parsing).unwrap());
    add_handler(Handler::new(None, Some("ai".to_string()), handle_member_parsing).unwrap());
    add_handler(
        Handler::new(
            Some(EXPANSION_ZT_RESOURCE_PREFIX.to_string()),
            None,
            handle_expansion_dropdown,
        )
        .unwrap(),
    );
    unsafe { custom_expansion::init_detours().unwrap() };
}
