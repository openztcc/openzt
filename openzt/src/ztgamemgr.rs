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

use openzt_detour::generated::{
    zoostatus::{SPEND_MARKETING, SPEND_RESEARCH},
    ztgamemgr::SUBTRACT_CASH,
};
#[cfg(feature = "reimplementation-tests")]
use openzt_detour::generated::{standalone::CREATE_ZTGAME_MGR, ztgamemgr::ZTGAME_MGR_1};
use tracing::info;

use crate::{command_console::CommandError, globals::globals, lua_fn};

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
    cash: f32,                     // 0x0C
    pad2: [u8; 0x30 - 0x10],       // 0x0C
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
    _pad0: [u8; 0x20],
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
    /// Everything else - including the unresolved monthly-history array region (see the struct doc
    /// comment).
    _rest: [u8; 0x1150 - 0x46],
}

const _: () = assert!(std::mem::size_of::<ZooStatus>() == 0x1150);

/// SYSTEMTIME struct from Windows API
/// TODO: Replace this with the actual SYSTEMTIME struct from the Windows API
#[derive(Debug, Clone)]
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

    /// Calls the vanilla `ZTGameMgr::subtractCash`: subtracts `amount` from the budget, then refreshes
    /// the on-screen money display (`ZTUI::main::setMoneyText`). Used by
    /// `ztresearch::ZTResearchBranch::update`'s native reimplementation of the branch funding cost,
    /// among other callers.
    ///
    /// Takes a trailing `bool` matching the real signature (`ZTGameMgr::subtractCash(float, bool)`,
    /// per the `.asm`'s `RET 8`). Neither platform's compiled body reads this second parameter -
    /// passing `false` here is only to make Rust's `thiscall` codegen push the correct second stack
    /// dword so the real function's own `ret 8` pops the right number of bytes.
    pub fn subtract_cash(&mut self, amount: f32) {
        unsafe { SUBTRACT_CASH.original()((self as *mut Self) as *const u32, amount, false) }
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
    unsafe {
        (*ptr).cash += args[0].parse::<f32>()?;
    }
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
