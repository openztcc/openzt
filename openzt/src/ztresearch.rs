//! Structs and methods for the vanilla `ZTResearchMgr` / `ZTResearchBranch` / `ZTResearchCategory` /
//! `ZTResearchProgram` classes, which drive the zoo's research tree: picking which program a branch
//! (e.g. "Animal Care") is currently working towards, tracking funding and progress, and applying a
//! program's effect once it completes.
//!
//! Field layouts below are derived from the decompiles in `resources/decompiles/ZTResearch*` and
//! `resources/decompiles/_forceResearch.c` / `_clickResearch.c`. Byte ranges with no decompiled
//! evidence are left as `padN` placeholders. `ZTResearchMgr`'s layout is additionally confirmed by
//! `openzt-detour/src/structs.rs` (size `0x18`).
//!
//! Traversal/read-only calculations (`get_branch`/`get_category`/`get_program`, funding level
//! selection, `pct_remaining_on_program`/`days_remaining_on_program`, `set_effect_discount`) are
//! reimplemented natively in Rust since their vanilla logic is simple and fully understood. Functions
//! with complex or only
//! partially understood side effects (completion dispatch, save/load, RNG-based program selection)
//! call into the original game code via the addresses already registered in
//! `openzt-detour/src/generated.rs` rather than risk a subtly wrong reimplementation.

use std::{
    ffi::{c_char, CStr},
    fmt,
    mem::size_of,
};

use num_enum::TryFromPrimitive;
use openzt_detour::generated::{standalone, ztresearchbranch, ztresearchcategory, ztresearchmgr, ztresearchprogram};
use tracing::{error, info, warn};

use crate::{
    bfconfigfile::BFConfigFile,
    command_console::CommandError,
    globals::{get_module_base, globals},
    lua_fn,
    string_registry::load_string_by_id,
    util::{get_from_memory, mut_from_memory, ref_from_memory, save_to_memory, ZTArray, ZTBufferString, ZTString},
};

/// Upper bounds used purely to stop `command_list_research` from looping forever/crashing on a
/// garbage count if the global address or a pointer chain turns out to be wrong; real vanilla data
/// is well under these.
const MAX_REASONABLE_BRANCHES: usize = 32;
const MAX_REASONABLE_CATEGORIES: usize = 128;
const MAX_REASONABLE_PROGRAMS: usize = 512;

/// The kind of effect a `ZTResearchProgram` applies once it completes, dispatched by the vanilla
/// `ZTResearchProgram::onCompletion` switch statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum ZTResearchEffectKind {
    /// Calls `setAvail` (unlocks a building/scenery entity) then reports completion directly.
    UnlockEntity = 0,
    BuildingUpgrade = 1,
    EntityCharacteristic = 2,
    GenusCharacteristic = 3,
    FamilyCharacteristic = 4,
    FoodCharacteristic = 5,
    TrickAvailable = 6,
    /// Applies a percentage discount to matching programs' `target_cost`; see `ZTResearchMgr::set_effect_discount`.
    EffectDiscount = 7,
}

/// One entry in a `ZTResearchBranch`'s inline funding-level table (`funding_table_start..funding_table_end`,
/// stride `0xc`, *not* a `ZTArray` of pointers). Loaded from one of the named sub-blocks a branch's
/// `.cfg` `funding=` list references (e.g. `funding=normal` -> a `[normal]` block with its own
/// `name`/`cost`/`work` keys). Selecting a higher index via
/// `ZTResearchBranch::increase_funding`/`decrease_funding` changes the `rate` used when computing
/// research progress.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ZTResearchFundingLevel {
    name_id: i32, // 0x0 - confirmed: the sub-block's `name` string id (e.g. 23100 = "%s none", 23101 = "%s min", 23102 = "%s normal", 23103 = "%s max")
    rate: f32,    // 0x4 - confirmed: the sub-block's `work` value (e.g. none=0, min=21, normal=30, max=45); used as a divisor by the vanilla days/pct-remaining calculations, must be > 0 for this level to be considered active
    cost: f32,    // 0x8 - confirmed: the sub-block's `cost` value (e.g. none=0, min=400, normal=1000, max=2000)
}

impl ZTResearchFundingLevel {
    pub fn name_id(&self) -> i32 {
        self.name_id
    }

    /// Display name, resolved through the same string table `get_string()` uses (e.g. "%s none").
    pub fn name(&self) -> Option<String> {
        load_string_by_id(self.name_id as u32)
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn cost(&self) -> f32 {
        self.cost
    }
}

/// A single research program, e.g. "Improved Elephant Enclosure". Owned (by pointer) by a
/// `ZTResearchCategory`'s `program_array`. Loaded by `ZTResearchProgram::loadProgram`, which treats
/// `this` directly as a `BFConfigFile*` (i.e. `ZTResearchProgram` inherits `BFConfigFile`, which
/// occupies the first `0xc` bytes) and reads a `.cfg` block of the form
/// `name`/`desc`/`icon`/`entityIcon`/`cost`/`order`/`target`/`effect`/`effectval1`/`effectval2`/
/// `effectval3`/`helpid`.
#[derive(Debug)]
#[repr(C)]
pub struct ZTResearchProgram {
    config_file: BFConfigFile,   // 0x00 - the inherited `BFConfigFile` base (see `bfconfigfile.rs`); `kind_tag` observed to always be 6 for research programs
    cached_name: ZTBufferString, // 0x0c - confirmed: built via `BFApp::buildString` from `id` (`name`) right after it's loaded
    cached_desc: ZTBufferString, // 0x18 - confirmed: built via `BFApp::buildString` from `desc_id` (`desc`) right after it's loaded
    desc_id: i32,                // 0x24 - confirmed: the `.cfg` `desc` string id, resolves via `get_string()`
    icon_ptr: u32,               // 0x28 - confirmed: the `.cfg` `icon` field, a raw C string pointer (only valid when non-null)
    entity_icon_ptr: u32,        // 0x2c - confirmed: the `.cfg` `entityIcon` field, a raw C string pointer; also the value `onCompletion` passes to `setBuildingUpgrade` as the building type name when `effect_kind == BuildingUpgrade`
    id: i32,                     // 0x30 - confirmed: the `.cfg` `name` string id, matched by `ZTResearchMgr::get_program`
    target_cost: f32,            // 0x34 - confirmed: the `.cfg` `cost` field
    current_progress: f32,       // 0x38 - not set by `loadProgram` (starts at 0 from the constructor); funding accumulated so far, complete once current_progress >= target_cost
    priority: u32,               // 0x3c - confirmed: the `.cfg` `order` field; tie-breaker used by `ZTResearchBranch::pick_random_program` when choosing the next not-yet-started program
    target_id: i32,              // 0x40 - confirmed: the `.cfg` `target` field; sentinel -1 when unset
    effect_kind_raw: i32,        // 0x44 - confirmed: the `.cfg` `effect` field; see `ZTResearchEffectKind`; sentinel -1 when unset; dispatches `ZTResearchProgram::on_completion`
    effect_param_0: i32,         // 0x48 - confirmed: the `.cfg` `effectval1` field
    effect_param_1: i32,         // 0x4c - confirmed: the `.cfg` `effectval2` field
    effect_param_2: i32,         // 0x50 - confirmed: the `.cfg` `effectval3` field
    help_id: i32,                // 0x54 - confirmed: the `.cfg` `helpid` field (only set if present; 0 by default from the constructor)
}

impl ZTResearchProgram {
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Display name, resolved through the same string table `get_string()` uses. Confirmed against
    /// the live game and against `ZTResearchProgram::loadProgram` (built from `id`/`name` via
    /// `BFApp::buildString` into `cached_name`).
    pub fn name(&self) -> Option<String> {
        load_string_by_id(self.id as u32)
    }

    /// The load-time cached copy of `name()`'s text (see `loadProgram`), rather than a fresh
    /// `get_string()` lookup.
    pub fn cached_name(&self) -> String {
        self.cached_name.copy_to_string()
    }

    /// The `.cfg` `desc` field's text, resolved through `get_string()`. Confirmed against the live
    /// game and against `loadProgram`.
    pub fn desc(&self) -> Option<String> {
        load_string_by_id(self.desc_id as u32)
    }

    /// The load-time cached copy of `desc()`'s text (see `loadProgram`), rather than a fresh
    /// `get_string()` lookup.
    pub fn cached_desc(&self) -> String {
        self.cached_desc.copy_to_string()
    }

    /// `BFConfigFile`'s "has data" flag; effectively always `true` for any program you can reach
    /// through `ZTResearchMgr`/`ZTResearchCategory`, since `loadProgram` bails out before finishing
    /// construction otherwise.
    pub fn is_config_loaded(&self) -> bool {
        self.config_file.is_loaded()
    }

    /// `BFConfigFile`'s internal "kind of config" tag; confirmed to always be `6` for research
    /// programs so far. Not useful for anything program-specific, just here for completeness.
    pub fn config_kind_tag(&self) -> u8 {
        self.config_file.kind_tag()
    }

    /// The `.cfg` `icon` field, if set.
    pub fn icon(&self) -> Option<String> {
        (self.icon_ptr != 0).then(|| unsafe { CStr::from_ptr(self.icon_ptr as *const c_char) }.to_string_lossy().into_owned())
    }

    /// The `.cfg` `entityIcon` field, if set. See `building_type_name` for its reuse as a building
    /// type name string when `effect_kind == BuildingUpgrade`.
    pub fn entity_icon(&self) -> Option<String> {
        (self.entity_icon_ptr != 0)
            .then(|| unsafe { CStr::from_ptr(self.entity_icon_ptr as *const c_char) }.to_string_lossy().into_owned())
    }

    /// The building type name passed to `setBuildingUpgrade` on completion - actually just
    /// `entity_icon()`, reused for this purpose. Only meaningful when
    /// `effect_kind() == Some(ZTResearchEffectKind::BuildingUpgrade)`; returns `None` otherwise to
    /// match what `onCompletion` itself does (every other effect kind ignores this field).
    pub fn building_type_name(&self) -> Option<String> {
        if self.effect_kind() != Some(ZTResearchEffectKind::BuildingUpgrade) {
            return None;
        }
        self.entity_icon()
    }

    pub fn target_cost(&self) -> f32 {
        self.target_cost
    }

    pub fn current_progress(&self) -> f32 {
        self.current_progress
    }

    pub fn is_complete(&self) -> bool {
        self.current_progress >= self.target_cost
    }

    pub fn priority(&self) -> u32 {
        self.priority
    }

    pub fn target_id(&self) -> i32 {
        self.target_id
    }

    pub fn effect_kind(&self) -> Option<ZTResearchEffectKind> {
        ZTResearchEffectKind::try_from(self.effect_kind_raw).ok()
    }

    pub fn effect_params(&self) -> (i32, i32, i32) {
        (self.effect_param_0, self.effect_param_1, self.effect_param_2)
    }

    /// The raw `.cfg` `helpid` field. Prefer this over `tooltip()` for on-screen UI tooltips - the
    /// game's help system has separate short/long text variants selected by a user setting (see
    /// `ui::tooltip::tooltip_text_from_id`'s `LONG_TOOLTIP_ID_OFFSET`), which only UI code knows how
    /// to apply; `tooltip()` below does a plain lookup that ignores that preference.
    pub fn help_id(&self) -> i32 {
        self.help_id
    }

    /// The `.cfg` `helpid` field's text, resolved via a direct (non-long/short-aware) string lookup.
    /// Fine for debug/console output; UI code displaying an on-screen tooltip should use `help_id()`
    /// with `ui::vanilla_main`'s tooltip helpers instead, so the long-tooltip user setting is honored.
    pub fn tooltip(&self) -> Option<String> {
        load_string_by_id(self.help_id as u32)
    }

    /// Calls the vanilla `ZTResearchProgram::onCompletion`, which dispatches on `effect_kind` into
    /// one of several other managers (building/entity/genus/family/food/trick/discount) and reports
    /// the completion to `ZTUI::zoostatus`. Left as a call into the original implementation since
    /// several of those downstream functions aren't otherwise reimplemented in OpenZT.
    pub fn on_completion(&mut self) -> u32 {
        unsafe { ztresearchprogram::ON_COMPLETION.original()((self as *mut Self) as *const u32) }
    }

    pub fn reset(&mut self) -> u32 {
        unsafe { ztresearchprogram::RESET.original()((self as *mut Self) as *const u32) }
    }

    pub fn load_program(&mut self, reader: *const u32) -> u32 {
        unsafe { ztresearchprogram::LOAD_PROGRAM.original()((self as *mut Self) as *const u32, reader) }
    }
}

impl fmt::Display for ZTResearchProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ZTResearchProgram {{")?;
        writeln!(f, "  id: {},", self.id)?;
        writeln!(f, "  name: {:?},", self.name())?;
        writeln!(f, "  desc: {:?},", self.desc())?;
        writeln!(f, "  icon: {:?},", self.icon())?;
        writeln!(f, "  entity_icon: {:?},", self.entity_icon())?;
        writeln!(f, "  building_type_name: {:?},", self.building_type_name())?;
        writeln!(f, "  target_cost: {},", self.target_cost)?;
        writeln!(f, "  current_progress: {},", self.current_progress)?;
        writeln!(f, "  priority: {},", self.priority)?;
        writeln!(f, "  target_id: {},", self.target_id)?;
        writeln!(f, "  effect_kind: {:?} ({}),", self.effect_kind(), self.effect_kind_raw)?;
        writeln!(f, "  effect_params: {:?},", self.effect_params())?;
        writeln!(f, "  tooltip: {:?},", self.tooltip())?;
        write!(f, "}}")
    }
}

/// A group of related research programs, e.g. "Elephants". Owned (by pointer) by a
/// `ZTResearchBranch`'s `category_array`. Loaded by `ZTResearchCategory::loadCategory` (which, like
/// `ZTResearchProgram`, treats `this` directly as a `BFConfigFile*`) from a `.cfg` block of the form
/// `name`/`desc`/`icon`/`helpid`/`expansion`, plus a `program=` list (-> `program_array`).
/// `ZTResearchBranch` shares an almost identical layout (see its own doc comment).
#[derive(Debug)]
#[repr(C)]
pub struct ZTResearchCategory {
    config_file: BFConfigFile,   // 0x00 - the inherited `BFConfigFile` base (see `bfconfigfile.rs`)
    id: i32,                     // 0x0c - confirmed by `ZTResearchCategory::loadCategory` (loaded from the `.cfg` `name` key); matched by `ZTResearchMgr::get_category`; doubles as the display-name string id (`get_string(id)` returns the category's name - this is what the in-game UI mislabels "Program")
    cached_name: ZTBufferString, // 0x10 - confirmed by `loadCategory`: built from `id`/`name` via `BFApp::buildString` right after it's loaded
    cached_desc: ZTBufferString, // 0x1c - confirmed by `loadCategory`: built from the `.cfg` `desc` field the same way (the raw `desc` string id is only ever a local variable in `loadCategory` - unlike `ZTResearchProgram`, it's never stored on the object itself)
    icon_ptr: u32,               // 0x28 - confirmed by `loadCategory`: the `.cfg` `icon` field, a raw C string pointer (only valid when non-null) - not a numeric id as previously guessed
    help_id: i32,                // 0x2c - confirmed: matches `.cfg` `helpid=` exactly (tested against `helpid=24213`) and matches `loadCategory`'s `getInt(..., "helpid", ...)` call storing here
    expansion_id: i32,           // 0x30 - confirmed by `loadCategory` (`.cfg` `expansion` key); +1 is passed to `ZTUI::expansionselect::isExpansionDisabled` when picking a program
    enabled: u8,                 // 0x34 - unlocked/available flag; gates `ZTResearchBranch::pick_random_program` and is persisted by `ZTResearchMgr::save`; not set by `loadCategory` itself
    pad2: [u8; 0x38 - 0x35],     // 0x35 - alignment padding
    program_array: ZTArray<ZTResearchProgram>, // 0x38
}

impl ZTResearchCategory {
    pub fn id(&self) -> i32 {
        self.id
    }

