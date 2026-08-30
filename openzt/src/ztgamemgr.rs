//! `ZTGameMgr` reimplementation. Follows a third named pattern variant, distinct from
//! `ztadvterrainmgr.rs`'s "thin-shell whole-class" call-through and `ztmegatilemgr.rs`'s "100%
//! vanilla-owned" style: **per-method delegation to an embedded, untouched sub-object**.
//! `ZTGameMgr`'s own vtable/non-virtual logic is (or, stage by stage, will be) fully ported to Rust,
//! but each method that needs the `ZooStatus` finance/rating tracker embedded inline at `this+0x10`
//! (confirmed via `_CreateZTGameMgr.c`'s `OOAnalyzer::ZooStatus::init((ZooStatus *)(puVar2 + 4),...)`)
//! calls through to real vanilla `zoostatus::*::original()` for that sub-object's own behavior, rather
//! than porting `ZooStatus`'s own ~30-method surface (out of scope - see
//! `openzt/plans/ztgamemgr-implementation-plan.md`). Because `ZTGameMgr` stays vanilla-layout-compatible
//! and `ZooStatus` lives inline in the *same* memory block rather than a separate allocation, calling
//! vanilla `ZooStatus::*` code against that live memory from within a Rust `ZTGameMgr` method body is
//! always safe - nothing is freed or reallocated, only read/written in place, so none of `CLAUDE.md`'s
//! cross-allocator hazards apply here.
//!
//! No dynamic containers live in `ZTGameMgr`'s or the real `ZooStatus`'s own memory - only scalars and
//! fixed-size arrays (see the implementation plan's "No dynamic containers" section), so the
//! vanilla-layout-compatible struct below never needs to model a `Vec`/map/tree.

use std::ffi::c_void;

use openzt_detour::generated::{
    standalone::{DEALLOCATE, WRITE_BYTES_TO_FILE},
    ztgamemgr_menumusichandler::UPDATE as MENU_MUSIC_HANDLER_UPDATE,
    ztsoundscape::UPDATE as ZTSOUNDSCAPE_UPDATE,
    ztui_main::{
        SET_ANIMAL_RATING as ZTUI_MAIN_SET_ANIMAL_RATING, SET_DATE_TEXT as ZTUI_MAIN_SET_DATE_TEXT, SET_GUEST_RATING as ZTUI_MAIN_SET_GUEST_RATING,
        SET_MONEY_TEXT as ZTUI_MAIN_SET_MONEY_TEXT, SET_ZOO_RATING as ZTUI_MAIN_SET_ZOO_RATING,
    },
    zoostatus::{
        SPEND_MARKETING, SPEND_RESEARCH, INIT as ZOOSTATUS_INIT, LOAD as ZOOSTATUS_LOAD, RATING_CHECKS as ZOOSTATUS_RATING_CHECKS,
        SAVE as ZOOSTATUS_SAVE, UPDATE as ZOOSTATUS_UPDATE,
    },
    ztaimgr::VIRT_METH_0X58F269,
    ztgamemgr::{
        ADD_CASH, GET_DATE, HOURS_AGO, IS_GAME_DATE, IS_REAL_WORLD_DATE, LOAD, SAVE, SET_NEW_GAME_DEFAULTS, SUBTRACT_CASH, TIME_AGO, UPDATE, UPDATE_SIM,
    },
};
#[cfg(feature = "reimplementation-tests")]
use openzt_detour::generated::{standalone::CREATE_ZTGAME_MGR, ztgamemgr::ZTGAME_MGR_1};
use openzt_detour_macro::detour_mod;
use tracing::{error, info};
use windows::Win32::{
    Foundation::{FILETIME, SYSTEMTIME},
    System::{SystemInformation::GetSystemTime, Time::{FileTimeToSystemTime, SystemTimeToFileTime}},
};

use crate::{
    command_console::CommandError,
    globals::{get_module_base, globals},
    lua_fn,
    util::{get_from_memory, mut_from_memory, ref_from_memory, save_to_memory},
};

/// `DAT_006394b8`'s RVA (Ghidra VA `0x006394b8` minus the default load base `0x400000`) - a raw, signed
/// tick accumulator `ZTGameMgr::updateSim` reads/writes directly (not a pointer, so no
/// `CachedGlobalInstance` chain-walk needed - just `get_module_base("zoo.exe") + RVA` each call, same
/// pattern as this codebase's other raw-global accesses, e.g. `ztresearch.rs`).
const DAT_006394B8_RVA: u32 = 0x006394b8 - 0x400000;