    /// See `ZTResearchProgram::is_config_loaded`.
    pub fn is_config_loaded(&self) -> bool {
        self.config_file.is_loaded()
    }

    /// Display name, resolved through the same string table `get_string()` uses. Confirmed against
    /// the live game.
    pub fn name(&self) -> Option<String> {
        load_string_by_id(self.id as u32)
    }

    /// The load-time cached copy of `name()`'s text (see `loadCategory`), rather than a fresh
    /// `get_string()` lookup.
    pub fn cached_name(&self) -> String {
        self.cached_name.copy_to_string()
    }

    /// The `.cfg` `desc` field's cached text, if any (empty when `desc=` was blank).
    pub fn desc(&self) -> String {
        self.cached_desc.copy_to_string()
    }

    /// The `.cfg` `icon` field, if set.
    pub fn icon(&self) -> Option<String> {
        (self.icon_ptr != 0).then(|| unsafe { CStr::from_ptr(self.icon_ptr as *const c_char) }.to_string_lossy().into_owned())
    }

    /// The raw `.cfg` `helpid` field. Prefer this over `tooltip()` for on-screen UI tooltips - see
    /// `ZTResearchProgram::help_id`'s doc comment for why.
    pub fn help_id(&self) -> i32 {
        self.help_id
    }

    /// The category's tooltip/help text, resolved from `help_id` through `get_string()`. Confirmed
    /// against the live game (`help_id` matches a known `.cfg` `helpid=` value exactly). Fine for
    /// debug/console output; UI tooltips should use `help_id()` instead (see `ZTResearchProgram::help_id`).
    pub fn tooltip(&self) -> Option<String> {
        load_string_by_id(self.help_id as u32)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }

    /// Sets the unlocked/available flag - toggled by the in-game research category checklist UI
    /// (a multi-select list, not a single "current category" picker), and read by
    /// `ZTResearchBranch::pick_random_program`/persisted by `ZTResearchMgr::save` like the rest of
    /// the category's state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled as u8;
    }

    pub fn expansion_id(&self) -> i32 {
        self.expansion_id
    }

    pub fn program_count(&self) -> usize {
        self.program_array.len()
    }

    pub fn program(&self, index: usize) -> &'static ZTResearchProgram {
        unsafe { ref_from_memory(self.program_array.get_ptr(index)) }
    }

    pub fn program_mut(&self, index: usize) -> &'static mut ZTResearchProgram {
        unsafe { mut_from_memory(self.program_array.get_ptr(index)) }
    }

    pub fn programs(&self) -> impl Iterator<Item = &'static ZTResearchProgram> + '_ {
        (0..self.program_count()).map(move |i| self.program(i))
    }

    pub fn programs_mut(&self) -> impl Iterator<Item = &'static mut ZTResearchProgram> + '_ {
        (0..self.program_count()).map(move |i| self.program_mut(i))
    }

    /// Calls the vanilla `ZTResearchCategory::loadCategory`; used while reading a mod/save's
    /// research definitions. `reader` is whatever stream/buffer pointer the original expects.
    pub fn load_category(&mut self, reader: *const i32) -> u32 {
        unsafe { ztresearchcategory::LOAD_CATEGORY.original()((self as *mut Self) as *const u32, reader) }
    }

    pub fn clear(&mut self) {
        unsafe { ztresearchcategory::CLEAR_CATEGORY.original()((self as *mut Self) as *const u32) }
    }
}

impl fmt::Display for ZTResearchCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ZTResearchCategory {{")?;
        writeln!(f, "  id: {},", self.id)?;
        writeln!(f, "  name: {:?},", self.name())?;
        writeln!(f, "  desc: {:?},", self.desc())?;
        writeln!(f, "  icon: {:?},", self.icon())?;
        writeln!(f, "  tooltip: {:?},", self.tooltip())?;
        writeln!(f, "  enabled: {},", self.is_enabled())?;
        writeln!(f, "  expansion_id: {},", self.expansion_id)?;
        writeln!(f, "  program_count: {},", self.program_count())?;
        write!(f, "}}")
    }
}

/// A top-level research branch, e.g. "Animal Care" or "Guest Amenities". Owned (by pointer) by
/// `ZTResearchMgr`'s `branch_array`. Loaded by `ZTResearchBranch::loadBranch` (which, like
/// `ZTResearchProgram`, treats `this` directly as a `BFConfigFile*`) from a `.cfg` block of the form
/// `name`/`desc`/`icon`/`noprogicon` (no `helpid`, no `expansion` - those are category-only), plus a
/// `category=` list (-> `category_array`) and a `funding=` list (-> the funding table, each entry
/// naming a sub-block with its own `name`/`cost`/`work` keys).
#[derive(Debug)]
#[repr(C)]
pub struct ZTResearchBranch {
    config_file: BFConfigFile,   // 0x00 - the inherited `BFConfigFile` base (see `bfconfigfile.rs`)
    id: i32,                     // 0x0c - confirmed by `loadBranch` (`.cfg` `name` key); matched by `ZTResearchMgr::get_branch`; doubles as the display-name string id (`get_string(id)` returns the branch's name, e.g. "Animal Care")
    cached_name: ZTBufferString, // 0x10 - confirmed by `loadBranch`: built from `id`/`name` via `BFApp::buildString` right after it's loaded
    cached_desc: ZTBufferString, // 0x1c - confirmed by `loadBranch`: built from the `.cfg` `desc` field the same way (the raw `desc` string id is never stored on the object itself, same as `ZTResearchCategory`)
    icon_ptr: u32,               // 0x28 - confirmed by `loadBranch`: the `.cfg` `icon` field, a raw C string pointer
    noprogicon_ptr: u32,         // 0x2c - confirmed by `loadBranch`: the `.cfg` `noprogicon` field (the icon shown when no program is being researched), a raw C string pointer
    current_category_ptr: u32,                   // 0x30 - selected category; cleared then reassigned by `pick_random_program`
    current_program_ptr: u32,                    // 0x34 - selected program; read directly by `days_remaining_on_program`/`pct_remaining_on_program`
    category_array: ZTArray<ZTResearchCategory>, // 0x38 - confirmed by `loadBranch` (populated from the `.cfg` `category=` list) and `pick_random_program`
    current_funding_level: i32,                  // 0x44 - index into the funding table, clamped by `increase_funding`/`decrease_funding`; reset to 0 by `loadBranch`
    funding_table_start: u32,                    // 0x48 - confirmed by `loadBranch`: inline `ZTResearchFundingLevel` table (stride 0xc, populated from the `.cfg` `funding=` list), *not* a `ZTArray` of pointers
    funding_table_end: u32,                      // 0x4c - confirmed by `loadBranch`
    funding_table_capacity: u32,                  // 0x50 - confirmed by `loadBranch` (checked to decide whether the table needs to grow); unused by us
}

impl ZTResearchBranch {
    pub fn id(&self) -> i32 {
        self.id
    }

    /// See `ZTResearchProgram::is_config_loaded`.
    pub fn is_config_loaded(&self) -> bool {
        self.config_file.is_loaded()
    }

    /// Display name, resolved through the same string table `get_string()` uses. Confirmed against
    /// the live game.
    pub fn name(&self) -> Option<String> {
        load_string_by_id(self.id as u32)
    }

    /// The load-time cached copy of `name()`'s text (see `loadBranch`), rather than a fresh
    /// `get_string()` lookup.
    pub fn cached_name(&self) -> String {
        self.cached_name.copy_to_string()
    }

    /// The `.cfg` `desc` field's cached text, if any (empty when `desc=` was blank).
    pub fn desc(&self) -> String {
        self.cached_desc.copy_to_string()
    }

    /// The `.cfg` `icon` field, if set.
    pub fn icon(&self) -> Option<String> {
        (self.icon_ptr != 0).then(|| unsafe { CStr::from_ptr(self.icon_ptr as *const c_char) }.to_string_lossy().into_owned())
    }

    /// The `.cfg` `noprogicon` field (the icon shown when no program is being researched), if set.
    pub fn noprogicon(&self) -> Option<String> {
        (self.noprogicon_ptr != 0)
            .then(|| unsafe { CStr::from_ptr(self.noprogicon_ptr as *const c_char) }.to_string_lossy().into_owned())
    }

    pub fn category_count(&self) -> usize {
        self.category_array.len()
    }

    pub fn category(&self, index: usize) -> &'static ZTResearchCategory {
        unsafe { ref_from_memory(self.category_array.get_ptr(index)) }
    }

    pub fn category_mut(&self, index: usize) -> &'static mut ZTResearchCategory {
        unsafe { mut_from_memory(self.category_array.get_ptr(index)) }
    }

    pub fn categories(&self) -> impl Iterator<Item = &'static ZTResearchCategory> + '_ {
        (0..self.category_count()).map(move |i| self.category(i))
    }

    pub fn categories_mut(&self) -> impl Iterator<Item = &'static mut ZTResearchCategory> + '_ {
        (0..self.category_count()).map(move |i| self.category_mut(i))
    }

    pub fn current_category(&self) -> Option<&'static ZTResearchCategory> {
        (self.current_category_ptr != 0).then(|| unsafe { ref_from_memory(self.current_category_ptr) })
    }

    pub fn current_program(&self) -> Option<&'static ZTResearchProgram> {
        (self.current_program_ptr != 0).then(|| unsafe { ref_from_memory(self.current_program_ptr) })
    }

    pub fn current_funding_level(&self) -> i32 {
        self.current_funding_level
    }

    fn funding_level_count(&self) -> usize {
        ((self.funding_table_end - self.funding_table_start) as usize) / size_of::<ZTResearchFundingLevel>()
    }

    fn funding_level(&self, index: usize) -> ZTResearchFundingLevel {
        get_from_memory(self.funding_table_start + (index * size_of::<ZTResearchFundingLevel>()) as u32)
    }

    /// Every entry in the funding-level table, for inspecting `unknown_0`/`unknown_8` (`rate` at
    /// offset `0x4` is the only confirmed field so far).
    pub fn funding_levels(&self) -> Vec<ZTResearchFundingLevel> {
        (0..self.funding_level_count()).map(|i| self.funding_level(i)).collect()
    }

    pub fn current_funding_rate(&self) -> Option<f32> {
        let index = self.current_funding_level as usize;
        (index < self.funding_level_count()).then(|| self.funding_level(index).rate())
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchBranch::increaseFunding`.
    pub fn increase_funding(&mut self) {
        let count = self.funding_level_count() as i32;
        if count == 0 {
            self.current_funding_level = 0;
        } else if self.current_funding_level + 1 < count {
            self.current_funding_level += 1;
        } else {
            self.current_funding_level = count - 1;
        }
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchBranch::decreaseFunding`.
    pub fn decrease_funding(&mut self) {
        if self.funding_level_count() > 0 && self.current_funding_level != 0 {
            self.current_funding_level -= 1;
        } else {
            self.current_funding_level = 0;
        }
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchBranch::pctRemainingOnProgram`. Returns `None`
    /// when there is no selected program or the active funding level isn't contributing (rate <= 0),
    /// mirroring the vanilla function's `-1` sentinel return.
    pub fn pct_remaining_on_program(&self) -> Option<i32> {
        let program = self.current_program()?;
        let rate = self.current_funding_rate()?;
        if rate <= 0.0 {
            return None;
        }
        Some((((program.target_cost - program.current_progress) * 100.0) / program.target_cost).round() as i32)
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchBranch::daysRemainingOnProgram`. Same guards as
    /// `pct_remaining_on_program`; the scale constant it multiplies by (`DAT_00635030`) is confirmed
    /// to be `30.0`.
    pub fn days_remaining_on_program(&self) -> Option<f32> {
        let program = self.current_program()?;
        let rate = self.current_funding_rate()?;
        if rate <= 0.0 {
            return None;
        }
        Some((program.target_cost - program.current_progress) * 30.0 / rate)
    }

    /// Calls the vanilla `ZTResearchBranch::update`; applies `days` in-game days of progress to the
    /// currently selected program. No decompile of this function's body is available.
    pub fn update(&mut self, days: u32) {
        unsafe { ztresearchbranch::UPDATE.original()((self as *mut Self) as *const u32, days) }
    }

    /// Calls the vanilla `ZTResearchBranch::pickRandomProgram`, which selects the branch's next
    /// active program (preferring one already in progress) using the game's own RNG stream.
    /// Reimplementing this natively risks desyncing that RNG stream from the rest of the game, so
    /// it's left as a call into the original implementation.
    pub fn pick_random_program(&mut self) {
        unsafe { ztresearchbranch::PICK_RANDOM_PROGRAM.original()((self as *mut Self) as *const u32) }
    }

    /// Calls the vanilla `ZTResearchBranch::loadBranch`, which reads a `.cfg` file (the same shape
    /// as this struct's own doc comment) and populates this branch's fields/`category_array`/funding
    /// table from it - the branch-level counterpart to `ZTResearchCategory::load_category`/
    /// `ZTResearchProgram::load_program`. `path` is a null-terminated path string.
    pub fn load_branch(&mut self, path: *const i8) -> u32 {
        unsafe { ztresearchbranch::LOAD_BRANCH.original()((self as *mut Self) as *const u32, path) }
    }

    /// Calls the vanilla `ZTResearchBranch::clearBranch`: resets `id` to `-1`, empties the cached
    /// name/desc buffers, zeroes `icon`/`noprogicon`/`current_category`/`current_program`, destroys
    /// and frees every category, and resets `category_array`/the funding table to empty (keeping
    /// their allocated capacity).
    pub fn clear_branch(&mut self) {
        unsafe { ztresearchbranch::CLEAR_BRANCH.original()((self as *mut Self) as *const u32) }
    }

    /// The vanilla "$400 (Min)"-style formatted text for the *currently selected* funding level (per
    /// `resources/decompiles/ZTResearchBranch_getFundingText.c`, this always uses
    /// `current_funding_level` - there's no way to ask for an arbitrary level's text). Calls into the
    /// original implementation (money formatting + a `%s`-templated name) rather than reimplementing
    /// it, since it depends on an unconfirmed scale constant (`DAT_00635040`) that only matters if
    /// you're reimplementing the formatting yourself.
    ///
    /// Note: the vanilla function may heap-allocate the returned string and frees it internally via
    /// an unnamed helper (`FUN_00401a2f`) we don't have a `FunctionDef` for, so we can't replicate
    /// that free here. In practice these strings are short enough to stay within the small-string-
    /// optimization inline buffer (no heap allocation at all), so this shouldn't leak in normal use -
    /// flagging it in case that ever changes.
    pub fn funding_text(&self) -> String {
        let mut buffer = [0u32; 3];
        unsafe {
            ztresearchbranch::GET_FUNDING_TEXT.original()(
                (self as *const Self) as *const u32,
                buffer.as_mut_ptr() as *const u32,
            );
        }
        get_from_memory::<ZTBufferString>(buffer.as_ptr() as u32).copy_to_string()
    }
}

impl fmt::Display for ZTResearchBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ZTResearchBranch {{")?;
        writeln!(f, "  id: {},", self.id)?;
        writeln!(f, "  name: {:?},", self.name())?;
        writeln!(f, "  desc: {:?},", self.desc())?;
        writeln!(f, "  icon: {:?},", self.icon())?;
        writeln!(f, "  noprogicon: {:?},", self.noprogicon())?;
        writeln!(f, "  category_count: {},", self.category_count())?;
        writeln!(f, "  current_funding_level: {},", self.current_funding_level)?;
        writeln!(f, "  current_category_ptr: {:#x},", self.current_category_ptr)?;
        writeln!(f, "  current_program_ptr: {:#x},", self.current_program_ptr)?;
        writeln!(f, "  pct_remaining_on_program: {:?},", self.pct_remaining_on_program())?;
        writeln!(f, "  days_remaining_on_program: {:?},", self.days_remaining_on_program())?;
        write!(f, "}}")
    }
}

/// The global research manager, one per game. Confirmed size `0x18` bytes (`openzt-detour/src/structs.rs`).
#[derive(Debug)]
#[repr(C)]
pub struct ZTResearchMgr {
    pad0: [u8; 0x8],                          // 0x00 - vtable? the byte at 0x01 is read by `ZTResearchBranch::pickRandomProgram` as an unconfirmed flag gating an expansion-availability check
    elapsed_ticks: u32,                       // 0x08 - accumulates `ZTResearchMgr::update`'s delta; once ~359 in-game days have accrued, every branch is updated and this resets to 0
    branch_array: ZTArray<ZTResearchBranch>,  // 0x0c
}

impl ZTResearchMgr {
    pub fn branch_count(&self) -> usize {
        self.branch_array.len()
    }

    pub fn branch(&self, index: usize) -> &'static ZTResearchBranch {
        unsafe { ref_from_memory(self.branch_array.get_ptr(index)) }
    }

    pub fn branch_mut(&self, index: usize) -> &'static mut ZTResearchBranch {
        unsafe { mut_from_memory(self.branch_array.get_ptr(index)) }
    }

    pub fn branches(&self) -> impl Iterator<Item = &'static ZTResearchBranch> + '_ {
        (0..self.branch_count()).map(move |i| self.branch(i))
    }

    pub fn branches_mut(&self) -> impl Iterator<Item = &'static mut ZTResearchBranch> + '_ {
        (0..self.branch_count()).map(move |i| self.branch_mut(i))
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchMgr::getBranch`.
    pub fn get_branch(&self, id: i32) -> Option<&'static ZTResearchBranch> {
        self.branches().find(|branch| branch.id == id)
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchMgr::getCategory`.
    pub fn get_category(&self, id: i32) -> Option<&'static ZTResearchCategory> {
        self.branches().flat_map(|branch| branch.categories()).find(|category| category.id == id)
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchMgr::getProgram`.
    pub fn get_program(&self, id: i32) -> Option<&'static ZTResearchProgram> {
        self.branches()
            .flat_map(|branch| branch.categories())
            .flat_map(|category| category.programs())
            .find(|program| program.id == id)
    }

    /// Reimplementation of `OOAnalyzer::ZTResearchMgr::setEffectDiscount`: applies a percentage
    /// discount to the `target_cost` of every program whose effect kind matches `kind`.
    pub fn set_effect_discount(&self, kind: ZTResearchEffectKind, discount_pct: i32) {
        for program in self.branches_mut().flat_map(|b| b.categories_mut()).flat_map(|c| c.programs_mut()) {
            if program.effect_kind_raw == kind as i32 {
                program.target_cost = (100 - discount_pct) as f32 * program.target_cost * 0.01;
            }
        }
    }

    /// Calls the vanilla `ZTResearchMgr::update`. `delta_ticks` is added to an internal accumulator;
    /// once enough time has accrued every branch is advanced (see `pad0`/`elapsed_ticks` above).
    pub fn update(&mut self, delta_ticks: i32) -> i32 {
        unsafe { ztresearchmgr::UPDATE.original()((self as *mut Self) as *const u32, delta_ticks as u32) }
    }

    /// Calls the vanilla `ZTResearchMgr::save`. `file` is whatever file-handle pointer the original
    /// `WriteBytesToFile` calls expect.
    pub fn save(&self, file: *const u32) -> bool {
        unsafe { ztresearchmgr::SAVE.original()((self as *const Self) as *const u32, file) != 0 }
    }

    /// Calls the vanilla `ZTResearchMgr::load` - the save-file counterpart to `save()`. Per
    /// `resources/decompiles/ZTResearchMgr_load.c`/`.asm`, `load` always starts by resetting every
    /// branch's `current_funding_level` to `0`, every category's `enabled` to `1`, and calling
    /// `ZTResearchProgram::reset()` on every program (which itself zeroes `current_progress` and, for
    /// `UnlockEntity`/`BuildingUpgrade` effects, calls back into the building/entity managers) -
    /// unconditionally, regardless of `version` or what's in the stream. Only if `version >= 0x28`
    /// does it then read a stream of `(kind, id, value)` tuples from `file` (`kind` 0 = a branch's
    /// `current_funding_level`, clamped to `0` if the saved value is `>=` that branch's own
    /// funding-level count; 1 = a category's `enabled` flag; 2 = a program's `current_progress`) and
    /// apply each one to the matching branch/category/program found via
    /// `get_branch`/`get_category`/`get_program` (an id with no match is silently skipped). Finally,
    /// regardless of `version`, it calls `ZTResearchProgram::on_completion()` on any program whose
    /// `current_progress >= target_cost` and `ZTResearchBranch::pick_random_program()` on every
    /// branch (consuming the game's RNG stream). Does **not** load research definitions from `.cfg`
    /// files - that's `ZTResearchBranch::load_branch`/`ZTResearchCategory::load_category`/
    /// `ZTResearchProgram::load_program`.
    pub fn load(&mut self, file: *const u32, version: u32) -> bool {
        unsafe { ztresearchmgr::LOAD.original()((self as *mut Self) as *const u32, file, version) }
    }

    /// Calls the vanilla `ZTResearchMgr::forceResearch` (the class-level half of the "research
    /// cheat"): immediately completes every branch's current program via
    /// `ZTResearchProgram::on_completion`, optionally carrying remaining progress into the next
    /// program. Unlike the actual in-game cheat button, this does *not* refresh the world/UI
    /// afterward - use the free function `force_research_cheat()` for that (it calls this with
    /// `continue_program` hardcoded to `false`, matching what the button does, plus the refresh).
    pub fn force_research(&mut self, continue_program: bool) {
        unsafe { ztresearchmgr::FORCE_RESEARCH.original()((self as *mut Self) as *const u32, continue_program) }
    }

    /// Calls the vanilla `ZTResearchMgr::clearBranches`: destroys and frees every branch (and
    /// everything under it), then resets `branch_array` to empty.
    pub fn clear_branches(&mut self) {
        unsafe { ztresearchmgr::CLEAR_BRANCHES.original()((self as *mut Self) as *const u32) }
    }
}

impl fmt::Display for ZTResearchMgr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ZTResearchMgr {{")?;
        writeln!(f, "  elapsed_ticks: {},", self.elapsed_ticks)?;
        writeln!(f, "  branch_count: {},", self.branch_count())?;
        write!(f, "}}")
    }
}

/// Calls the vanilla standalone `forceResearch` cheat-console function - the one the in-game
/// "force research" cheat actually triggers, per `resources/decompiles/_forceResearch.c`. It calls
/// `ZTResearchMgr::forceResearch(GLOBAL_ZTResearchMgr, false)` (the `continue_program` flag is
/// hardcoded `false` here - use `ZTResearchMgr::force_research` directly if you need `true`) and
/// then notifies every `ZTWorldMgr` entity of the change, so the world/UI actually refreshes; plain
/// `ZTResearchMgr::force_research` alone does not do that last step. Takes no arguments and needs no
/// `ZTResearchMgr` reference - it looks up the global instance itself.
pub fn force_research_cheat() {
    unsafe { standalone::FORCE_RESEARCH.original()() }
}

/// a command that prints the top-level ZTResearchMgr summary
/// usage: `get_ztresearchmgr`
fn command_get_ztresearchmgr(_args: Vec<&str>) -> Result<String, CommandError> {
    Ok(format!("{}", globals().ztresearchmgr()))
}

/// a command that resolves the ZTResearchMgr global step by step, logging each hop, so a crash
/// during resolution shows exactly which step it happened on instead of taking the whole game down
/// silently. Doesn't dereference the manager itself, only the pointer chain that finds it.
/// usage: `debug_research_ptr`
fn command_debug_research_ptr(_args: Vec<&str>) -> Result<String, CommandError> {
    let base = get_module_base("zoo.exe");
    info!("[research-debug] module base (\"zoo.exe\") = {:#x}", base);

    let global_slot = base as u32 + 0x0023_9010;
    info!("[research-debug] global slot address (base + 0x239010) = {:#x}", global_slot);

    let raw_slot_value = get_from_memory::<u32>(global_slot);
    info!("[research-debug] value stored at global slot = {:#x}", raw_slot_value);

    if raw_slot_value == 0 {
        error!("[research-debug] global slot is null; ZTResearchMgr not constructed yet, or the address/indirection is wrong");
        return Err(CommandError::new("global slot at base + 0x239010 is null".to_string()));
    }

    let mgr_ptr = globals().ztresearchmgr_ptr();
    info!("[research-debug] globals().ztresearchmgr_ptr() = {:#x}", mgr_ptr as u32);

    if mgr_ptr.is_null() {
        error!("[research-debug] resolved ZTResearchMgr pointer is null");
        return Err(CommandError::new("resolved ZTResearchMgr pointer is null".to_string()));
    }

    Ok(format!(
        "module_base={:#x} global_slot={:#x} raw_slot_value={:#x} mgr_ptr={:#x}",
        base, global_slot, raw_slot_value, mgr_ptr as u32
    ))
}

/// a command that prints every branch/category/program in the research tree, for verifying the
/// struct layouts in this file line up with the live game. Every pointer is logged and null-checked
/// before it's dereferenced, and iteration counts are sanity-checked against
/// `MAX_REASONABLE_*`, so if this crashes the log will show exactly which (branch, category,
/// program) index it happened on rather than taking the whole game down blind.
/// usage: `list_research`
fn command_list_research(_args: Vec<&str>) -> Result<String, CommandError> {
    let mgr_ptr = globals().ztresearchmgr_ptr();
    info!("[research-debug] ztresearchmgr_ptr() = {:#x}", mgr_ptr as u32);
    if mgr_ptr.is_null() {
        error!("[research-debug] ZTResearchMgr pointer is null; run debug_research_ptr() for details");
        return Err(CommandError::new("ZTResearchMgr pointer is null".to_string()));
    }

    let mgr = globals().ztresearchmgr();
    info!("[research-debug] mgr.elapsed_ticks = {}", mgr.elapsed_ticks);

    let branch_count = mgr.branch_array.len();
    info!("[research-debug] mgr.branch_array.len() = {}", branch_count);
    if branch_count > MAX_REASONABLE_BRANCHES {
        error!(
            "[research-debug] branch_array.len() = {} exceeds MAX_REASONABLE_BRANCHES ({}); bailing out before dereferencing to avoid a crash - the global address/indirection is probably wrong",
            branch_count, MAX_REASONABLE_BRANCHES
        );
        return Err(CommandError::new(format!("implausible branch count {}", branch_count)));
    }

    let mut result = format!("{}\n", mgr);

    for i in 0..branch_count {
        let branch_ptr = mgr.branch_array.get_ptr(i);
        info!("[research-debug] branch[{}] ptr = {:#x}", i, branch_ptr);
        if branch_ptr == 0 {
            warn!("[research-debug] branch[{}] is null, skipping", i);
            continue;
        }
        let branch = mgr.branch(i);
        result.push_str(&format!("{}\n", branch));

        for (level_index, level) in branch.funding_levels().iter().enumerate() {
            result.push_str(&format!(
                "  FundingLevel[{}]: name={:?} rate(work)={} cost={}\n",
                level_index,
                level.name(),
                level.rate(),
                level.cost()
            ));
        }

        let category_count = branch.category_array.len();
        info!("[research-debug] branch[{}] (id={}) category_array.len() = {}", i, branch.id, category_count);
        if category_count > MAX_REASONABLE_CATEGORIES {
            error!(
                "[research-debug] branch[{}].category_array.len() = {} exceeds MAX_REASONABLE_CATEGORIES ({}); skipping this branch's categories",
                i, category_count, MAX_REASONABLE_CATEGORIES
            );
            continue;
        }

        for j in 0..category_count {
            let category_ptr = branch.category_array.get_ptr(j);
            info!("[research-debug] branch[{}].category[{}] ptr = {:#x}", i, j, category_ptr);
            if category_ptr == 0 {
                warn!("[research-debug] branch[{}].category[{}] is null, skipping", i, j);
                continue;
            }
            let category = branch.category(j);
            result.push_str(&format!("{}\n", category));

            let program_count = category.program_array.len();
            info!(
                "[research-debug] branch[{}].category[{}] (id={}) program_array.len() = {}",
                i, j, category.id, program_count
            );
            if program_count > MAX_REASONABLE_PROGRAMS {
                error!(
                    "[research-debug] branch[{}].category[{}].program_array.len() = {} exceeds MAX_REASONABLE_PROGRAMS ({}); skipping this category's programs",
                    i, j, program_count, MAX_REASONABLE_PROGRAMS
                );
                continue;
            }

            for k in 0..program_count {
                let program_ptr = category.program_array.get_ptr(k);
                info!("[research-debug] branch[{}].category[{}].program[{}] ptr = {:#x}", i, j, k, program_ptr);
                if program_ptr == 0 {
                    warn!("[research-debug] branch[{}].category[{}].program[{}] is null, skipping", i, j, k);
                    continue;
                }
                let program = category.program(k);
                result.push_str(&format!("{}\n", program));
            }
        }
    }

    Ok(result)
}

/// a command that prints each branch's currently selected category/program and funding level
/// usage: `current_research`
fn command_current_research(_args: Vec<&str>) -> Result<String, CommandError> {
    let mgr = globals().ztresearchmgr();
    let mut result = String::new();

    for branch in mgr.branches() {
        result.push_str(&format!(
            "Branch {} ({:?}) - funding level {}",
            branch.id(),
            branch.name(),
            branch.current_funding_level()
        ));
        match branch.current_funding_rate() {
            Some(rate) => result.push_str(&format!(" (rate {}):\n", rate)),
            None => result.push_str(" (rate unavailable):\n"),
        }

        match branch.current_category() {
            Some(category) => {
                result.push_str(&format!("  category: id={} name={:?}\n", category.id(), category.name()))
            }
            None => result.push_str("  category: none selected\n"),
        }

        match branch.current_program() {
            Some(program) => {
                result.push_str(&format!(
                    "  program: id={}, progress={:.1}/{:.1}",
                    program.id(),
                    program.current_progress(),
                    program.target_cost()
                ));
                match branch.pct_remaining_on_program() {
                    Some(pct) => result.push_str(&format!(", {}% remaining", pct)),
                    None => result.push_str(", % remaining unavailable"),
                }
                match branch.days_remaining_on_program() {
                    Some(days) => result.push_str(&format!(", {:.1} days remaining\n", days)),
                    None => result.push_str(", days remaining unavailable\n"),
                }
            }
            None => result.push_str("  program: none selected\n"),
        }
    }

    Ok(result)
}

/// a command that would restore research progress from the active save file via `ZTResearchMgr::load`.
/// Currently unsupported from the console: `load` needs a real open save-file stream pointer (which it
/// dereferences directly to check the stream's EOF flag - not just something it forwards opaquely) plus
/// the save's format version, neither of which the console has access to. Previously this command called
/// `load()` with neither argument, which - since `load` is a 2-stack-arg `thiscall` (`ret 0x8`) - silently
/// corrupted the stack on return; that bug is fixed by requiring both arguments explicitly rather than by
/// guessing values that would just crash instead.
/// usage: `load_research`
fn command_load_research(_args: Vec<&str>) -> Result<String, CommandError> {
    Err(CommandError::new(
        "load_research is currently unsupported: ZTResearchMgr::load needs a real save-file stream pointer and version, which aren't available from the console".to_string(),
    ))
}

/// a command that triggers the same "force research" cheat the in-game button does (completes every
/// branch's current program and refreshes the world/UI) via `force_research_cheat`
/// usage: `force_research`
fn command_force_research(_args: Vec<&str>) -> Result<String, CommandError> {
    force_research_cheat();
    Ok("Forced research completion".to_string())
}

/// registers the Lua functions
pub fn init() {
    // get_ztresearchmgr() - no args
    lua_fn!("get_ztresearchmgr", "Returns ZTResearchMgr debug info", "get_ztresearchmgr()", || {
        match command_get_ztresearchmgr(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string()))),
        }
    });

    // debug_research_ptr() - no args
    lua_fn!(
        "debug_research_ptr",
        "Resolves the ZTResearchMgr global step by step, logging each hop",
        "debug_research_ptr()",
        || {
            match command_debug_research_ptr(vec![]) {
                Ok(result) => Ok((Some(result), None::<String>)),
                Err(e) => Ok((None::<String>, Some(e.to_string()))),
            }
        }
    );

    // list_research() - no args
    lua_fn!(
        "list_research",
        "Lists every research branch/category/program, logging every pointer as it's walked",
        "list_research()",
        || {
            match command_list_research(vec![]) {
                Ok(result) => Ok((Some(result), None::<String>)),
                Err(e) => Ok((None::<String>, Some(e.to_string()))),
            }
        }
    );

    // current_research() - no args
    lua_fn!(
        "current_research",
        "Shows each branch's currently selected category/program and funding level",
        "current_research()",
        || {
            match command_current_research(vec![]) {
                Ok(result) => Ok((Some(result), None::<String>)),
                Err(e) => Ok((None::<String>, Some(e.to_string()))),
            }
        }
    );

    // load_research() - no args
    lua_fn!(
        "load_research",
        "Restores research progress from the active save file",
        "load_research()",
        || {
            match command_load_research(vec![]) {
                Ok(result) => Ok((Some(result), None::<String>)),
                Err(e) => Ok((None::<String>, Some(e.to_string()))),
            }
        }
    );

    // force_research() - no args
    lua_fn!(
        "force_research",
        "Triggers the same 'force research' cheat the in-game button does",
        "force_research()",
        || {
            match command_force_research(vec![]) {
                Ok(result) => Ok((Some(result), None::<String>)),
                Err(e) => Ok((None::<String>, Some(e.to_string()))),
            }
        }
    );

    research_config_reimplementation::init();
}

/// Native reimplementation of the `.cfg`-driven research tree loading
/// (`ZTResearchBranch::loadBranch`/`ZTResearchCategory::loadCategory`/`ZTResearchProgram::loadProgram`),
/// built on top of the existing generic `openzt-configparser` INI parser rather than a bespoke port of
/// `BFConfigFile`'s parsing engine.
///
/// By default (the `#[cfg(not(feature = "vanilla-research-config"))]` arm) this module owns the full
/// construction *and* destruction lifecycle of `ZTResearchBranch`/`ZTResearchCategory`/
/// `ZTResearchProgram` objects: `loadBranches` parses every manifest entry itself and splices
/// Rust-allocated objects directly into `branch_array`/`category_array`/`program_array`, and the six
/// vanilla lifecycle functions (`clearBranches`/`clearBranch`/`~ZTResearchBranch`/`clearCategory`/
/// `~ZTResearchCategory`/`~ZTResearchProgram`) are replaced outright with equivalent Rust logic that
/// frees the same Rust-allocated memory - so no vanilla `delete` ever runs against Rust-allocated memory
/// and no vanilla `new` output is ever freed with a mismatched deallocator. If parsing fails partway
/// through a manifest, the whole call falls back to vanilla's own `loadBranches` (see `replace_branches`).
///
/// With `--features vanilla-research-config`, only `loadBranches` is detoured, and it reverts to its
/// original shadow-mode behavior: the vanilla function always runs first and its result is always what's
/// returned, while this module independently parses the same files and logs any mismatch - useful for
/// re-validating the parsing/field logic shared by both arms without risking gameplay. The other six
/// lifecycle detours become plain passthroughs to vanilla in this arm.
mod research_config_reimplementation {
    use openzt_configparser::ini::Ini;
    use openzt_detour_macro::detour_mod;
    use tracing::{debug, error, info};

    use super::*;
    use crate::{encoding_utils::decode_game_text, resource_manager::lazyresourcemap::get_file};

    #[derive(Debug, Default)]
    struct ReimplementedProgram {
        name_id: i32,
        desc_id: i32,
        icon: Option<String>,
        entity_icon: Option<String>,
        cost: f32,
        order: i32,
        target: i32,
        effect: i32,
        effect_params: (i32, i32, i32),
        help_id: i32,
    }

    #[derive(Debug, Default)]
    struct ReimplementedFundingLevel {
        name_id: i32,
        rate: f32,
        cost: f32,
    }

    #[derive(Debug, Default)]
    struct ReimplementedCategory {
        name_id: i32,
        desc_id: i32,
        icon: Option<String>,
        help_id: i32,
        expansion_id: i32,
        programs: Vec<ReimplementedProgram>,
    }

    #[derive(Debug, Default)]
    struct ReimplementedBranch {
        name_id: i32,
        desc_id: i32,
        icon: Option<String>,
        noprogicon: Option<String>,
        funding: Vec<ReimplementedFundingLevel>,
        categories: Vec<ReimplementedCategory>,
    }

    /// Loads and parses a resource-relative `.cfg` path the same way `legacy_loading.rs` does for
    /// mod `.cfg` files, except with vanilla's actual comment convention (`;` only - `BFConfigFile::parse`
    /// never treats `#`/`:` as comments, unlike the leniency OpenZT's own mod loader allows).
    fn read_cfg(path: &str) -> Option<Ini> {
        let Some((_, data)) = get_file(path) else {
            error!("research-config-reimplementation: resource '{path}' not found");
            return None;
        };
        let text = decode_game_text(&data);
        let mut ini = Ini::new_cs();
        ini.set_comment_symbols(&[';']);
        match ini.read(text) {
            Ok(_) => Some(ini),
            Err(e) => {
                error!("research-config-reimplementation: failed to parse '{path}': {e}");
                None
            }
        }
    }

    /// All values for a repeated key, dropping any that trim to empty. Confirmed against the vanilla
    /// `.cfg` source (e.g. `icon=` with nothing after it in `research/branres.cfg`) and
    /// `BFConfigFile::addKeyVal` (`BFConfigFile_addKeyVal.c`): a value that trims to nothing is never
    /// pushed onto the key's value vector at all, so from `getString`/`getStringList`'s perspective an
    /// empty `icon=` line is indistinguishable from no `icon=` line - both leave the vector empty.
    /// `Ini::get_vec` keeps the empty string, so this filters it back out to match.
    fn values(ini: &Ini, section: &str, key: &str) -> Vec<String> {
        ini.get_vec(section, key).unwrap_or_default().into_iter().filter(|v| !v.trim().is_empty()).collect()
    }

    /// `BFConfigFile::getString`/`getInt`/`getFloat` all return the *first* value for a repeated key
    /// (see `BFConfigFile_getString.c`); `Ini::get` returns the *last* one instead, so pull from
    /// `values` directly to match vanilla.
    fn first(ini: &Ini, section: &str, key: &str) -> Option<String> {
        values(ini, section, key).into_iter().next()
    }

    fn first_parse<T: std::str::FromStr>(ini: &Ini, section: &str, key: &str) -> Option<T> {
        first(ini, section, key)?.parse().ok()
    }

    fn load_program(path: &str) -> Option<ReimplementedProgram> {
        let ini = read_cfg(path)?;
        Some(ReimplementedProgram {
            name_id: first_parse(&ini, "research", "name").unwrap_or_default(),
            desc_id: first_parse(&ini, "research", "desc").unwrap_or_default(),
            icon: first(&ini, "research", "icon"),
            entity_icon: first(&ini, "research", "entityIcon"),
            cost: first_parse(&ini, "research", "cost").unwrap_or_default(),
            order: first_parse(&ini, "research", "order").unwrap_or_default(),
            target: first_parse(&ini, "research", "target").unwrap_or(-1),
            effect: first_parse(&ini, "research", "effect").unwrap_or(-1),
            effect_params: (
                first_parse(&ini, "research", "effectval1").unwrap_or_default(),
                first_parse(&ini, "research", "effectval2").unwrap_or_default(),
                first_parse(&ini, "research", "effectval3").unwrap_or_default(),
            ),
            help_id: first_parse(&ini, "research", "helpid").unwrap_or_default(),
        })
    }

    fn load_category(path: &str) -> Option<ReimplementedCategory> {
        let ini = read_cfg(path)?;
        let programs = values(&ini, "category", "program").iter().filter_map(|p| load_program(p)).collect();
        Some(ReimplementedCategory {
            name_id: first_parse(&ini, "category", "name").unwrap_or_default(),
            desc_id: first_parse(&ini, "category", "desc").unwrap_or_default(),
            icon: first(&ini, "category", "icon"),
            help_id: first_parse(&ini, "category", "helpid").unwrap_or_default(),
            expansion_id: first_parse(&ini, "category", "expansion").unwrap_or_default(),
            programs,
        })
    }

    fn load_branch(ini: &Ini) -> ReimplementedBranch {
        let categories = values(ini, "branch", "category").iter().filter_map(|p| load_category(p)).collect();
        let funding = values(ini, "branch", "funding")
            .iter()
            .map(|block| ReimplementedFundingLevel {
                name_id: first_parse(ini, block, "name").unwrap_or_default(),
                rate: first_parse(ini, block, "work").unwrap_or_default(),
                cost: first_parse(ini, block, "cost").unwrap_or_default(),
            })
            .collect();
        ReimplementedBranch {
            name_id: first_parse(ini, "branch", "name").unwrap_or_default(),
            desc_id: first_parse(ini, "branch", "desc").unwrap_or_default(),
            icon: first(ini, "branch", "icon"),
            noprogicon: first(ini, "branch", "noprogicon"),
            funding,
            categories,
        }
    }

    /// Resolves a string id the same way `load_string_by_id` does, defaulting to an empty string
    /// (matching how `cached_name`/`cached_desc` read back when nothing was ever cached) instead of
    /// `None`, so it can be compared directly against `cached_name()`/`cached_desc()`.
    fn resolved(id: i32) -> String {
        load_string_by_id(id as u32).unwrap_or_default()
    }

    #[cfg(feature = "vanilla-research-config")]
    fn compare_program(path: &str, live: &ZTResearchProgram, reimpl: &ReimplementedProgram, mismatches: &mut Vec<String>) {
        if live.id() != reimpl.name_id {
            mismatches.push(format!("{path}: name id {} != {}", live.id(), reimpl.name_id));
        }
        if live.cached_name() != resolved(reimpl.name_id) {
            mismatches.push(format!("{path}: name text {:?} != {:?}", live.cached_name(), resolved(reimpl.name_id)));
        }
        if live.cached_desc() != resolved(reimpl.desc_id) {
            mismatches.push(format!("{path}: desc text {:?} != {:?}", live.cached_desc(), resolved(reimpl.desc_id)));
        }
        if live.icon() != reimpl.icon {
            mismatches.push(format!("{path}: icon {:?} != {:?}", live.icon(), reimpl.icon));
        }
        if live.entity_icon() != reimpl.entity_icon {
            mismatches.push(format!("{path}: entityIcon {:?} != {:?}", live.entity_icon(), reimpl.entity_icon));
        }
        if (live.target_cost() - reimpl.cost).abs() > f32::EPSILON {
            mismatches.push(format!("{path}: cost {} != {}", live.target_cost(), reimpl.cost));
        }
        if live.priority() != reimpl.order as u32 {
            mismatches.push(format!("{path}: order {} != {}", live.priority(), reimpl.order));
        }
        if live.target_id() != reimpl.target {
            mismatches.push(format!("{path}: target {} != {}", live.target_id(), reimpl.target));
        }
        if live.effect_kind_raw != reimpl.effect {
            mismatches.push(format!("{path}: effect {} != {}", live.effect_kind_raw, reimpl.effect));
        }
        if live.effect_params() != reimpl.effect_params {
            mismatches.push(format!("{path}: effect params {:?} != {:?}", live.effect_params(), reimpl.effect_params));
        }
        if live.help_id() != reimpl.help_id {
            mismatches.push(format!("{path}: helpid {} != {}", live.help_id(), reimpl.help_id));
        }
    }

    #[cfg(feature = "vanilla-research-config")]
    fn compare_category(path: &str, live: &ZTResearchCategory, reimpl: &ReimplementedCategory, mismatches: &mut Vec<String>) {
        if live.id() != reimpl.name_id {
            mismatches.push(format!("{path}: name id {} != {}", live.id(), reimpl.name_id));
        }
        if live.cached_name() != resolved(reimpl.name_id) {
            mismatches.push(format!("{path}: name text {:?} != {:?}", live.cached_name(), resolved(reimpl.name_id)));
        }
        if live.desc() != resolved(reimpl.desc_id) {
            mismatches.push(format!("{path}: desc text {:?} != {:?}", live.desc(), resolved(reimpl.desc_id)));
        }
        if live.icon() != reimpl.icon {
            mismatches.push(format!("{path}: icon {:?} != {:?}", live.icon(), reimpl.icon));
        }
        if live.help_id() != reimpl.help_id {
            mismatches.push(format!("{path}: helpid {} != {}", live.help_id(), reimpl.help_id));
        }
        if live.expansion_id() != reimpl.expansion_id {
            mismatches.push(format!("{path}: expansion {} != {}", live.expansion_id(), reimpl.expansion_id));
        }
        if live.program_count() != reimpl.programs.len() {
            mismatches.push(format!("{path}: program count {} != {}", live.program_count(), reimpl.programs.len()));
        }
        for (i, (live_program, reimpl_program)) in live.programs().zip(reimpl.programs.iter()).enumerate() {
            compare_program(&format!("{path}[program {i}]"), live_program, reimpl_program, mismatches);
        }
    }

    /// Compares a just-`loadBranch`-ed live branch against the reimplementation's parse of the same
    /// file. `categories_before` is the branch's `category_count()` *before* this call to `loadBranch`
    /// ran.
    ///
    /// `loadBranch` never clears the branch first (unlike `loadCategory`, whose first line is
    /// `clearCategory`) - it only resets `current_funding_level`. `category_array` is append-only
    /// across repeated `loadBranch` calls on the same branch object (confirmed: `research/branres.cfg`
    /// and `research/brandres.cfg` - base game vs. the dinosaur pack's branch file - both declare
    /// `name=23050` and get loaded into the literal same object, confirmed via the raw `this` pointer
    /// logged from both calls being identical), so it's compared positionally against just the entries
    /// newly appended by *this* call, skipping however many were already there.
    ///
    /// The funding table is different: it's overwritten in place on every call, not accumulated -
    /// confirmed by poisoning an existing entry's `name_id` with an impossible sentinel right before
    /// calling the original `loadBranch` and observing it always comes back overwritten (with the
    /// correct value, not the sentinel), even though the table's start pointer/capacity/count never
    /// change across calls. So it's compared directly, in full, against this file's own parsed funding
    /// list - the same plain "overwritten every call" treatment as the `name`/`desc`/`icon` header
    /// fields, not the accumulate-and-skip treatment `category_array` needs.
    #[cfg(feature = "vanilla-research-config")]
    fn compare_branch(path: &str, live: &ZTResearchBranch, reimpl: &ReimplementedBranch, categories_before: usize, mismatches: &mut Vec<String>) {
        if live.id() != reimpl.name_id {
            mismatches.push(format!("{path}: name id {} != {}", live.id(), reimpl.name_id));
        }
        if live.cached_name() != resolved(reimpl.name_id) {
            mismatches.push(format!("{path}: name text {:?} != {:?}", live.cached_name(), resolved(reimpl.name_id)));
        }
        if live.desc() != resolved(reimpl.desc_id) {
            mismatches.push(format!("{path}: desc text {:?} != {:?}", live.desc(), resolved(reimpl.desc_id)));
        }
        if live.icon() != reimpl.icon {
            mismatches.push(format!("{path}: icon {:?} != {:?}", live.icon(), reimpl.icon));
        }
        if live.noprogicon() != reimpl.noprogicon {
            mismatches.push(format!("{path}: noprogicon {:?} != {:?}", live.noprogicon(), reimpl.noprogicon));
        }
        let live_funding = live.funding_levels();
        if live_funding.len() != reimpl.funding.len() {
            mismatches.push(format!("{path}: funding level count {} != {}", live_funding.len(), reimpl.funding.len()));
        }
        for (i, (live_level, reimpl_level)) in live_funding.iter().zip(reimpl.funding.iter()).enumerate() {
            if live_level.name_id() != reimpl_level.name_id {
                mismatches.push(format!("{path}[funding {i}]: name id {} != {}", live_level.name_id(), reimpl_level.name_id));
            }
            if (live_level.rate() - reimpl_level.rate).abs() > f32::EPSILON {
                mismatches.push(format!("{path}[funding {i}]: work/rate {} != {}", live_level.rate(), reimpl_level.rate));
            }
            if (live_level.cost() - reimpl_level.cost).abs() > f32::EPSILON {
                mismatches.push(format!("{path}[funding {i}]: cost {} != {}", live_level.cost(), reimpl_level.cost));
            }
        }
        let new_category_count = live.category_count().saturating_sub(categories_before);
        if new_category_count != reimpl.categories.len() {
            mismatches.push(format!(
                "{path}: newly appended category count {} != {} (already had {categories_before} before this call)",
                new_category_count,
                reimpl.categories.len()
            ));
        }
        for (i, (live_category, reimpl_category)) in live.categories().skip(categories_before).zip(reimpl.categories.iter()).enumerate() {
            compare_category(&format!("{path}[category {i}]"), live_category, reimpl_category, mismatches);
        }
    }

    /// Peeks a branch file's own declared `[branch] name=<id>` without doing a full load - the same
    /// thing `ZTResearchMgr::loadBranches` does per manifest entry (via a throwaway `BFConfigFile`)
    /// before deciding whether to reuse an existing branch or create a new one.
    #[cfg(feature = "vanilla-research-config")]
    fn peek_branch_id(path: &str) -> Option<i32> {
        let ini = read_cfg(path)?;
        first_parse(&ini, "branch", "name")
    }

    /// The manifest's own `[branches] branch=...` list (confirmed against the real
    /// `config.ztd!research.cfg` - `[branches]` section, repeated `branch=` key - not the
    /// `[branch] category=...`/`funding=...` shape every other file in this family uses).
    fn load_manifest(ini: &Ini) -> Vec<String> {
        values(ini, "branches", "branch")
    }

    /// One manifest entry, resolved against live state *before* `loadBranches` runs: which branch file,
    /// what id it declares, and how many categories that id's branch already had (0 for an id that
    /// doesn't exist yet, i.e. this entry will create a new branch rather than extend an existing one).
    #[cfg(feature = "vanilla-research-config")]
    struct ManifestEntry {
        path: String,
        id: i32,
        categories_before: usize,
    }

    #[cfg(feature = "vanilla-research-config")]
    fn manifest_entries(manifest_ini: &Ini, mgr_before: &ZTResearchMgr) -> Vec<ManifestEntry> {
        load_manifest(manifest_ini)
            .into_iter()
            .filter_map(|path| {
                let Some(id) = peek_branch_id(&path) else {
                    error!("research-config-reimplementation: failed to peek id from '{path}' listed in manifest");
                    return None;
                };
                let categories_before = mgr_before.get_branch(id).map(|b| b.category_count()).unwrap_or(0);
                Some(ManifestEntry { path, id, categories_before })
            })
            .collect()
    }

    /// Compares the whole outcome of a `loadBranches` call: that `branch_array` grew by exactly the
    /// number of genuinely new ids (an id already present before this call, or repeated within the same
    /// manifest, is a reuse and shouldn't grow the count), and, for every manifest entry, the full
    /// branch/category/program content behind its (possibly reused) branch object - the same
    /// `compare_branch` used to run from a per-`loadBranch`-call detour, now run once per manifest entry
    /// from the single top-level `loadBranches` hook instead.
    #[cfg(feature = "vanilla-research-config")]
    fn compare_load_branches(manifest_path: &str, ids_before: &std::collections::HashSet<i32>, mgr_after: &ZTResearchMgr, entries: &[ManifestEntry], mismatches: &mut Vec<String>) {
        let new_id_count = entries.iter().map(|e| e.id).collect::<std::collections::HashSet<_>>().difference(ids_before).count();
        let expected_count = ids_before.len() + new_id_count;
        if mgr_after.branch_count() != expected_count {
            mismatches.push(format!(
                "{manifest_path}: branch_array has {} entries after, expected {} ({} before + {} new)",
                mgr_after.branch_count(),
                expected_count,
                ids_before.len(),
                new_id_count
            ));
        }

        for entry in entries {
            let Some(live_branch) = mgr_after.get_branch(entry.id) else {
                mismatches.push(format!("{}: branch id {} not found in branch_array after loadBranches", entry.path, entry.id));
                continue;
            };
            let Some(ini) = read_cfg(&entry.path) else {
                error!("research-config-reimplementation: failed to independently parse '{}' for comparison", entry.path);
                continue;
            };
            let reimpl = load_branch(&ini);
            compare_branch(&entry.path, live_branch, &reimpl, entry.categories_before, mismatches);
        }
    }

    /// Low-level raw-memory helpers shared by `construction` and `destruction`: reconstructing an
    /// owning `Vec` from a live struct's raw start/end/(buffer_end|capacity) pointers, and the
    /// reverse (leaking a `Vec`'s buffer back into those same three raw pointers). Every allocation
    /// in this file goes through Rust's own allocator (`Box`/`Vec`), never vanilla's game heap - see
    /// the module doc comment above for why. `ZTArray<T>`/`ZTBufferString`'s 3-pointer shape
    /// (`start_ptr`/`end_ptr`/`buffer_end_ptr`) is the same read-side layout already validated
    /// extensively elsewhere in this file; these helpers just add the write side.
    #[cfg(not(feature = "vanilla-research-config"))]
    mod raw_mem {
        use std::ffi::{c_char, CString};

        use super::*;

        /// Allocates a null-terminated buffer for `text` and wraps it in a `ZTBufferString`. The
        /// null terminator sits immediately at `end_ptr` (matching a plain C string), with
        /// `buffer_end_ptr` covering whatever spare capacity the backing `Vec` happened to allocate -
        /// harmless slack, not a correctness requirement, since every reader in this file
        /// (`copy_to_string`/etc.) stops at `end_ptr` regardless.
        pub(super) fn alloc_buffer_string(text: &str) -> ZTBufferString {
            let mut bytes = text.as_bytes().to_vec();
            let len = bytes.len() as u32;
            bytes.push(0);
            let cap = bytes.capacity() as u32;
            let ptr = bytes.as_mut_ptr() as u32;
            std::mem::forget(bytes);
            ZTBufferString::from_raw_parts(ptr, ptr + len, ptr + cap)
        }

        pub(super) fn free_buffer_string(s: &ZTBufferString) {
            let (start, _end, buffer_end) = s.raw_parts();
            if start == 0 {
                return;
            }
            let cap = (buffer_end - start) as usize;
            drop(unsafe { Vec::<u8>::from_raw_parts(start as *mut u8, cap, cap) });
        }

        /// Allocates an owned, null-terminated C string for `text` (0 for `None`) for
        /// `icon_ptr`/`entity_icon_ptr`/`noprogicon_ptr`. Unlike vanilla - whose equivalent fields
        /// point into a shared, separately-owned config string table that's never freed per-object
        /// (see every `~ZTResearch*` decompile: none of them touch these fields) - we own this
        /// allocation outright, so `free_owned_cstring` (not vanilla) is responsible for reclaiming
        /// it, both when overwriting an existing value and during full teardown.
        pub(super) fn alloc_owned_cstring(text: Option<&str>) -> u32 {
            match text.and_then(|t| CString::new(t).ok()) {
                Some(cstring) => cstring.into_raw() as u32,
                None => 0,
            }
        }

        pub(super) fn free_owned_cstring(ptr: u32) {
            if ptr != 0 {
                drop(unsafe { CString::from_raw(ptr as *mut c_char) });
            }
        }

        /// Reconstructs an owning `Vec<u32>` of raw pointers from a `ZTArray<T>`'s raw parts, or an
        /// empty `Vec` if the array was never allocated (`start_ptr == 0` - vanilla's own
        /// representation of an empty array; see e.g. `ZTResearchCategory::ZTResearchCategory`'s
        /// constructor decompile).
        pub(super) fn vec_from_ptr_array<T>(array: &ZTArray<T>) -> Vec<u32> {
            let (start, end, buffer_end) = array.raw_parts();
            if start == 0 {
                return Vec::new();
            }
            let len = ((end - start) / 4) as usize;
            let cap = ((buffer_end - start) / 4) as usize;
            unsafe { Vec::from_raw_parts(start as *mut u32, len, cap) }
        }

        /// Inverse of `vec_from_ptr_array`: leaks `vec`'s buffer into a fresh `ZTArray<T>` (all-zero
        /// raw parts if empty, matching vanilla's own empty-array representation).
        pub(super) fn ptr_array_from_vec<T>(mut vec: Vec<u32>) -> ZTArray<T> {
            if vec.is_empty() {
                return ZTArray::from_raw_parts(0, 0, 0);
            }
            let ptr = vec.as_mut_ptr() as u32;
            let len = vec.len() as u32;
            let cap = vec.capacity() as u32;
            std::mem::forget(vec);
            ZTArray::from_raw_parts(ptr, ptr + len * 4, ptr + cap * 4)
        }

        /// Fully frees a pointer array's own backing buffer (the pointers it contained must already
        /// have been destroyed by the caller - this only reclaims the array-of-pointers allocation
        /// itself). Used only by the full-teardown paths in `destruction`; the "reset in place, keep
        /// capacity" paths never call this.
        pub(super) fn free_ptr_array<T>(array: &ZTArray<T>) {
            let (start, _end, buffer_end) = array.raw_parts();
            if start == 0 {
                return;
            }
            let cap = ((buffer_end - start) / 4) as usize;
            drop(unsafe { Vec::<u32>::from_raw_parts(start as *mut u32, cap, cap) });
        }

        /// Leaks `vec` into `ZTResearchBranch`'s funding-table raw parts (all-zero if empty).
        pub(super) fn funding_table_from_vec(mut vec: Vec<ZTResearchFundingLevel>) -> (u32, u32, u32) {
            if vec.is_empty() {
                return (0, 0, 0);
            }
            let stride = size_of::<ZTResearchFundingLevel>() as u32;
            let ptr = vec.as_mut_ptr() as u32;
            let len = vec.len() as u32;
            let cap = vec.capacity() as u32;
            std::mem::forget(vec);
            (ptr, ptr + len * stride, ptr + cap * stride)
        }

        /// Frees a branch's funding-table buffer (the inline `ZTResearchFundingLevel` block, *not* a
        /// `ZTArray` of pointers - see the field's own doc comment on `ZTResearchBranch`).
        pub(super) fn free_funding_table(branch: &ZTResearchBranch) {
            let start = branch.funding_table_start;
            if start == 0 {
                return;
            }
            let stride = size_of::<ZTResearchFundingLevel>() as u32;
            let cap = ((branch.funding_table_capacity - start) / stride) as usize;
            drop(unsafe { Vec::<ZTResearchFundingLevel>::from_raw_parts(start as *mut ZTResearchFundingLevel, cap, cap) });
        }
    }

    /// Builds fresh `ZTResearchBranch`/`ZTResearchCategory`/`ZTResearchProgram` objects (and extends
    /// existing branches) directly from the parsed `Reimplemented*` values, entirely on Rust's own
    /// allocator. See the module doc comment above for why this replaces vanilla's `loadBranches`
    /// wholesale rather than calling into it.
    #[cfg(not(feature = "vanilla-research-config"))]
    mod construction {
        use super::{raw_mem::*, *};

        /// Builds a fresh program and populates every field from `reimpl`. Mirrors the tail of
        /// vanilla `ZTResearchProgram::loadProgram`, which always ends by calling `reset()` - for
        /// `UnlockEntity`/`BuildingUpgrade` effects that's what actually marks the target
        /// entity/building unavailable until the program completes (`setAvail`/`setBuildingUpgrade`);
        /// skipping it would silently leave every such target available from the start. `reset()` is
        /// left as a call into the original implementation (like `on_completion`/`pick_random_program`
        /// elsewhere in this file) since its downstream effects aren't independently reimplemented.
        fn construct_program(reimpl: &ReimplementedProgram) -> *mut ZTResearchProgram {
            let program = Box::new(ZTResearchProgram {
                config_file: BFConfigFile::default(),
                cached_name: alloc_buffer_string(&resolved(reimpl.name_id)),
                cached_desc: alloc_buffer_string(&resolved(reimpl.desc_id)),
                desc_id: reimpl.desc_id,
                icon_ptr: alloc_owned_cstring(reimpl.icon.as_deref()),
                entity_icon_ptr: alloc_owned_cstring(reimpl.entity_icon.as_deref()),
                id: reimpl.name_id,
                target_cost: reimpl.cost,
                current_progress: 0.0,
                priority: reimpl.order as u32,
                target_id: reimpl.target,
                effect_kind_raw: reimpl.effect,
                effect_param_0: reimpl.effect_params.0,
                effect_param_1: reimpl.effect_params.1,
                effect_param_2: reimpl.effect_params.2,
                help_id: reimpl.help_id,
            });
            let ptr = Box::into_raw(program);
            unsafe { (*ptr).reset() };
            ptr
        }

        /// Builds a fresh category, including its full `program_array`. `enabled` is always forced to
        /// `1`, matching `ZTResearchCategory::clearCategory` - called unconditionally as
        /// `loadCategory`'s first line in vanilla, so every loaded category ends up enabled regardless
        /// of anything in the `.cfg` (there is no `enabled=` key in this file format at all).
        fn construct_category(reimpl: &ReimplementedCategory) -> *mut ZTResearchCategory {
            let mut programs = Vec::with_capacity(reimpl.programs.len());
            for program in &reimpl.programs {
                programs.push(construct_program(program) as u32);
            }
            let category = Box::new(ZTResearchCategory {
                config_file: BFConfigFile::default(),
                id: reimpl.name_id,
                cached_name: alloc_buffer_string(&resolved(reimpl.name_id)),
                cached_desc: alloc_buffer_string(&resolved(reimpl.desc_id)),
                icon_ptr: alloc_owned_cstring(reimpl.icon.as_deref()),
                help_id: reimpl.help_id,
                expansion_id: reimpl.expansion_id,
                enabled: 1,
                pad2: [0; 3],
                program_array: ptr_array_from_vec(programs),
            });
            Box::into_raw(category)
        }

        /// Appends a freshly-constructed branch pointer to `mgr.branch_array`.
        fn append_branch(mgr: &mut ZTResearchMgr, ptr: u32) {
            let mut branches = vec_from_ptr_array(&mgr.branch_array);
            branches.push(ptr);
            mgr.branch_array = ptr_array_from_vec(branches);
        }

        /// Applies one manifest entry's parsed branch to `mgr`: reuses the existing branch object if
        /// one with this id already exists (matching vanilla's own id-based reuse in `loadBranches`),
        /// or allocates a fresh one and appends it to `branch_array` otherwise. Header fields and the
        /// funding table are overwritten every call; `category_array` is append-only across repeated
        /// calls on the same branch - see `compare_branch`'s doc comment (the shadow-mode arm, above)
        /// for how this was confirmed against the live game.
        fn apply_branch(mgr: &mut ZTResearchMgr, reimpl: &ReimplementedBranch) {
            let id = reimpl.name_id;
            // Materialize the lookup into a plain (borrow-free) raw pointer before matching on it -
            // `branches_mut()`'s returned iterator otherwise keeps `mgr` immutably borrowed for the
            // whole `match`, conflicting with the `None` arm's `append_branch(mgr, ...)`.
            let found: Option<*mut ZTResearchBranch> = mgr.branches_mut().find(|b| b.id() == id).map(|b| b as *mut ZTResearchBranch);
            let ptr: *mut ZTResearchBranch = match found {
                Some(existing) => existing,
                None => {
                    let branch = Box::new(ZTResearchBranch {
                        config_file: BFConfigFile::default(),
                        id,
                        cached_name: alloc_buffer_string(""),
                        cached_desc: alloc_buffer_string(""),
                        icon_ptr: 0,
                        noprogicon_ptr: 0,
                        current_category_ptr: 0,
                        current_program_ptr: 0,
                        category_array: ZTArray::from_raw_parts(0, 0, 0),
                        current_funding_level: 0,
                        funding_table_start: 0,
                        funding_table_end: 0,
                        funding_table_capacity: 0,
                    });
                    let ptr = Box::into_raw(branch);
                    append_branch(mgr, ptr as u32);
                    ptr
                }
            };

            let branch = unsafe { &mut *ptr };

            branch.id = id;
            free_buffer_string(&branch.cached_name);
            branch.cached_name = alloc_buffer_string(&resolved(reimpl.name_id));
            free_buffer_string(&branch.cached_desc);
            branch.cached_desc = alloc_buffer_string(&resolved(reimpl.desc_id));
            free_owned_cstring(branch.icon_ptr);
            branch.icon_ptr = alloc_owned_cstring(reimpl.icon.as_deref());
            free_owned_cstring(branch.noprogicon_ptr);
            branch.noprogicon_ptr = alloc_owned_cstring(reimpl.noprogicon.as_deref());

            let mut categories = vec_from_ptr_array(&branch.category_array);
            for category in &reimpl.categories {
                categories.push(construct_category(category) as u32);
            }
            branch.category_array = ptr_array_from_vec(categories);

            free_funding_table(branch);
            let mut funding = Vec::with_capacity(reimpl.funding.len());
            for level in &reimpl.funding {
                funding.push(ZTResearchFundingLevel { name_id: level.name_id, rate: level.rate, cost: level.cost });
            }
            let (start, end, capacity) = funding_table_from_vec(funding);
            branch.funding_table_start = start;
            branch.funding_table_end = end;
            branch.funding_table_capacity = capacity;
            branch.current_funding_level = 0;

            // Mirrors the tail of vanilla `ZTResearchBranch::loadBranch`, which always ends by
            // calling `pickRandomProgram` - left as a call into the original implementation since it
            // consumes the game's RNG stream (see `pick_random_program`'s doc comment elsewhere in
            // this file for why reimplementing it natively risks desyncing that stream).
            branch.pick_random_program();
        }

        /// Parses every manifest entry fully, touching nothing but local state - no `ZTResearchMgr`
        /// access at all, deliberately, so this is always safe to fall back from (a parse failure or a
        /// panic here can never leave `mgr` partially mutated). `apply_all` below is the mutating half,
        /// and is *not* safe to fall back from once it's started - see its own doc comment.
        pub(super) fn parse_manifest(manifest_path: &str) -> Option<Vec<ReimplementedBranch>> {
            let manifest_ini = read_cfg(manifest_path)?;
            let paths = load_manifest(&manifest_ini);

            let mut parsed = Vec::with_capacity(paths.len());
            for path in &paths {
                let ini = read_cfg(path)?;
                parsed.push(load_branch(&ini));
            }
            Some(parsed)
        }

        /// Splices every already-parsed branch into `mgr`. Only ever called after `parse_manifest`
        /// has fully succeeded, so this itself can't "fail" in the ordinary sense - the only way it
        /// stops partway through is a panic, which the caller must *not* respond to by falling back to
        /// vanilla (see the module doc comment above: `mgr` may already be partially mutated with
        /// Rust-allocated objects by that point, and vanilla's `loadBranches` has no way to know that -
        /// it would either reuse/extend our still-being-built branch mid-construction or duplicate
        /// work already done, neither of which is safe).
        pub(super) fn apply_all(mgr: &mut ZTResearchMgr, parsed: &[ReimplementedBranch]) {
            for reimpl in parsed {
                apply_branch(mgr, reimpl);
            }
        }
    }

    /// Faithful translations of the decompiled `clearBranch`/`clearCategory`/`~ZTResearchBranch`/
    /// `~ZTResearchCategory`/`~ZTResearchProgram`/`clearBranches` into Rust, using `Box`/`Vec`
    /// reclamation instead of vanilla's free helpers (see the module doc comment above). Every
    /// `loaded`/`tree_root`-gated branch in those decompiles is skipped here on purpose: every object
    /// this module ever constructs (`construction`, above) leaves both fields at 0 forever, so those
    /// branches never fire on real vanilla objects and would be actively unsafe to run against ours
    /// (vanilla's `BFConfigFile::release` dereferences `tree_root` unconditionally).
    #[cfg(not(feature = "vanilla-research-config"))]
    mod destruction {
        use super::{raw_mem::*, *};

        /// Calls `destroy` on every non-null pointer in a raw `start_ptr..end_ptr` pointer array
        /// (stride 4) without touching the array's own backing buffer - the "iterate and free
        /// contents, keep capacity" half shared by every `reset_*_contents`/`clear_branches` below.
        fn destroy_each_ptr<T>(start_ptr: u32, end_ptr: u32, mut destroy: impl FnMut(*mut T)) {
            let mut addr = start_ptr;
            while addr < end_ptr {
                let ptr = get_from_memory::<u32>(addr) as *mut T;
                if !ptr.is_null() {
                    destroy(ptr);
                }
                addr += 4;
            }
        }

        /// Full teardown of a program - there's no "clear in place" for programs, only full destroy
        /// (see `ZTResearchProgram::~ZTResearchProgram`'s decompile).
        pub(super) unsafe fn destroy_program(ptr: *mut ZTResearchProgram) {
            if ptr.is_null() {
                return;
            }
            let program = unsafe { &*ptr };
            free_buffer_string(&program.cached_name);
            free_buffer_string(&program.cached_desc);
            free_owned_cstring(program.icon_ptr);
            free_owned_cstring(program.entity_icon_ptr);
            drop(unsafe { Box::from_raw(ptr) });
        }

        /// Mirrors `ZTResearchCategory::clearCategory`: resets the category to an empty, disabled-id
        /// (but still `enabled`) state in place, destroying every program but keeping
        /// `program_array`'s own backing buffer.
        pub(super) fn reset_category_contents(category: &mut ZTResearchCategory) {
            category.id = -1;
            free_buffer_string(&category.cached_name);
            category.cached_name = alloc_buffer_string("");
            free_buffer_string(&category.cached_desc);
            category.cached_desc = alloc_buffer_string("");
            category.enabled = 1;
            free_owned_cstring(category.icon_ptr);
            category.icon_ptr = 0;
            category.help_id = 0;
            category.expansion_id = 0;
            let (start, end, buffer_end) = category.program_array.raw_parts();
            destroy_each_ptr::<ZTResearchProgram>(start, end, |p| unsafe { destroy_program(p) });
            category.program_array = ZTArray::from_raw_parts(start, start, buffer_end);
        }

        /// Mirrors `ZTResearchCategory::~ZTResearchCategory`: `reset_category_contents` then frees
        /// `program_array`/`cached_desc`/`cached_name`'s own buffers and the category object itself.
        pub(super) unsafe fn destroy_category(ptr: *mut ZTResearchCategory) {
            if ptr.is_null() {
                return;
            }
            let category = unsafe { &mut *ptr };
            reset_category_contents(category);
            free_ptr_array(&category.program_array);
            free_buffer_string(&category.cached_desc);
            free_buffer_string(&category.cached_name);
            drop(unsafe { Box::from_raw(ptr) });
        }

        /// Mirrors `ZTResearchBranch::clearBranch`: resets the branch to an empty, id `-1` state in
        /// place, destroying every category but keeping `category_array`'s own backing buffer, and
        /// emptying (but not freeing) the funding table.
        pub(super) fn reset_branch_contents(branch: &mut ZTResearchBranch) {
            branch.id = -1;
            free_buffer_string(&branch.cached_name);
            branch.cached_name = alloc_buffer_string("");
            free_buffer_string(&branch.cached_desc);
            branch.cached_desc = alloc_buffer_string("");
            free_owned_cstring(branch.icon_ptr);
            branch.icon_ptr = 0;
            free_owned_cstring(branch.noprogicon_ptr);
            branch.noprogicon_ptr = 0;
            branch.current_category_ptr = 0;
            branch.current_program_ptr = 0;
            let (start, end, buffer_end) = branch.category_array.raw_parts();
            destroy_each_ptr::<ZTResearchCategory>(start, end, |p| unsafe { destroy_category(p) });
            branch.category_array = ZTArray::from_raw_parts(start, start, buffer_end);
            branch.current_funding_level = 0;
            branch.funding_table_end = branch.funding_table_start;
        }

        /// Mirrors `ZTResearchBranch::~ZTResearchBranch`: `reset_branch_contents` then frees the
        /// funding table/`category_array`/`cached_desc`/`cached_name`'s own buffers and the branch
        /// object itself.
        pub(super) unsafe fn destroy_branch(ptr: *mut ZTResearchBranch) {
            if ptr.is_null() {
                return;
            }
            let branch = unsafe { &mut *ptr };
            reset_branch_contents(branch);
            free_funding_table(branch);
            free_ptr_array(&branch.category_array);
            free_buffer_string(&branch.cached_desc);
            free_buffer_string(&branch.cached_name);
            drop(unsafe { Box::from_raw(ptr) });
        }

        /// Mirrors `ZTResearchMgr::clearBranches`: destroys and frees every branch, then resets
        /// `branch_array` to empty (keeping its own backing buffer) and zeroes `elapsed_ticks`.
        pub(super) fn clear_branches(mgr: &mut ZTResearchMgr) {
            let (start, end, buffer_end) = mgr.branch_array.raw_parts();
            destroy_each_ptr::<ZTResearchBranch>(start, end, |p| unsafe { destroy_branch(p) });
            mgr.branch_array = ZTArray::from_raw_parts(start, start, buffer_end);
            mgr.elapsed_ticks = 0;
        }
    }

    /// Shadow-mode arm: only `loadBranches` observes/compares; the six lifecycle functions are plain
    /// passthroughs to vanilla (there's nothing to shadow-test on the destruction side - see the module
    /// doc comment above).
    #[cfg(feature = "vanilla-research-config")]
    #[detour_mod]
    mod detours {
        use std::{collections::HashSet, ffi::CStr};

        use openzt_detour::generated::{
            ztresearchbranch::{CLEAR_BRANCH, ZTRESEARCH_BRANCH},
            ztresearchcategory::{CLEAR_CATEGORY, ZTRESEARCH_CATEGORY},
            ztresearchmgr::{CLEAR_BRANCHES, LOAD_BRANCHES},
            ztresearchprogram::ZTRESEARCH_PROGRAM,
        };

        use super::*;
        use crate::util::ref_from_memory;

        #[detour(LOAD_BRANCHES)]
        unsafe extern "thiscall" fn load_branches(this: *const u32, manifest_path: *const i8) -> u32 {
            let manifest_path_str = unsafe { CStr::from_ptr(manifest_path) }.to_string_lossy().into_owned();

            // Read the manifest ourselves before calling the original, so we know which branch files
            // (and, by peeking each one's own declared id, which existing-vs-new outcome) to expect -
            // independent of vanilla, matching this module's usual "compute a prediction, then check it
            // against what actually happened" shape.
            let (ids_before, entries) = match super::read_cfg(&manifest_path_str) {
                Some(manifest_ini) => {
                    let mgr_before = unsafe { ref_from_memory::<ZTResearchMgr>(this) };
                    let ids_before: HashSet<i32> = mgr_before.branches().map(|b| b.id()).collect();
                    let entries = super::manifest_entries(&manifest_ini, mgr_before);
                    (Some(ids_before), Some(entries))
                }
                None => {
                    error!("research-config-reimplementation: failed to independently parse manifest '{manifest_path_str}'");
                    (None, None)
                }
            };

            let result = unsafe { LOAD_BRANCHES_DETOUR.call(this, manifest_path) };
            debug!("research-config-reimplementation: ZTResearchMgr::loadBranches(\"{manifest_path_str}\") called, this={:#x}, vanilla result: {result}", this as u32);

            if result == 0 {
                return result;
            }
            let (Some(ids_before), Some(entries)) = (ids_before, entries) else {
                return result;
            };

            let mgr_after = unsafe { ref_from_memory::<ZTResearchMgr>(this) };
            let mut mismatches = Vec::new();
            super::compare_load_branches(&manifest_path_str, &ids_before, mgr_after, &entries, &mut mismatches);
            if mismatches.is_empty() {
                if !entries.is_empty() {
                    info!(
                        "research-config-reimplementation: '{manifest_path_str}' matches reimplementation ({} branches in manifest)",
                        entries.len()
                    );
                } else {
                    debug!("research-config-reimplementation: '{manifest_path_str}' matches reimplementation (0 branches in manifest)");
                }
            } else {
                for mismatch in &mismatches {
                    error!("research-config-reimplementation mismatch: {mismatch}");
                }
            }

            result
        }

        #[detour(CLEAR_BRANCHES)]
        unsafe extern "thiscall" fn clear_branches(this: *const u32) {
            unsafe { CLEAR_BRANCHES_DETOUR.call(this) }
        }

        #[detour(CLEAR_BRANCH)]
        unsafe extern "thiscall" fn clear_branch(this: *const u32) {
            unsafe { CLEAR_BRANCH_DETOUR.call(this) }
        }

        #[detour(ZTRESEARCH_BRANCH)]
        unsafe extern "thiscall" fn ztresearch_branch_dtor(this: *const u32) {
            unsafe { ZTRESEARCH_BRANCH_DETOUR.call(this) }
        }

        #[detour(CLEAR_CATEGORY)]
        unsafe extern "thiscall" fn clear_category(this: *const u32) {
            unsafe { CLEAR_CATEGORY_DETOUR.call(this) }
        }

        #[detour(ZTRESEARCH_CATEGORY)]
        unsafe extern "thiscall" fn ztresearch_category_dtor(this: *const u32) {
            unsafe { ZTRESEARCH_CATEGORY_DETOUR.call(this) }
        }

        #[detour(ZTRESEARCH_PROGRAM)]
        unsafe extern "thiscall" fn ztresearch_program_dtor(this: *const u32) {
            unsafe { ZTRESEARCH_PROGRAM_DETOUR.call(this) }
        }
    }

    /// Default arm: full construction/destruction replacement - see the module doc comment above.
    #[cfg(not(feature = "vanilla-research-config"))]
    #[detour_mod]
    mod detours {
        use std::{ffi::CStr, panic::AssertUnwindSafe};

        use openzt_detour::generated::{
            ztresearchbranch::{CLEAR_BRANCH, ZTRESEARCH_BRANCH},
            ztresearchcategory::{CLEAR_CATEGORY, ZTRESEARCH_CATEGORY},
            ztresearchmgr::{CLEAR_BRANCHES, LOAD_BRANCHES},
            ztresearchprogram::ZTRESEARCH_PROGRAM,
        };

        use super::*;
        use crate::util::mut_from_memory;

        #[detour(LOAD_BRANCHES)]
        unsafe extern "thiscall" fn load_branches(this: *const u32, manifest_path: *const i8) -> u32 {
            let manifest_path_str = unsafe { CStr::from_ptr(manifest_path) }.to_string_lossy().into_owned();
            debug!("research-config-reimplementation: loadBranches(\"{manifest_path_str}\") called, this={:#x}", this as u32);

            // Parsing touches no `ZTResearchMgr` state at all, so a parse failure or panic here can
            // never leave `mgr` partially mutated - always safe to fall back to vanilla.
            let parsed = match std::panic::catch_unwind(AssertUnwindSafe(|| super::construction::parse_manifest(&manifest_path_str))) {
                Ok(Some(parsed)) => parsed,
                Ok(None) | Err(_) => {
                    error!("research-config-reimplementation: failed to parse '{manifest_path_str}', falling back to vanilla");
                    return unsafe { LOAD_BRANCHES_DETOUR.call(this, manifest_path) };
                }
            };

            // Applying mutates `mgr` directly - once it's started, `mgr` may already contain
            // partially-constructed Rust-allocated branches, so a panic here must NOT fall back to
            // vanilla (see `apply_all`'s doc comment): that would mean vanilla re-processing the same
            // manifest against a `mgr` it doesn't know is already half-updated.
            let mgr = unsafe { mut_from_memory::<ZTResearchMgr>(this) };
            let branch_count = parsed.len();
            if std::panic::catch_unwind(AssertUnwindSafe(|| super::construction::apply_all(mgr, &parsed))).is_err() {
                error!("research-config-reimplementation: panic while applying '{manifest_path_str}'; branch_array may be partially updated, not falling back");
            } else if branch_count > 0 {
                info!("research-config-reimplementation: replaced loadBranches(\"{manifest_path_str}\") natively ({branch_count} branches in manifest)");
            } else {
                debug!("research-config-reimplementation: replaced loadBranches(\"{manifest_path_str}\") natively (0 branches in manifest)");
            }
            1
        }

        #[detour(CLEAR_BRANCHES)]
        unsafe extern "thiscall" fn clear_branches(this: *const u32) {
            let mgr = unsafe { mut_from_memory::<ZTResearchMgr>(this) };
            if std::panic::catch_unwind(AssertUnwindSafe(|| super::destruction::clear_branches(mgr))).is_err() {
                error!("research-config-reimplementation: panic while clearing branches; memory may be leaked");
            }
        }

        #[detour(CLEAR_BRANCH)]
        unsafe extern "thiscall" fn clear_branch(this: *const u32) {
            let branch = unsafe { mut_from_memory::<ZTResearchBranch>(this) };
            if std::panic::catch_unwind(AssertUnwindSafe(|| super::destruction::reset_branch_contents(branch))).is_err() {
                error!("research-config-reimplementation: panic while resetting branch contents; memory may be leaked");
            }
        }

        #[detour(ZTRESEARCH_BRANCH)]
        unsafe extern "thiscall" fn ztresearch_branch_dtor(this: *const u32) {
            let ptr = this as *mut ZTResearchBranch;
            if std::panic::catch_unwind(AssertUnwindSafe(|| unsafe { super::destruction::destroy_branch(ptr) })).is_err() {
                error!("research-config-reimplementation: panic while destroying branch; memory may be leaked");
            }
        }

        #[detour(CLEAR_CATEGORY)]
        unsafe extern "thiscall" fn clear_category(this: *const u32) {
            let category = unsafe { mut_from_memory::<ZTResearchCategory>(this) };
            if std::panic::catch_unwind(AssertUnwindSafe(|| super::destruction::reset_category_contents(category))).is_err() {
                error!("research-config-reimplementation: panic while resetting category contents; memory may be leaked");
            }
        }

        #[detour(ZTRESEARCH_CATEGORY)]
        unsafe extern "thiscall" fn ztresearch_category_dtor(this: *const u32) {
            let ptr = this as *mut ZTResearchCategory;
            if std::panic::catch_unwind(AssertUnwindSafe(|| unsafe { super::destruction::destroy_category(ptr) })).is_err() {
                error!("research-config-reimplementation: panic while destroying category; memory may be leaked");
            }
        }

        #[detour(ZTRESEARCH_PROGRAM)]
        unsafe extern "thiscall" fn ztresearch_program_dtor(this: *const u32) {
            let ptr = this as *mut ZTResearchProgram;
            if std::panic::catch_unwind(AssertUnwindSafe(|| unsafe { super::destruction::destroy_program(ptr) })).is_err() {
                error!("research-config-reimplementation: panic while destroying program; memory may be leaked");
            }
        }
    }

    pub fn init() {
        if let Err(e) = unsafe { detours::init_detours() } {
            error!("Failed to initialise research-config-reimplementation detours: {e:?}");
        }
    }
}

/// Native reimplementation of `ZTResearchMgr::save`/`load`'s save-file persistence format - a small,
/// self-describing stream of `(kind, id, value)` tuples capturing each branch's `current_funding_level`,
/// each category's `enabled` flag, and each program's `current_progress`. Confirmed byte-for-byte from
/// `resources/decompiles/ZTResearchMgr_save.c` and cross-checked against `ZTResearchMgr_load.c`/`.asm`.
///
/// This is **shadow-mode only**: nothing here ever runs during real save/load (`ZTResearchMgr::save`/
/// `load` are not detoured), it exists purely to be exercised by the live proptest-vs-`.original()`
/// comparison in `reimplementation_tests` (see `live_support`, gated by the `reimplementation-tests`
/// feature).
///
/// `load`'s actual behavior is considerably more than "read the stream and apply it" - see
/// `predict_load`'s doc comment - including two side effects deliberately **not** modeled or compared
/// here: `ZTResearchProgram::on_completion()` (called on any program whose `current_progress` ends up
/// `>= target_cost`) and `ZTResearchBranch::pick_random_program()` (called on every branch, consuming
/// the game's RNG stream). Both are already treated elsewhere in this file as too complex/consequential
/// to reimplement (see `ZTResearchProgram::on_completion`/`ZTResearchBranch::pick_random_program`'s own
/// doc comments) - `live_support` neutralizes `on_completion` for its synthetic programs by fixing
/// `effect_kind_raw` to an always-unset value (see `live_support::build_program`) rather than trying to
/// predict or avoid it structurally.
pub(crate) mod research_save_reimplementation {
    use std::collections::HashMap;

    use super::*;

    /// One `(kind, id, value)` tuple from the save stream. `Program`'s value is stored as raw `f32`
    /// bits (not `f32` itself) so records - and the `PartialEq`/`HashMap` machinery used to compare
    /// them - don't have to special-case `NaN != NaN`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum SaveRecord {
        Branch { id: i32, current_funding_level: i32 },
        Category { id: i32, enabled: u8 },
        Program { id: i32, current_progress_bits: u32 },
    }

    /// Walks a manager's `branch_array`/`category_array`/`program_array` in the same nested order
    /// `ZTResearchMgr::save` does (branch, then each of its categories, then each category's
    /// programs), producing one record per branch/category/program.
    pub(crate) fn snapshot_mgr(mgr: &ZTResearchMgr) -> Vec<SaveRecord> {
        let mut records = Vec::new();
        for branch in mgr.branches() {
            records.push(SaveRecord::Branch { id: branch.id, current_funding_level: branch.current_funding_level });
            for category in branch.categories() {
                records.push(SaveRecord::Category { id: category.id, enabled: category.enabled });
                for program in category.programs() {
                    records.push(SaveRecord::Program { id: program.id, current_progress_bits: program.current_progress.to_bits() });
                }
            }
        }
        records
    }

    /// The exact byte stream `ZTResearchMgr::save` writes for `records`: a leading `int32 0` header, one
    /// `(kind, id, value)` tuple per record (`kind` `0`/`1`/`2` for `Branch`/`Category`/`Program`; a
    /// `Category`'s `enabled` is a single byte, everything else is a little-endian `int32`/`float32`),
    /// and a trailing `int32 -1` terminator. No counts are ever written - the stream is fully
    /// self-describing via the `kind` tag.
    pub(crate) fn serialize(records: &[SaveRecord]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + records.len() * 12);
        bytes.extend_from_slice(&0i32.to_le_bytes());
        for record in records {
            match *record {
                SaveRecord::Branch { id, current_funding_level } => {
                    bytes.extend_from_slice(&0i32.to_le_bytes());
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.extend_from_slice(&current_funding_level.to_le_bytes());
                }
                SaveRecord::Category { id, enabled } => {
                    bytes.extend_from_slice(&1i32.to_le_bytes());
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.push(enabled);
                }
                SaveRecord::Program { id, current_progress_bits } => {
                    bytes.extend_from_slice(&2i32.to_le_bytes());
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.extend_from_slice(&current_progress_bits.to_le_bytes());
                }
            }
        }
        bytes.extend_from_slice(&(-1i32).to_le_bytes());
        bytes
    }

    fn read_i32(bytes: &[u8], cursor: &mut usize) -> Option<i32> {
        let chunk = bytes.get(*cursor..*cursor + 4)?;
        *cursor += 4;
        Some(i32::from_le_bytes(chunk.try_into().unwrap()))
    }

    /// The inverse of `serialize`. Returns `None` on any malformed stream: truncated (a size/count
    /// doesn't fit), an unrecognized `kind` (anything other than `0`/`1`/`2`/the `-1` terminator), or
    /// trailing bytes left over after the terminator.
    pub(crate) fn parse(bytes: &[u8]) -> Option<Vec<SaveRecord>> {
        let mut cursor = 0usize;
        read_i32(bytes, &mut cursor)?; // header, discarded - matches `load`, which reads but never uses it
        let mut records = Vec::new();
        loop {
            let kind = read_i32(bytes, &mut cursor)?;
            if kind < 0 {
                break;
            }
            let id = read_i32(bytes, &mut cursor)?;
            match kind {
                0 => {
                    let current_funding_level = read_i32(bytes, &mut cursor)?;
                    records.push(SaveRecord::Branch { id, current_funding_level });
                }
                1 => {
                    let enabled = *bytes.get(cursor)?;
                    cursor += 1;
                    records.push(SaveRecord::Category { id, enabled });
                }
                2 => {
                    let current_progress_bits = read_i32(bytes, &mut cursor)? as u32;
                    records.push(SaveRecord::Program { id, current_progress_bits });
                }
                _ => return None,
            }
        }
        (cursor == bytes.len()).then_some(records)
    }

    /// The funding level/enabled flag/current progress `ZTResearchMgr::load` ends up with for every
    /// branch/category/program id it knows about (not just the ones a stream record touched).
    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct PredictedState {
        pub(crate) funding_levels: HashMap<i32, i32>,
        pub(crate) enabled: HashMap<i32, u8>,
        pub(crate) progress_bits: HashMap<i32, u32>,
    }

    /// Predicts what `ZTResearchMgr::load` leaves in `current_funding_level`/`enabled`/
    /// `current_progress` (excluding the `on_completion`/`pick_random_program` side effects - see the
    /// module doc comment above), given the ids it already knows about and the stream's records.
    ///
    /// Confirmed from `ZTResearchMgr_load.c`/`.asm`: **every** branch/category/program is reset first,
    /// unconditionally (`current_funding_level = 0`, `enabled = 1`, `current_progress = 0.0`),
    /// completely independent of `version` or the stream (this happens before either is even looked
    /// at). Only if `version >= 0x28` is the stream then read and applied on top, record by record, in
    /// order; a record whose `id` doesn't match any known branch/category/program is silently skipped
    /// (mirrors `getBranch`/`getCategory`/`getProgram` returning null). A `Branch` record's value is
    /// clamped to `0` if it's `>=` that branch's own funding-level count, using an **unsigned**
    /// comparison (so a negative saved value is always clamped, per the decompiled
    /// `-(uint)(value < count) & value`).
    pub(crate) fn predict_load(
        branch_ids: &[i32],
        category_ids: &[i32],
        program_ids: &[i32],
        funding_level_counts: &HashMap<i32, usize>,
        records: &[SaveRecord],
        version: u32,
    ) -> PredictedState {
        let mut funding_levels: HashMap<i32, i32> = branch_ids.iter().map(|&id| (id, 0)).collect();
        let mut enabled: HashMap<i32, u8> = category_ids.iter().map(|&id| (id, 1)).collect();
        let mut progress_bits: HashMap<i32, u32> = program_ids.iter().map(|&id| (id, 0.0f32.to_bits())).collect();

        const MIN_VERSION_WITH_RESEARCH_DATA: u32 = 0x28;
        if version >= MIN_VERSION_WITH_RESEARCH_DATA {
            for record in records {
                match *record {
                    SaveRecord::Branch { id, current_funding_level } => {
                        if let Some(slot) = funding_levels.get_mut(&id) {
                            let count = funding_level_counts.get(&id).copied().unwrap_or(0) as u32;
                            *slot = if (current_funding_level as u32) < count { current_funding_level } else { 0 };
                        }
                    }
                    SaveRecord::Category { id, enabled: value } => {
                        if let Some(slot) = enabled.get_mut(&id) {
                            *slot = value;
                        }
                    }
                    SaveRecord::Program { id, current_progress_bits } => {
                        if let Some(slot) = progress_bits.get_mut(&id) {
                            *slot = current_progress_bits;
                        }
                    }
                }
            }
        }

        PredictedState { funding_levels, enabled, progress_bits }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn serialize_parse_round_trip() {
            let records = vec![
                SaveRecord::Branch { id: 100, current_funding_level: 2 },
                SaveRecord::Category { id: 200, enabled: 1 },
                SaveRecord::Program { id: 300, current_progress_bits: 12.5f32.to_bits() },
                SaveRecord::Branch { id: -5, current_funding_level: -1 },
                SaveRecord::Category { id: i32::MAX, enabled: 0 },
                SaveRecord::Program { id: i32::MIN, current_progress_bits: f32::NAN.to_bits() },
            ];
            let bytes = serialize(&records);
            assert_eq!(parse(&bytes), Some(records));
        }

        #[test]
        fn empty_record_list_round_trips() {
            let bytes = serialize(&[]);
            assert_eq!(bytes.len(), 8); // header + terminator only
            assert_eq!(parse(&bytes), Some(vec![]));
        }

        #[test]
        fn parse_rejects_truncated_stream() {
            let mut bytes = serialize(&[SaveRecord::Branch { id: 1, current_funding_level: 2 }]);
            bytes.pop();
            assert_eq!(parse(&bytes), None);
        }

        #[test]
        fn parse_rejects_unknown_kind() {
            let mut bytes = 0i32.to_le_bytes().to_vec();
            bytes.extend_from_slice(&3i32.to_le_bytes()); // no such kind
            bytes.extend_from_slice(&0i32.to_le_bytes());
            assert_eq!(parse(&bytes), None);
        }

        #[test]
        fn parse_rejects_trailing_garbage() {
            let mut bytes = serialize(&[]);
            bytes.push(0xff);
            assert_eq!(parse(&bytes), None);
        }

        #[test]
        fn predict_load_resets_untouched_ids_and_applies_matching_records() {
            let funding_level_counts = HashMap::from([(1, 3usize)]);
            let records = vec![
                SaveRecord::Branch { id: 1, current_funding_level: 2 },
                SaveRecord::Branch { id: 1, current_funding_level: 99 }, // out of range: clamps to 0, and overrides the record above
                SaveRecord::Category { id: 10, enabled: 0 },
                SaveRecord::Program { id: 100, current_progress_bits: 5.0f32.to_bits() },
                SaveRecord::Branch { id: 999, current_funding_level: 1 }, // no matching branch: ignored
            ];
            let predicted = predict_load(&[1, 2], &[10, 20], &[100], &funding_level_counts, &records, 0x28);
            assert_eq!(predicted.funding_levels, HashMap::from([(1, 0), (2, 0)]));
            assert_eq!(predicted.enabled, HashMap::from([(10, 0), (20, 1)]));
            assert_eq!(predicted.progress_bits, HashMap::from([(100, 5.0f32.to_bits())]));
        }

        #[test]
        fn predict_load_below_version_threshold_only_resets() {
            let funding_level_counts = HashMap::from([(1, 3usize)]);
            let records = vec![SaveRecord::Branch { id: 1, current_funding_level: 2 }];
            let predicted = predict_load(&[1], &[], &[], &funding_level_counts, &records, 0x27);
            assert_eq!(predicted.funding_levels, HashMap::from([(1, 0)]));
        }
    }

    /// Synthetic `ZTResearchBranch`/`ZTResearchCategory`/`ZTResearchProgram` construction/teardown for
    /// the live `reimplementation_tests` comparison harness. Deliberately **not** shared with
    /// `research_config_reimplementation::construction`/`raw_mem` above - that module is compiled only
    /// under `not(vanilla-research-config)`, the opposite of this file's usual feature gate, and reusing
    /// it here would tie this test-only code to an unrelated feature flag. Every allocation here goes
    /// through Rust's own allocator, mirroring that module's own rationale for doing the same.
    #[cfg(feature = "reimplementation-tests")]
    pub(crate) mod live_support {
        use super::*;

        /// A program to splice into a synthetic category. `effect_kind_raw` is always fixed to `-1`
        /// (unset) rather than generated - see the module doc comment above: this is what keeps
        /// `ZTResearchProgram::on_completion()` (triggered by `load` whenever `current_progress` ends
        /// up `>= target_cost`) a guaranteed no-op, regardless of what values a test case generates for
        /// `current_progress`/`target_cost`, instead of risking a dispatch into `setAvail`/
        /// `setBuildingUpgrade`/etc. with ids that don't correspond to any real entity.
        pub(crate) struct GeneratedProgram {
            pub(crate) id: i32,
            pub(crate) target_cost: f32,
            pub(crate) current_progress: f32,
        }

        /// A category to splice into a synthetic branch. `expansion_id` is always fixed to `0` (the
        /// common "no specific expansion" default real `.cfg` categories use) rather than generated,
        /// since `pick_random_program` - called on every branch as part of `load`'s tail, see the
        /// module doc comment above - passes it to `ZTUI::expansionselect::isExpansionDisabled`, and an
        /// arbitrary generated id risks indexing that lookup out of bounds.
        pub(crate) struct GeneratedCategory {
            pub(crate) id: i32,
            pub(crate) enabled: u8,
            pub(crate) programs: Vec<GeneratedProgram>,
        }

        /// A branch to splice into `ZTResearchMgr::branch_array`. `funding_level_count` controls how
        /// many (dummy) entries the branch's funding table has, to exercise `predict_load`'s clamp rule;
        /// the entries' own content is never read by `save`/`load`, only the table's size.
        pub(crate) struct GeneratedBranch {
            pub(crate) id: i32,
            pub(crate) current_funding_level: i32,
            pub(crate) funding_level_count: usize,
            pub(crate) categories: Vec<GeneratedCategory>,
        }

        fn ptr_array_from_vec<T>(mut vec: Vec<u32>) -> ZTArray<T> {
            if vec.is_empty() {
                return ZTArray::from_raw_parts(0, 0, 0);
            }
            let ptr = vec.as_mut_ptr() as u32;
            let len = vec.len() as u32;
            let cap = vec.capacity() as u32;
            std::mem::forget(vec);
            ZTArray::from_raw_parts(ptr, ptr + len * 4, ptr + cap * 4)
        }

        fn vec_from_ptr_array<T>(array: &ZTArray<T>) -> Vec<u32> {
            let (start, end, buffer_end) = array.raw_parts();
            if start == 0 {
                return Vec::new();
            }
            let len = ((end - start) / 4) as usize;
            let cap = ((buffer_end - start) / 4) as usize;
            unsafe { Vec::from_raw_parts(start as *mut u32, len, cap) }
        }

        fn funding_table_from_vec(mut vec: Vec<ZTResearchFundingLevel>) -> (u32, u32, u32) {
            if vec.is_empty() {
                return (0, 0, 0);
            }
            let stride = size_of::<ZTResearchFundingLevel>() as u32;
            let ptr = vec.as_mut_ptr() as u32;
            let len = vec.len() as u32;
            let cap = vec.capacity() as u32;
            std::mem::forget(vec);
            (ptr, ptr + len * stride, ptr + cap * stride)
        }

        fn free_funding_table(start: u32, capacity_end: u32) {
            if start == 0 {
                return;
            }
            let stride = size_of::<ZTResearchFundingLevel>() as u32;
            let cap = ((capacity_end - start) / stride) as usize;
            drop(unsafe { Vec::<ZTResearchFundingLevel>::from_raw_parts(start as *mut ZTResearchFundingLevel, cap, cap) });
        }

        fn build_program(spec: &GeneratedProgram) -> *mut ZTResearchProgram {
            let program = Box::new(ZTResearchProgram {
                config_file: BFConfigFile::default(),
                cached_name: ZTBufferString::from_raw_parts(0, 0, 0),
                cached_desc: ZTBufferString::from_raw_parts(0, 0, 0),
                desc_id: 0,
                icon_ptr: 0,
                entity_icon_ptr: 0,
                id: spec.id,
                target_cost: spec.target_cost,
                current_progress: spec.current_progress,
                priority: 0,
                target_id: -1,
                effect_kind_raw: -1,
                effect_param_0: 0,
                effect_param_1: 0,
                effect_param_2: 0,
                help_id: 0,
            });
            Box::into_raw(program)
        }

        fn destroy_program(ptr: *mut ZTResearchProgram) {
            if ptr.is_null() {
                return;
            }
            drop(unsafe { Box::from_raw(ptr) });
        }

        fn build_category(spec: &GeneratedCategory) -> *mut ZTResearchCategory {
            let mut programs = Vec::with_capacity(spec.programs.len());
            for program in &spec.programs {
                programs.push(build_program(program) as u32);
            }
            let category = Box::new(ZTResearchCategory {
                config_file: BFConfigFile::default(),
                id: spec.id,
                cached_name: ZTBufferString::from_raw_parts(0, 0, 0),
                cached_desc: ZTBufferString::from_raw_parts(0, 0, 0),
                icon_ptr: 0,
                help_id: 0,
                expansion_id: 0,
                enabled: spec.enabled,
                pad2: [0; 3],
                program_array: ptr_array_from_vec(programs),
            });
            Box::into_raw(category)
        }

        fn destroy_category(ptr: *mut ZTResearchCategory) {
            if ptr.is_null() {
                return;
            }
            let category = unsafe { &*ptr };
            for program_ptr in vec_from_ptr_array(&category.program_array) {
                destroy_program(program_ptr as *mut ZTResearchProgram);
            }
            drop(unsafe { Box::from_raw(ptr) });
        }

        fn build_branch(spec: &GeneratedBranch) -> *mut ZTResearchBranch {
            let mut categories = Vec::with_capacity(spec.categories.len());
            for category in &spec.categories {
                categories.push(build_category(category) as u32);
            }
            let funding_table = vec![ZTResearchFundingLevel { name_id: 0, rate: 0.0, cost: 0.0 }; spec.funding_level_count];
            let (funding_table_start, funding_table_end, funding_table_capacity) = funding_table_from_vec(funding_table);
            let branch = Box::new(ZTResearchBranch {
                config_file: BFConfigFile::default(),
                id: spec.id,
                cached_name: ZTBufferString::from_raw_parts(0, 0, 0),
                cached_desc: ZTBufferString::from_raw_parts(0, 0, 0),
                icon_ptr: 0,
                noprogicon_ptr: 0,
                current_category_ptr: 0,
                current_program_ptr: 0,
                category_array: ptr_array_from_vec(categories),
                current_funding_level: spec.current_funding_level,
                funding_table_start,
                funding_table_end,
                funding_table_capacity,
            });
            Box::into_raw(branch)
        }

        fn destroy_branch(ptr: *mut ZTResearchBranch) {
            if ptr.is_null() {
                return;
            }
            let branch = unsafe { &*ptr };
            for category_ptr in vec_from_ptr_array(&branch.category_array) {
                destroy_category(category_ptr as *mut ZTResearchCategory);
            }
            free_funding_table(branch.funding_table_start, branch.funding_table_capacity);
            drop(unsafe { Box::from_raw(ptr) });
        }

        /// Temporarily replaces `mgr.branch_array` with freshly-built branches from `specs`, runs `f`,
        /// then destroys the synthetic branches and restores `mgr.branch_array` to exactly what it held
        /// before this call (raw parts, not contents - the same save/restore convention
        /// `research_config_reimplementation`'s shadow-mode arm uses for the same field). Safe to call
        /// repeatedly with different `specs`, e.g. once per generated proptest case.
        pub(crate) fn with_synthetic_branches<R>(mgr: &mut ZTResearchMgr, specs: &[GeneratedBranch], f: impl FnOnce(&mut ZTResearchMgr) -> R) -> R {
            let original_branch_array_raw_parts = mgr.branch_array.raw_parts();

            let mut branch_ptrs = Vec::with_capacity(specs.len());
            for spec in specs {
                branch_ptrs.push(build_branch(spec) as u32);
            }
            mgr.branch_array = ptr_array_from_vec(branch_ptrs);

            let result = f(mgr);

            for branch_ptr in vec_from_ptr_array(&mgr.branch_array) {
                destroy_branch(branch_ptr as *mut ZTResearchBranch);
            }
            let (start, end, buffer_end) = original_branch_array_raw_parts;
            mgr.branch_array = ZTArray::from_raw_parts(start, end, buffer_end);

            result
        }

        /// Builds a fully synthetic, standalone `ZTResearchMgr` - **not** the real live singleton at
        /// `globals().ztresearchmgr_ptr()` - runs `f` with `specs` spliced into it via
        /// `with_synthetic_branches`, then frees it. Empirically, the real singleton is still null at
        /// the `LOAD_LANG_DLLS` injection point this harness runs from (confirmed live: calling
        /// `globals().ztresearchmgr_ptr()` there returns a null pointer), so comparison tests can't
        /// depend on it existing yet. `ZTResearchMgr::save` never reads anything but `this`, so it's
        /// always safe to call on a standalone instance built this way; `load` needs the extra care in
        /// `with_global_ztresearchmgr_ptr` below.
        pub(crate) fn with_standalone_mgr<R>(specs: &[GeneratedBranch], f: impl FnOnce(&mut ZTResearchMgr) -> R) -> R {
            let mut mgr = Box::new(ZTResearchMgr { pad0: [0; 8], elapsed_ticks: 0, branch_array: ZTArray::from_raw_parts(0, 0, 0) });
            with_synthetic_branches(&mut mgr, specs, f)
        }

        /// Address of the fixed global slot `globals().ztresearchmgr_ptr()` reads (`base + 0x239010`,
        /// no further indirection - see `ztresearch::command_debug_research_ptr`, which walks this same
        /// slot for diagnostics). `ZTResearchBranch::pick_random_program` reads this slot directly (as
        /// `GLOBAL_ZTResearchMgr`, confirmed via `ZTResearchBranch_pickRandomProgram.c`/`.asm` and
        /// `ZTApp_exit_override.c`), not `this` - so it's a real, load-bearing dependency independent of
        /// whatever `ZTResearchMgr` a caller is invoking `load`/`pick_random_program` on.
        fn global_slot_address() -> u32 {
            get_module_base("zoo.exe") as u32 + 0x0023_9010
        }

        /// Temporarily points the real `GLOBAL_ZTResearchMgr` slot at `mgr`, runs `f`, then restores
        /// whatever the slot held before this call. Needed because `ZTResearchMgr::load`'s tail
        /// unconditionally calls `ZTResearchBranch::pick_random_program` on every branch, which
        /// dereferences that global directly (see `global_slot_address`'s doc comment) - at this early
        /// injection point the real singleton isn't constructed yet (see `with_standalone_mgr`'s doc
        /// comment), so without this the dereference would read a null/garbage pointer. `save` never
        /// reads this slot, so this wrapper is only needed around `load` calls.
        pub(crate) fn with_global_ztresearchmgr_ptr<R>(mgr: &mut ZTResearchMgr, f: impl FnOnce(&mut ZTResearchMgr) -> R) -> R {
            let slot = global_slot_address();
            let original = get_from_memory::<u32>(slot);
            save_to_memory(slot, mgr as *mut ZTResearchMgr as u32);

            let result = f(mgr);

            save_to_memory(slot, original);
            result
        }
    }
}