/// ZTGameMgr struct. Real allocation size `0x11b0` (`_CreateZTGameMgr.c`, `operator_new(0x11b0)`).
#[derive(Debug)]
#[repr(C)]
pub struct ZTGameMgr {
    vtable: u32, // 0x0
    /// `start`/`stop`/`gotoStart`/`~ZTGameMgr`'s own "already started" guard flag
    /// (`if (*(char*)(this+4) != 0) stop(this);`). Explicitly zeroed by `CreateZTGameMgr`
    /// (`*(undefined1 *)(puVar2 + 1) = 0;`).
    started: bool, // 0x4
    _pad1b: [u8; 3],
    /// `BFGameMgr::save`/`load`/`setNewGameDefaults`/`updateSim`'s own raw elapsed-simulation-ticks
    /// accumulator (`this->mbr_0x8 += param_1` every `updateSim` call, reset to `0` in
    /// `setNewGameDefaults`).
    elapsed_sim_ticks: u32, // 0x8
    cash: f32,                // 0x0C
    pad2a: [u8; 0x28 - 0x10], // 0x10
    /// Set to `true` by `updateSim` when the game date's `w_month` field changes across its
    /// `FILETIME` round-trip (`ZTGameMgr_updateSim.c`/`.asm`: `this->field_0x28 = 1` when
    /// `*(short*)&this->field_0x1196` - i.e. `date.w_month`, confirmed against the `.asm`'s
    /// `word ptr [ESI+0x1196]` compare, **not** `w_day_of_week` as an earlier pass of this plan
    /// mis-labelled it - `Systemtime`'s own field order puts `w_day_of_week` at `+4`, not `+2` -
    /// differs before/after). Never cleared by `updateSim` itself; whatever consumes it is out of
    /// scope for this reimplementation.
    day_changed_flag: bool,   // 0x28
    pad2b: [u8; 0x30 - 0x29], // 0x29
    num_animals: u16,              // 0x30
    pad3: [u8; 0x38 - 0x32],       // 0x30
    num_species: u16,              // 0x38
    pad4: [u8; 0x3C - 0x3A],       // 0x38
    num_tired_guests: u16,         // 0x3C
    pad5: [u8; 0x40 - 0x3E],       // 0x3C
    num_hungry_guests: u16,        // 0x40
    pad6: [u8; 0x44 - 0x42],       // 0x40
    num_thirst_guests: u16,        // 0x44
    pad7: [u8; 0x48 - 0x46],       // 0x44
    num_guests_restroom_need: u16, // 0x48
    pad8: [u8; 0x54 - 0x4A],       // 0x48
    num_guests: u16,               // 0x54
    // This pad also covers the embedded `ZooStatus`'s "monthly history" fixed-size float-array region
    // (see `ZooStatus`'s own doc comment below) - offsets guessed at, unconfirmed, and pending a real
    // shape resolution:
    // admissions_income_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x254),
    // concessions_benefit_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x29c),
    // recycling_benefit_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x340),
    // // net_income maybe?: get_from_memory::<i32>(zt_game_mgr_prt + 0x404),
    // income_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x404),
    // income_expense_totals_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x44c),
    // zoo_rating_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x464),
    // unknown_array: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x4c4),
    // construction_cost_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x824),
    pad9: [u8; 0x1160 - 0x56], // 0x54
    zoo_admission_cost: f32,   // 0x1160
    pad10: [u8; 0x1190 - 0x1164], // 0x1160
    /// `ZTSoundscape*`, read/written by `start`/`stop`/`updateSim`/the destructor. Explicitly zeroed
    /// by `CreateZTGameMgr` (`puVar2[0x464] = 0;`, dword index `0x464` = byte offset `0x1190`).
    soundscape_ptr: u32, // 0x1190
    date: Systemtime,    // 0x1194
    /// `ZTGameMgr::MenuMusicHandler*`, read by `update`/`startMenuMusic`/`startMenuMusicFade`/the
    /// destructor. Explicitly zeroed by `CreateZTGameMgr` (`puVar2[0x469] = 0;`, dword index `0x469` =
    /// byte offset `0x11A4`).
    menu_music_handler_ptr: u32, // 0x11A4
    pad11: [u8; 0x11b0 - 0x11A8], // 0x11A8 - unaccounted trailing space (includes a menu-music-ini
                                   // read result at 0x11A8 that neither `ZTGameMgr`'s own logic nor any
                                   // external caller touches - fine to leave unnamed, see the
                                   // implementation plan)
}

const _: () = assert!(std::mem::size_of::<ZTGameMgr>() == 0x11b0);

/// Typed, logic-free view of the vanilla `ZooStatus` object embedded inline inside `ZTGameMgr` at
/// `+0x10`. Every real operation on this data still goes through `zoostatus::*::original()` - this
/// struct exists only to name the offsets `ZTGameMgr`'s own logic already touches
/// (`command_zoostats`'s `num_*` fields, same relative positions as `ZTGameMgr`'s own copies above,
/// less the `0x10` embedding offset), not to be a byte-exact map of every one of `ZooStatus`'s `0x1150`
/// bytes.
///
/// The "monthly history" fixed-size float-array region (see the implementation plan's "No dynamic
/// containers" background) is deliberately left as unnamed padding: loop-bound evidence in
/// `ZooStatus_init.c`/`_calculateSums.c` points to 2D `[[f32; N]; M]` shapes (using a decompiler-local
/// `ZooStatus` unit stride that isn't independently confirmed), not the flat `[f32; 12]` per-field this
/// file used to guess in a stale `TODO` comment - naively keeping that flat guess produces overlapping
/// fields for at least two of the arrays (`income_expense_totals_by_month`/`zoo_rating_by_month`).
/// Resolving the real shapes is left for a future pass, same as the still-unresolved `+0x104`/`+0x108`/
/// `+0x1164` `ZooStatus`-relative gaps the roadmap plan already calls out as out of scope.
#[derive(Debug)]
#[repr(C)]
pub struct ZooStatus {
    _pad0a: [u8; 0x1c],
    /// Current zoo rating, read directly (no formula) by `ZTGameMgr::updateSim` and passed straight to
    /// `ZTUI::main::setZooRating` (confirmed via the `.asm`: `EBX` = `&this->field_0x10` i.e. this
    /// `ZooStatus` sub-object, `dword ptr [EBX+0x1c]` read right before the `setZooRating` call).
    zoo_rating_current: i32, // 0x1c
    num_animals: u16, // 0x20
    _pad1: [u8; 0x28 - 0x22],
    num_species: u16, // 0x28
    _pad2: [u8; 0x2C - 0x2A],
    num_tired_guests: u16, // 0x2C
    _pad3: [u8; 0x30 - 0x2E],
    num_hungry_guests: u16, // 0x30
    _pad4: [u8; 0x34 - 0x32],
    num_thirst_guests: u16, // 0x34
    _pad5: [u8; 0x38 - 0x36],
    num_guests_restroom_need: u16, // 0x38
    _pad6: [u8; 0x44 - 0x3A],
    num_guests: u16, // 0x44
    _rest_pre: [u8; 0x5c - 0x46],
    /// Raw animal-happiness metric `ZTGameMgr::updateSim` feeds through `((metric + 100) * 100) / 200`
    /// before calling `ZTUI::main::setAnimalRating` (confirmed via the `.asm`: `dword ptr [EBX+0x5c]`,
    /// `EBX` = this `ZooStatus` sub-object).
    animal_rating_metric: i32, // 0x5c
    /// Same shape as `animal_rating_metric`, feeding `ZTUI::main::setGuestRating`
    /// (`dword ptr [EBX+0x60]`).
    guest_rating_metric: i32, // 0x60
    /// Everything else - including the unresolved monthly-history array region (see the struct doc
    /// comment).
    _rest: [u8; 0x1150 - 0x64],
}

const _: () = assert!(std::mem::size_of::<ZooStatus>() == 0x1150);

/// Vanilla's embedded `SYSTEMTIME` (same field order/size as `windows::Win32::Foundation::SYSTEMTIME`,
/// confirmed against that crate's own definition) - kept as a distinct, private type rather than the
/// real `windows` struct directly so `ZTGameMgr`'s own field remains `#[repr(C)]`-stable independent of
/// that crate's own attributes; `updateSim` (Stage 3) converts to/from the real `SYSTEMTIME`/`FILETIME`
/// via [`Systemtime::to_win32`]/[`Systemtime::from_win32`] for its `SystemTimeToFileTime`/
/// `FileTimeToSystemTime` round-trip.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
struct Systemtime {
    w_year: u16,
    w_month: u16,
    w_day_of_week: u16,
    w_day: u16,
    w_hour: u16,
    w_minute: u16,
    w_second: u16,
    w_milliseconds: u16,
}

impl Systemtime {
    fn to_win32(self) -> SYSTEMTIME {
        SYSTEMTIME {
            wYear: self.w_year,
            wMonth: self.w_month,
            wDayOfWeek: self.w_day_of_week,
            wDay: self.w_day,
            wHour: self.w_hour,
            wMinute: self.w_minute,
            wSecond: self.w_second,
            wMilliseconds: self.w_milliseconds,
        }
    }

    fn from_win32(value: SYSTEMTIME) -> Self {
        Self {
            w_year: value.wYear,
            w_month: value.wMonth,
            w_day_of_week: value.wDayOfWeek,
            w_day: value.wDay,
            w_hour: value.wHour,
            w_minute: value.wMinute,
            w_second: value.wSecond,
            w_milliseconds: value.wMilliseconds,
        }
    }
}

/// Packs a real `FILETIME`'s two dwords into a single raw 64-bit tick count, matching the real
/// `SUB`/`SBB`-pair arithmetic `timeAgo`/`hoursAgo` do over the two halves directly.
fn filetime_to_ticks(file_time: FILETIME) -> u64 {
    ((file_time.dwHighDateTime as u64) << 32) | file_time.dwLowDateTime as u64
}

/// Inverse of [`filetime_to_ticks`].
fn ticks_to_filetime(ticks: u64) -> FILETIME {
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

impl ZTGameMgr {
    /// enables or disables dev mode
    fn enable_dev_mode(enable: bool) {
        let enable_dev_mode_address = 0x63858A;
        unsafe {
            *(enable_dev_mode_address as *mut bool) = enable;
        }
    }

    /// The current budget, in dollars.
    pub fn cash(&self) -> f32 {
        self.cash
    }

    /// Exposed for the live `reimplementation_tests` comparison harness, to pin the real, live
    /// `ZTGameMgr` singleton's budget to a known value around a `ZTResearchBranch::update` comparison
    /// call - see `ztresearch::reimplementation_tests` support for why this writes the real singleton
    /// rather than a synthetic instance.
    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn set_cash(&mut self, value: f32) {
        self.cash = value;
    }

    /// Test-only accessors for Stage 2's `ZTGAMEMGR_SAVE_LOAD` live test, letting it seed/read the
    /// three fields `save`/`load` actually touch (`cash`/`date`/`elapsed_sim_ticks`) without exposing
    /// the private `Systemtime` type outside this module.
    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn set_elapsed_sim_ticks(&mut self, value: u32) {
        self.elapsed_sim_ticks = value;
    }

    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn elapsed_sim_ticks(&self) -> u32 {
        self.elapsed_sim_ticks
    }

    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn set_date_bytes(&mut self, bytes: [u8; 0x10]) {
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), &mut self.date as *mut Systemtime as *mut u8, 0x10) };
    }

    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn date_bytes(&self) -> [u8; 0x10] {
        let mut out = [0u8; 0x10];
        unsafe { std::ptr::copy_nonoverlapping(&self.date as *const Systemtime as *const u8, out.as_mut_ptr(), 0x10) };
        out
    }

    /// Ports `ZTGameMgr::subtractCash` (`ZTGameMgr_subtractCash.c`, read in full): subtracts `amount`
    /// from the budget, then refreshes the on-screen money display (`ZTUI::main::setMoneyText`). Used
    /// by `ztresearch::ZTResearchBranch::update`'s native reimplementation of the branch funding cost,
    /// among other callers.
    ///
    /// Now that Stage 4 wires a real `#[detour(SUBTRACT_CASH)]`, this ports the actual body rather than
    /// calling through to `.original()` (as it did through Stages 0-3) - vanilla-side callers of the
    /// real function address (not just Rust API callers) now also route through this same logic once
    /// detoured. The real signature has a trailing, unread `bool` parameter (`ZTGameMgr::subtractCash(float,
    /// bool)`, per the `.asm`'s `RET 8`) - not part of this method's own logic, so not part of this
    /// method's own signature either; the detour wrapper below supplies/discards it.
    pub fn subtract_cash(&mut self, amount: f32) {
        self.cash -= amount;
        unsafe { ZTUI_MAIN_SET_MONEY_TEXT.original()() };
    }

    /// Ports `ZTGameMgr::addCash` (`ZTGameMgr_addCash.c`, read in full): adds `amount` to the budget,
    /// then refreshes the on-screen money display (`ZTUI::main::setMoneyText`).
    pub fn add_cash(&mut self, amount: f32) {
        self.cash += amount;
        unsafe { ZTUI_MAIN_SET_MONEY_TEXT.original()() };
    }

    /// Ports `ZTGameMgr::getDate` (`ZTGameMgr_getDate.c`/`.asm`, read in full): converts `date` to a
    /// `FILETIME` via `SystemTimeToFileTime` and returns it as a raw 64-bit tick count. Per the real
    /// body, a conversion failure is never checked - the decompile just proceeds with whatever ended up
    /// in the (potentially-uninitialized) local `FILETIME`. This port can't reproduce genuine stack
    /// garbage, so it substitutes a zeroed `FILETIME` on failure instead (same reasoning as
    /// `update_sim`'s own `SystemTimeToFileTime` call, which ignores failure identically).
    pub fn get_date(&self) -> u64 {
        let mut file_time = FILETIME::default();
        let _ = unsafe { SystemTimeToFileTime(&self.date.to_win32(), &mut file_time) };
        filetime_to_ticks(file_time)
    }

    /// Ports `ZTGameMgr::isGameDate` (`ZTGameMgr_isGameDate.c`/`.asm`, read in full - the `.asm` is the
    /// clean read; the `.c`'s messy `CONCAT`/register-reuse noise around the return value is decompiler
    /// artifact, not real extra logic). Round-trips `get_date()` back through `FileTimeToSystemTime` and
    /// compares `day`/`month` against the result, `0xffffffff` acting as a per-field wildcard. Returns
    /// `false` outright if the round-trip fails.
    ///
    /// **Correction from the implementation plan**, which described this as `isGameDate(month, day)`:
    /// the real parameter order, confirmed independently from both the `.c`'s offset math
    /// (`local_10._6_4_` = `wDay` compared against `param_1`; `local_10._2_4_` = `wMonth` compared
    /// against `param_2`) and the `.asm`'s stack-offset reads, is `(day, month)`.
    pub fn is_game_date(&self, day: u32, month: u32) -> bool {
        let file_time = ticks_to_filetime(self.get_date());
        let mut sys_time = SYSTEMTIME::default();
        if unsafe { FileTimeToSystemTime(&file_time, &mut sys_time) }.is_err() {
            return false;
        }
        (day == 0xffffffff || sys_time.wDay as u32 == day) && (month == 0xffffffff || sys_time.wMonth as u32 == month)
    }

    /// Ports `ZTGameMgr::isRealWorldDate` (`ZTGameMgr_isRealWorldDate.c`, read in full). Unlike
    /// `is_game_date`, this has no `this` dependency at all (confirmed by its `stdcall`/no-`this`
    /// `IS_REAL_WORLD_DATE` entry) - it calls `GetSystemTime` directly and has no `0xffffffff` wildcard
    /// handling. **Same parameter-order correction as `is_game_date`**: real order is `(day, month)`,
    /// not the plan's `(month, day)`.
    pub fn is_real_world_date(day: u32, month: u32) -> bool {
        let sys_time = unsafe { GetSystemTime() };
        sys_time.wDay as u32 == day && sys_time.wMonth as u32 == month
    }

    /// Ports `ZTGameMgr::timeAgo` (`ZTGameMgr_timeAgo.c`/`.asm`, read in full): `get_date() - reference`
    /// as a 64-bit subtraction (the real body's `SUB`/`SBB` pair over the two `FILETIME` dwords is
    /// exactly a wrapping `u64` subtraction). See the `TIME_AGO` `FunctionDef`'s own doc comment in
    /// `generated.rs` for why the auto-generated entry needed hand-correcting before this could be
    /// wired to a real detour at all.
    pub fn time_ago(&self, reference: u64) -> u64 {
        self.get_date().wrapping_sub(reference)
    }

    /// Ports `ZTGameMgr::hoursAgo` (`ZTGameMgr_hoursAgo.c`/`.asm`, read in full): same `get_date() -
    /// reference` 64-bit subtraction as `time_ago`, divided by `36_000_000_000` (100ns intervals per
    /// hour) via the real body's own unsigned 64-bit division (`_aulldiv`). See the `HOURS_AGO`
    /// `FunctionDef`'s own doc comment in `generated.rs` for the separate return-type correction this
    /// needed (the auto-generated `*const u64` would have silently dropped the high dword of a genuine
    /// EDX:EAX register-pair return).
    pub fn hours_ago(&self, reference: u64) -> u64 {
        self.get_date().wrapping_sub(reference) / 36_000_000_000
    }

    /// Calls the vanilla `ZooStatus::spendResearch` on the embedded `ZooStatus` finance-tracker at
    /// `self + 0x10` (per `resources/decompiles/ZTResearchBranch_update.c`, which calls it right
    /// before `subtractCash`; the sibling `ZTMarketing::update` confirms the same `&GameMgr->field_0x10`
    /// `ZooStatus` sub-object at the exact same call shape with its own `spendMarketing`). Per
    /// `resources/decompiles/ZooStatus_spendResearch.c`, this only ever writes running-total fields on
    /// `this` itself - no further calls, no other side effects. Used by
    /// `ztresearch::ZTResearchBranch::update`'s native reimplementation, called before `subtract_cash`
    /// to match vanilla's own call order.
    pub fn spend_research(&mut self, amount: f32) {
        let zoostatus_ptr = (self as *mut Self as u32 + 0x10) as *const u32;
        unsafe { SPEND_RESEARCH.original()(zoostatus_ptr, amount) }
    }

    /// Calls the vanilla `ZooStatus::spendMarketing` on the same embedded `ZooStatus` sub-object as
    /// `spend_research`. Used by `ztmarketing::ZTMarketing::update`, called before `subtract_cash` to
    /// match vanilla's own call order.
    pub fn spend_marketing(&mut self, amount: f32) {
        let zoostatus_ptr = (self as *mut Self as u32 + 0x10) as *const u32;
        unsafe { SPEND_MARKETING.original()(zoostatus_ptr, amount) }
    }

    /// Ports `ZTGameMgr::setNewGameDefaults` (vtable `+0x4`), with `BFGameMgr::setNewGameDefaults`'s own
    /// base-class body (just zeroing `elapsed_sim_ticks`) inlined directly - see the module doc comment
    /// for why there's no separate `BFGameMgr`-vs-`ZTGameMgr` split kept in this port.
    ///
    /// Per the decompile/`.asm` (`ZTGameMgr_setNewGameDefaults.c`/`.asm`, read in full - **the plan this
    /// was scoped from mis-described this method's first step as "zero `elapsed_sim_ticks`"; the real
    /// first write is `[this+0xc] = 0`, i.e. `cash`, confirmed independently by both the Windows
    /// `LEA ECX,[ESI+0xc]` / `MOV [ECX],EAX` `.asm` and the macOS decompile's `ZTEcon__init((float
    /// *)(param_1 + 0xc))` - `elapsed_sim_ticks` is only ever zeroed once, at the very end**), the real
    /// order is:
    /// 1. `cash = 0.0`
    /// 2. `ZooStatus::init(&self.zoo_status, config)` (real vanilla call-through, embedded sub-object)
    /// 3. set `date` to the hardcoded new-game default (2001-01-01, a Monday, 00:00:00.000)
    /// 4. if `is_new_game`: call through `GLOBAL_ZTAIMgr`'s real vtable slot `+0x4` (`0x0058f269`,
    ///    inherited unchanged from `BFAIMgr` - see `private/docs/vtables/BFAIMgr.md`) with a single `0`
    ///    argument, matching the real thiscall/1-arg shape confirmed by both this function's and
    ///    `_setCursorQuality`'s own `.asm`
    /// 5. `ZooStatus::ratingChecks(&self.zoo_status)` (real vanilla call-through)
    /// 6. `elapsed_sim_ticks = 0`
    pub fn set_new_game_defaults(&mut self, config: *const u32, is_new_game: bool) {
        self.cash = 0.0;

        let zoostatus_ptr = (self as *mut Self as u32 + 0x10) as *const u32;
        unsafe { ZOOSTATUS_INIT.original()(zoostatus_ptr, config as *const c_void) };

        self.date = Systemtime {
            w_year: 0x7d1,
            w_month: 1,
            w_day_of_week: 1,
            w_day: 1,
            w_hour: 0,
            w_minute: 0,
            w_second: 0,
            w_milliseconds: 0,
        };

        if is_new_game {
            unsafe { VIRT_METH_0X58F269.original()(globals().ztaimgr_ptr(), 0) };
        }

        unsafe { ZOOSTATUS_RATING_CHECKS.original()(zoostatus_ptr) };

        self.elapsed_sim_ticks = 0;
    }

    /// Ports `ZTGameMgr::save` (vtable `+0x8`), with `BFGameMgr::save`'s own base-class body (just
    /// writing `elapsed_sim_ticks`) inlined directly - see the module doc comment for why there's no
    /// separate `BFGameMgr`-vs-`ZTGameMgr` split kept in this port.
    ///
    /// Per the decompile (`ZTGameMgr_save.c`, read in full), write order is: a fixed `0` marker dword
    /// (`local_8` - pure format padding, no real state); `ZooStatus::save` on the embedded sub-object at
    /// `self+0x10` (real vanilla call-through); `date` raw (16 bytes); `cash` (captured into a local
    /// before the marker write in the decompile, but reading it here instead is equivalent - nothing in
    /// between can mutate `cash`); then chain to base (just `elapsed_sim_ticks`). Every step's success is
    /// ANDed together, matching `ztawardmgr.rs`'s own `save`.
    pub fn save(&self, file: *const u32) -> bool {
        let marker: u32 = 0;
        let mut ok = unsafe { WRITE_BYTES_TO_FILE.original()(&marker as *const u32, 4, 1, file as *const i8) };

        let zoostatus_ptr = (self as *const Self as u32 + 0x10) as *const u32;
        let zoostatus_result = unsafe { ZOOSTATUS_SAVE.original()(zoostatus_ptr, file as *const i8) };
        ok &= zoostatus_result == 1;

        ok &= unsafe { WRITE_BYTES_TO_FILE.original()(&self.date as *const Systemtime as *const u32, 0x10, 1, file as *const i8) };

        ok &= unsafe { WRITE_BYTES_TO_FILE.original()(&self.cash as *const f32 as *const u32, 4, 1, file as *const i8) };

        // BFGameMgr::save inlined: writes the raw elapsed_sim_ticks dword.
        ok &= unsafe { WRITE_BYTES_TO_FILE.original()(&self.elapsed_sim_ticks as *const u32, 4, 1, file as *const i8) };

        ok
    }

    /// Ports `ZTGameMgr::load` (vtable `+0xc`), with `BFGameMgr::load`'s own base-class body inlined
    /// directly (see [`Self::save`]'s doc comment for why).
    ///
    /// Per the decompile (`ZTGameMgr_load.c`, read in full), read order mirrors `save`'s write order: a
    /// `0` marker dword (read and discarded - only its success/failure matters) -> `ZooStatus::load` on
    /// the embedded sub-object -> `date` raw (16 bytes, written directly into `self.date` regardless of
    /// what happens next, matching the decompile's direct-into-field read) -> `cash` (read into a local
    /// first). **`cash` is only assigned from that local after every earlier read has succeeded** -
    /// matching the decompile's `if (bVar4 != 0) { this->field_0xc = local_8; ... }` gating: if the
    /// marker or `ZooStatus::load` fails, this returns `false` immediately without touching `cash` or
    /// even attempting the `date`/`cash` reads; if `date` or `cash` fails, `cash` is left untouched (but
    /// `date` may already have been partially overwritten, exactly as vanilla's own read-in-place would
    /// leave it).
    ///
    /// `BFGameMgr::load`'s own inlined base body (`BFGameMgr_load.c`) only reads `elapsed_sim_ticks` when
    /// `version > 0x48`; older saves leave it zeroed instead.
    pub fn load(&mut self, file: *const u32, version: u32) -> bool {
        let mut marker: u32 = 0;
        let marker_ok = unsafe { DEALLOCATE.original()(&mut marker as *mut u32 as *const u32, 4, 1, file as *const u8) } == 1;
        if !marker_ok {
            return false;
        }

        let zoostatus_ptr = (self as *mut Self as u32 + 0x10) as *const u32;
        let zoostatus_result = unsafe { ZOOSTATUS_LOAD.original()(zoostatus_ptr, file as *const u8, version) };
        if (zoostatus_result & 0xff) == 0 {
            return false;
        }

        let date_ok = unsafe { DEALLOCATE.original()(&mut self.date as *mut Systemtime as *const u32, 0x10, 1, file as *const u8) } == 1;
        let mut cash: f32 = 0.0;
        let cash_ok = unsafe { DEALLOCATE.original()(&mut cash as *mut f32 as *const u32, 4, 1, file as *const u8) } == 1;

        if !(date_ok && cash_ok) {
            return false;
        }

        self.cash = cash;

        // BFGameMgr::load inlined: only reads elapsed_sim_ticks for saves newer than version 0x48.
        if version > 0x48 {
            unsafe { DEALLOCATE.original()(&mut self.elapsed_sim_ticks as *mut u32 as *const u32, 4, 1, file as *const u8) == 1 }
        } else {
            self.elapsed_sim_ticks = 0;
            true
        }
    }

    /// Ports `ZTGameMgr::update` (vtable `+0x10`). Per the decompile (`ZTGameMgr_update.c`, read in
    /// full) this is a pure call-through to the embedded, out-of-scope `MenuMusicHandler` when present -
    /// no logic of `ZTGameMgr`'s own.
    pub fn update(&self, delta: u32) {
        if self.menu_music_handler_ptr != 0 {
            unsafe { MENU_MUSIC_HANDLER_UPDATE.original()(self.menu_music_handler_ptr as *const c_void, delta) };
        }
    }

    /// Ports `ZTGameMgr::updateSim` (vtable `+0x14`). Per the decompile/`.asm` (`ZTGameMgr_updateSim.c`/
    /// `.asm`, read in full), in order:
    /// 1. `elapsed_sim_ticks += delta` (`BFGameMgr::updateSim`'s own base body, inlined - see the module
    ///    doc comment for why there's no separate `BFGameMgr`-vs-`ZTGameMgr` split kept in this port).
    /// 2. The raw global tick accumulator `DAT_006394b8` (`this->mbr_0x8`'s sibling, not part of
    ///    `ZTGameMgr`'s own memory) `+= delta`.
    /// 3. `ZooStatus::update(&self.zoo_status, delta)` (real vanilla call-through, embedded sub-object).
    /// 4. If the accumulator now exceeds `0x3e9` (1001): reduce it `%= 0x3e9`, then recompute and push
    ///    animal/guest/zoo ratings plus the money/date UI text (`ZTUI::main::set{Animal,Guest,Zoo}Rating`/
    ///    `setMoneyText`/`setDateText`) - the animal/guest metrics live inside the embedded `ZooStatus`
    ///    sub-object (confirmed via the `.asm`: `EBX` = `&this->field_0x10` for these reads, not `this`
    ///    directly, resolving what looked like a `ZTGameMgr`- vs `ZooStatus`-relative offset mismatch
    ///    between the `.c` and `.asm`), each fed through `((metric + 100) * 100) / 200` unless the
    ///    corresponding count (`num_animals`/`num_guests`) is `0`, in which case the rating is `0`
    ///    outright; zoo rating is read directly, no formula.
    /// 5. If `soundscape_ptr` is non-null, call through `ZTSoundscape::update` (out-of-scope class).
    /// 6. Advance `date` by `delta` simulation ticks via a real `SystemTimeToFileTime`/`FileTimeToSystemTime`
    ///    round-trip (`delta * 72000000` 100ns-intervals added to the `FILETIME` value - a
    ///    `SystemTimeToFileTime` failure is intentionally ignored here, matching the decompile's own
    ///    `GetLastError()`-then-continue error path, which has no further observable effect; a
    ///    `FileTimeToSystemTime` failure aborts the rest of the method, also matching), then sets
    ///    `day_changed_flag` if the round-trip changed `date.w_month` (see that field's own doc comment
    ///    for why - **not** `w_day_of_week`, correcting an earlier pass of the implementation plan).
    pub fn update_sim(&mut self, delta: u32) {
        self.elapsed_sim_ticks = self.elapsed_sim_ticks.wrapping_add(delta);

        let dat_addr = get_module_base("zoo.exe") as u32 + DAT_006394B8_RVA;
        let mut tick_accumulator: i32 = get_from_memory(dat_addr);
        tick_accumulator = tick_accumulator.wrapping_add(delta as i32);
        save_to_memory(dat_addr, tick_accumulator);

        let zoostatus_ptr = (self as *mut Self as u32 + 0x10) as *const u32;
        unsafe { ZOOSTATUS_UPDATE.original()(zoostatus_ptr, delta as i32) };

        if tick_accumulator > 0x3e9 {
            tick_accumulator %= 0x3e9;
            save_to_memory(dat_addr, tick_accumulator);

            let zoostatus = unsafe { &*(zoostatus_ptr as *const ZooStatus) };

            let animal_rating = if self.num_animals == 0 { 0 } else { (zoostatus.animal_rating_metric + 100) * 100 / 200 };
            unsafe { ZTUI_MAIN_SET_ANIMAL_RATING.original()(animal_rating) };

            let guest_rating = if self.num_guests == 0 { 0 } else { (zoostatus.guest_rating_metric + 100) * 100 / 200 };
            unsafe { ZTUI_MAIN_SET_GUEST_RATING.original()(guest_rating) };

            unsafe { ZTUI_MAIN_SET_ZOO_RATING.original()(zoostatus.zoo_rating_current) };
            unsafe { ZTUI_MAIN_SET_MONEY_TEXT.original()() };
            unsafe { ZTUI_MAIN_SET_DATE_TEXT.original()() };
        }

        if self.soundscape_ptr != 0 {
            unsafe { ZTSOUNDSCAPE_UPDATE.original()(self.soundscape_ptr as *const c_void, delta as i32) };
        }

        let previous_month = self.date.w_month;

        let mut file_time = FILETIME::default();
        // A SystemTimeToFileTime failure is intentionally ignored (matches the decompile's own
        // GetLastError()-then-continue path, which has no further observable effect).
        let _ = unsafe { SystemTimeToFileTime(&self.date.to_win32(), &mut file_time) };

        let file_time_ticks = ((file_time.dwHighDateTime as u64) << 32) | file_time.dwLowDateTime as u64;
        let new_file_time_ticks = file_time_ticks.wrapping_add((delta as u64) * 72000000);
        file_time.dwLowDateTime = new_file_time_ticks as u32;
        file_time.dwHighDateTime = (new_file_time_ticks >> 32) as u32;

        let mut new_sys_time = SYSTEMTIME::default();
        if unsafe { FileTimeToSystemTime(&file_time, &mut new_sys_time) }.is_err() {
            return;
        }
        self.date = Systemtime::from_win32(new_sys_time);

        if self.date.w_month != previous_month {
            self.day_changed_flag = true;
        }
    }

    /// Test-only accessors for Stage 3's `ZTGAMEMGR_UPDATE_SIM` live test, letting it seed/read
    /// `day_changed_flag` without exposing it as public API.
    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn set_day_changed_flag(&mut self, value: bool) {
        self.day_changed_flag = value;
    }

    #[cfg(feature = "reimplementation-tests")]
    pub(crate) fn day_changed_flag(&self) -> bool {
        self.day_changed_flag
    }
}

/// a command that prints the SYSTEMTIME struct in memory in a human-readable format
/// usage: `get_date`
pub fn command_get_date_str(_args: Vec<&str>) -> Result<String, CommandError> {
    let ztgamemgr = globals().ztgamemgr();
    let date = ztgamemgr.date.clone();
    info!("Date: {:#?}", date);

    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        date.w_year, date.w_month, date.w_day, date.w_hour, date.w_minute, date.w_second
    ))
}

/// a command that adds cash to the player's account
/// usage: `add_cash <amount>`
pub fn command_add_cash(args: Vec<&str>) -> Result<String, CommandError> {
    let ptr = globals().ztgamemgr_ptr();
    let amount = args[0].parse::<f32>()?;
    unsafe { (*ptr).add_cash(amount) };
    Ok(format!("Added ${}", args[0]))
}

/// a command that enables or disables dev mode
/// usage: `enable_dev_mode <true/false>`
pub fn command_enable_dev_mode(args: Vec<&str>) -> Result<String, CommandError> {
    let enable = args[0].parse()?;
    ZTGameMgr::enable_dev_mode(enable);
    Ok(format!("Dev mode enabled: {}", enable))
}

/// a command that prints various stats about the zoo
/// usage: `zoostats`
pub fn command_zoostats(_args: Vec<&str>) -> Result<String, CommandError> {
    let ztgamemgr = globals().ztgamemgr();
    Ok(format!("\nBudget: {}\nAnimals: {}\nSpecies: {}\nTired Guests: {}\nHungry Guests: {}\nThirsty Guests: {}\nGuests Need Restroom: {}\nNum Guests: {}\nZoo Admission Cost: ${}", ztgamemgr.cash, ztgamemgr.num_animals, ztgamemgr.num_species, ztgamemgr.num_tired_guests, ztgamemgr.num_hungry_guests, ztgamemgr.num_thirst_guests, ztgamemgr.num_guests_restroom_need, ztgamemgr.num_guests, ztgamemgr.zoo_admission_cost))
}

/// Stage 5 of `openzt/plans/ztgamemgr-implementation-plan.md`: the destructor and the
/// `start`/`stop`/`gotoStart`/`removedZooDoo`/`startMenuMusic*` call-through tier are deliberately **not**
/// detoured. No porting, no new live tests - only a documented decision per item, all confirmed directly
/// against the decompiles (`private/resources/decompiles/ZTGameMgr_*`) this session:
///
/// - **Destructor** (`ZTGAME_MGR_0`/`ZTGAME_MGR_1`): `~ZTGameMgr_0.c` tears down the embedded
///   `ZTSoundscape` (if `soundscape_ptr != 0`) and `MenuMusicHandler` (if `menu_music_handler_ptr != 0`,
///   both out of scope, see the module doc comment) then swaps the vtable pointer to `BFMgr_vftable` and
///   returns, continuing into `BFMgr`'s own base-class teardown; `~ZTGameMgr_1.c` is just the deleting
///   variant (`bDelete` byte gating a `FUN_00402629` free after the same body runs). Since this
///   reimplementation stays vanilla-layout-compatible (style 1: same memory, no independent Rust-owned
///   heap state), there is nothing this detour would do differently from vanilla's own body - mirrors
///   `ztmegatilemgr.rs`'s and `ztadvterrainmgr.rs`'s own stated reasoning for skipping their destructors.
/// - **`start()`/`stop()`**: confirmed clean, fully-typed decompiles (`ZTGameMgr_start.c`/`_stop.c`) -
///   `start()` `operator_new`s a fresh `ZTSoundscape`, pulls ambient-sound config from the still-untouched
///   `GLOBAL_ZTScenarioMgr` (`getCrowdAmbientsName`/`getWorldAmbientsName`/`getCrowdConfigName`/
///   `getWorldConfigName`), inits the soundscape, and sets `started`; `stop()` tears the soundscape back
///   down, clears `started`, and toggles two `BFUIMgr` elements (`0x430`/`0x42f`) plus a `DX8SndMgr`
///   vtable call, gated by a raw global flag (`DAT_00638588`). Genuinely portable, but every dependency
///   (`ZTSoundscape`, `ZTScenarioMgr`'s ambient getters, `BFUIMgr`, `DX8SndMgr`) is out of this plan's
///   scope - left un-detoured rather than porting orchestration logic around still-un-reimplemented
///   collaborators.
/// - **`gotoStart(...)`**: `ZTGameMgr_gotoStart.c` is confirmed genuinely decompiler-mangled, not just
///   verbose - `unaff_EBX`/`unaff_ESI`/`unaff_EDI` register-allocation artifacts stand in for real
///   parameters/locals, and the recovered signature (14 params, mostly untyped `undefined`) doesn't match
///   any real call site. `generated.rs`'s own `GOTO_START` entry (`u8×12, u32×2`) reflects the same
///   automatic-signature-recovery confusion. Not faithfully portable from this decompile - left
///   un-detoured.
/// - **`removedZooDoo(...)`**: `ZTGameMgr_removedZooDoo.c` is likewise confirmed genuinely mangled - 11
///   raw/untyped parameters plus `unaff_EBX`/`unaff_EBP`/`unaff_EDI` register artifacts, clearly a
///   partially-recovered inlined helper (a tile-distance search loop feeding
///   `ZooStatus::refundConstruction`/`addCash`), not the real method signature. Left un-detoured.
/// - **`startMenuMusic()`/`startMenuMusicFade()`**: confirmed by reading all three
///   `startMenuMusicFade_{0,1,2}` decompiles and `.meta` files directly (resolving the plan's open
///   question about whether they're 3 redundant call sites or 3 real variants - they're neither, cleanly).
///   `_0` (thiscall, `0x004c9d67`) and `_2` (fastcall, `0x004cc59d`) have identical trivial bodies -
///   forward to `MenuMusicHandler::startFade` when `menu_music_handler_ptr != 0` - and both `.meta`s list
///   `startFade` as a called function: these are two distinct compiled/calling-convention instances of
///   the *same* logical `ZTGameMgr::startMenuMusicFade`, not duplicates worth deduping here. `_1`
///   (fastcall, `0x004ca478`) is a **different function** entirely - its body directly implements a
///   vtable dispatch (`(**(code**)(*vtable+0x50))()`) plus a fade-state flag/counter reset, calls nothing
///   named (empty `calling_functions` in its `.meta`, unlike `_0`/`_2`), and is almost certainly
///   `MenuMusicHandler::startFade`'s own real body, mislabeled with the `ZTGameMgr::` name by the
///   decompile corpus's automated naming pass. All three stay out of scope regardless (either
///   `ZTGameMgr::startMenuMusicFade` itself - a pure call-through into the out-of-scope
///   `MenuMusicHandler` - or `MenuMusicHandler::startFade` itself, squarely inside that same out-of-scope
///   class), as does the single-address `startMenuMusic()` (same `menu_music_handler_ptr`-gated
///   call-through shape, plus a UI callback via `FUN_00521e18`). Left un-detoured.
///
/// registers the Lua functions
pub fn init() {
    // get_date() - no args
    lua_fn!("get_date", "Returns current in-game date/time", "get_date()", || {
        match command_get_date_str(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string()))),
        }
    });

    // add_cash(amount) - single f32 arg
    lua_fn!("add_cash", "Adds cash to player's budget", "add_cash(amount)", |amount: f32| {
        let amount_str = amount.to_string();
        match command_add_cash(vec![&amount_str]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string()))),
        }
    });

    // enable_dev_mode(enabled) - bool arg
    lua_fn!(
        "enable_dev_mode",
        "Enables/disables developer mode",
        "enable_dev_mode(true/false)",
        |enabled: bool| {
            let enabled_str = enabled.to_string();
            match command_enable_dev_mode(vec![&enabled_str]) {
                Ok(result) => Ok((Some(result), None::<String>)),
                Err(e) => Ok((None::<String>, Some(e.to_string()))),
            }
        }
    );

    // zoostats() - no args
    lua_fn!("zoostats", "Returns zoo statistics", "zoostats()", || {
        match command_zoostats(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string()))),
        }
    });

    if let Err(e) = unsafe { gamemgr_lifecycle_detours::init_detours() } {
        error!("Failed to initialise ZTGameMgr lifecycle detours: {e:?}");
    }

    if let Err(e) = unsafe { gamemgr_finance_detours::init_detours() } {
        error!("Failed to initialise ZTGameMgr finance/date detours: {e:?}");
    }
}

/// Stage 1-3's vtable detours (`setNewGameDefaults`/`save`/`load`/`update`/`updateSim`, per
/// `openzt/plans/ztgamemgr-implementation-plan.md`); named `lifecycle` to distinguish from
/// `gamemgr_finance_detours` below, which covers Stage 4's non-virtual `addCash`/`subtractCash`/
/// date-family helpers.
#[detour_mod]
mod gamemgr_lifecycle_detours {
    use super::*;

    #[detour(SET_NEW_GAME_DEFAULTS)]
    unsafe extern "thiscall" fn set_new_game_defaults(this: *const u32, config: *const u32, is_new_game: bool) {
        unsafe { mut_from_memory::<ZTGameMgr>(this) }.set_new_game_defaults(config, is_new_game);
    }

    #[detour(SAVE)]
    unsafe extern "thiscall" fn save(this: *const u32, file: *const u32) -> u32 {
        unsafe { ref_from_memory::<ZTGameMgr>(this) }.save(file) as u32
    }

    #[detour(LOAD)]
    unsafe extern "thiscall" fn load(this: *const u32, file: *const u32, version: u32) -> u8 {
        unsafe { mut_from_memory::<ZTGameMgr>(this) }.load(file, version) as u8
    }

    #[detour(UPDATE)]
    unsafe extern "thiscall" fn update(this: *const u32, delta: u32) {
        unsafe { ref_from_memory::<ZTGameMgr>(this) }.update(delta);
    }

    #[detour(UPDATE_SIM)]
    unsafe extern "thiscall" fn update_sim(this: *const u32, delta: u32) {
        unsafe { mut_from_memory::<ZTGameMgr>(this) }.update_sim(delta);
    }
}

/// Stage 4's non-virtual finance/date detours (`addCash`/`subtractCash`/`getDate`/`isGameDate`/
/// `isRealWorldDate`/`timeAgo`/`hoursAgo`, per `openzt/plans/ztgamemgr-implementation-plan.md`).
#[detour_mod]
mod gamemgr_finance_detours {
    use super::*;

    #[detour(ADD_CASH)]
    unsafe extern "thiscall" fn add_cash(this: *const u32, amount: f32) {
        unsafe { mut_from_memory::<ZTGameMgr>(this) }.add_cash(amount);
    }

    /// The real signature carries a trailing `bool` (`ZTGameMgr::subtractCash(float, bool)`, per the
    /// `.asm`'s `RET 8`) that neither platform's compiled body reads - discarded here, only present so
    /// the detour's stack accounting matches the real function's.
    #[detour(SUBTRACT_CASH)]
    unsafe extern "thiscall" fn subtract_cash(this: *const u32, amount: f32, _unused: bool) {
        unsafe { mut_from_memory::<ZTGameMgr>(this) }.subtract_cash(amount);
    }

    #[detour(GET_DATE)]
    unsafe extern "thiscall" fn get_date(this: *const u32, out: *const FILETIME) -> *const FILETIME {
        let ticks = unsafe { ref_from_memory::<ZTGameMgr>(this) }.get_date();
        unsafe { *(out as *mut FILETIME) = ticks_to_filetime(ticks) };
        out
    }

    #[detour(IS_GAME_DATE)]
    unsafe extern "thiscall" fn is_game_date(this: *const u32, day: u32, month: u32) -> u32 {
        unsafe { ref_from_memory::<ZTGameMgr>(this) }.is_game_date(day, month) as u32
    }

    #[detour(IS_REAL_WORLD_DATE)]
    unsafe extern "stdcall" fn is_real_world_date(day: i32, month: u32) -> u32 {
        ZTGameMgr::is_real_world_date(day as u32, month) as u32
    }

    #[detour(TIME_AGO)]
    unsafe extern "thiscall" fn time_ago(this: *const u32, out: *const FILETIME, reference: FILETIME) -> *const FILETIME {
        let result = unsafe { ref_from_memory::<ZTGameMgr>(this) }.time_ago(filetime_to_ticks(reference));
        unsafe { *(out as *mut FILETIME) = ticks_to_filetime(result) };
        out
    }

    #[detour(HOURS_AGO)]
    unsafe extern "thiscall" fn hours_ago(this: *const u32, reference_low: u32, reference_high: i32) -> u64 {
        let reference = ((reference_high as u32 as u64) << 32) | reference_low as u64;
        unsafe { ref_from_memory::<ZTGameMgr>(this) }.hours_ago(reference)
    }
}

/// Live-comparison test support for `reimplementation_tests`. Unlike `ZTAwardMgr` (fixed global
/// address, no standalone-instance capability), `ZTGameMgr` has a genuine free-function constructor
/// (`standalone::CREATE_ZTGAME_MGR`) that `operator_new`s a fresh `0x11b0`-byte block and returns it,
/// entirely independent of the real `GLOBAL_ZTGameMgr` singleton - enabling the same "build a second
/// standalone instance, drive real vanilla `.original()` calls against one and the Rust
/// reimplementation against the other" pattern `ztthoughtmgr`/`ztmegatilemgr` use.
#[cfg(feature = "reimplementation-tests")]
pub(crate) mod live_support {
    use super::*;

    /// Builds a standalone `ZTGameMgr` via the real vanilla free-function constructor. Confirmed via
    /// `_CreateZTGameMgr.c` that construction explicitly zeroes `started` (offset `0x4`),
    /// `soundscape_ptr` (`0x1190`), and `menu_music_handler_ptr` (`0x11A4`) - `operator_new` itself does
    /// not guarantee zeroed memory, so any *other* field a later stage's test reads must either be
    /// confirmed as genuinely initialized by this constructor plus `set_new_game_defaults` (Stage 1), or
    /// the block explicitly `memset` to `0` first.
    pub(crate) fn build_standalone_mgr() -> *mut ZTGameMgr {
        unsafe { CREATE_ZTGAME_MGR.original()() as *mut ZTGameMgr }
    }

    /// Tears down a standalone instance built by [`build_standalone_mgr`] via the real vanilla deleting
    /// destructor (`ZTGAME_MGR_1`, `bDelete=1`) - safe here since this reimplementation stays
    /// vanilla-layout-compatible with no independent Rust-owned heap state to worry about
    /// double-freeing, unlike `ztthoughtmgr`'s intrusive list nodes (see `CLAUDE.md`'s cross-allocator
    /// warning).
    pub(crate) fn destroy_standalone_mgr(ptr: *mut ZTGameMgr) {
        if ptr.is_null() {
            return;
        }
        unsafe { ZTGAME_MGR_1.original()(ptr as *const u32, 1) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ztgamemgr_size_matches_real_allocation() {
        assert_eq!(std::mem::size_of::<ZTGameMgr>(), 0x11b0);
    }

    #[test]
    fn zoostatus_size_matches_embedded_region() {
        assert_eq!(std::mem::size_of::<ZooStatus>(), 0x1150);
    }
}
