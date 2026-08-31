//! `ZTGameMgr::MenuMusicHandler` reimplementation - see
//! `openzt/plans/menumusichandler-implementation-plan.md` for the full investigation. A **self-contained
//! leaf class**: every method is called through a fixed address (there is no `MenuMusicHandler` vtable),
//! so detouring this class's own 6 addresses transparently redirects every caller - all of which live in
//! un-ported, un-detoured `ZTGameMgr` methods (`initMenuMusic`/`startMenuMusic`/`startMenuMusicFade_*`)
//! that stay real vanilla code and never need to change.
//!
//! Vanilla-layout-compatible (style 1, per `CLAUDE.md`'s two-style split): the destructor is
//! deliberately left un-detoured (see the plan's Stage 5), so real, un-ported vanilla code still walks
//! this struct's raw memory directly to tear it down - `this+0x0` must therefore always hold a real,
//! vanilla-allocated `SNDSound*` (or `0`), which falls out naturally as long as every mutator here writes
//! through vanilla's own allocator/constructors rather than substituting Rust-owned state.
//!
//! **Stage 1 only** (struct + constructor - `init`/`startPlay`/`startFade`/`update` are later stages of
//! the same plan, deliberately not ported here).

use std::ffi::c_void;

use openzt_detour::generated::{
    bfinifile::READ,
    std_basic_string::{BASIC_STRING_0, BASIC_STRING_2},
    ztgamemgr_menumusichandler::MENU_MUSIC_HANDLER_1 as CONSTRUCTOR,
};
use openzt_detour_macro::detour_mod;
use tracing::error;

use crate::util::mut_from_memory;

/// Real allocation size `0x14`, confirmed directly by `ZTGameMgr_initMenuMusic.c`'s
/// `operator_new(0x14)` call (not merely inferred from the constructor's own field writes, which stop at
/// offset `0x10`) - see the implementation plan's struct-layout table for the per-field evidence.
#[repr(C)]
pub struct MenuMusicHandler {
    sound_ptr: u32,               // 0x0 - SNDSound*, 0 when none
    fading: u8,                   // 0x4
    _pad1: [u8; 3],
    fade_counter: i32,            // 0x8
    ini_menu_music_disabled: u8,  // 0xc
    _pad2: [u8; 3],
    warmup_ticks: i32,            // 0x10
}

const _: () = assert!(std::mem::size_of::<MenuMusicHandler>() == 0x14);

impl MenuMusicHandler {
    /// Reimplementation of `ZTGameMgr::MenuMusicHandler::MenuMusicHandler`, per
    /// `MenuMusicHandler_MenuMusicHandler.c`/`.asm`. Zeroes `sound_ptr`/`fading`/`fade_counter`/
    /// `warmup_ticks`, then resolves `ini_menu_music_disabled` from a real
    /// `BFIniFile::read("UI", "noMenuMusic", 0)` call - built from two temporary, real-vanilla-constructed
    /// `std::string`s ([`VanillaString`]), exactly matching vanilla's own stack-string construction, torn
    /// down again before returning.
    pub fn construct(&mut self) {
        self.sound_ptr = 0;
        self.fading = 0;
        self.fade_counter = 0;
        self.warmup_ticks = 0;

        let section = VanillaString::new("UI");
        let key = VanillaString::new("noMenuMusic");
        let result = unsafe { READ.original()(section.as_ptr(), key.as_ptr(), 0) };
        self.ini_menu_music_disabled = (result == 1) as u8;
    }
}

/// A real, vanilla-allocator-owned `std::string` - `{char* ptr; u32 len; u32 capacity}`, 12 bytes. This
/// build's std::string has no small-string-optimization buffer (confirmed directly from
/// `MenuMusicHandler_MenuMusicHandler.asm`'s stack-slot accounting: each of the constructor's two
/// temporary strings occupies exactly 3 stack dwords, matching this layout exactly), so it always
/// allocates its character buffer on the heap - built and torn down through the real vanilla
/// constructor/destructor (`std_basic_string::BASIC_STRING_2`/`BASIC_STRING_0`) rather than a hand-rolled
/// stand-in, matching `ztgamemgr-implementation-plan.md`'s `VanillaTagString` precedent (live-tested, then
/// reverted for unrelated reasons - see that plan's `removedZooDoo` section). Never write `ptr`/`len`/
/// `capacity` directly from Rust: construction and destruction always go through vanilla's own allocator,
/// so a Rust-side write here would risk exactly the cross-allocator hazard `CLAUDE.md` warns about.
#[repr(C)]
struct VanillaString {
    ptr: *mut u8,
    len: u32,
    capacity: u32,
}

impl VanillaString {
    /// Constructs from an iterator range `[str.as_ptr(), str.as_ptr() + str.len())`, matching
    /// `BASIC_STRING_2`'s real vanilla calling convention exactly (`this, first, last, allocator` - the
    /// trailing allocator argument is an uninitialized/unused stack slot in vanilla's own call site too,
    /// passed as `0` here).
    fn new(s: &str) -> Self {
        let mut this = VanillaString { ptr: std::ptr::null_mut(), len: 0, capacity: 0 };
        let start = s.as_ptr();
        let end = unsafe { start.add(s.len()) };
        unsafe {
            BASIC_STRING_2.original()(&mut this as *mut VanillaString as *const c_void, start as *const u32, end as i32, 0);
        }
        this
    }

    fn as_ptr(&self) -> *const u32 {
        self as *const VanillaString as *const u32
    }
}

impl Drop for VanillaString {
    fn drop(&mut self) {
        unsafe { BASIC_STRING_0.original()(self as *mut VanillaString as *const c_void) };
    }
}

#[detour_mod]
mod menu_music_handler_detours {
    use super::*;

    #[detour(CONSTRUCTOR)]
    unsafe extern "fastcall" fn constructor(this: *const u32) -> *const u32 {
        unsafe { mut_from_memory::<MenuMusicHandler>(this) }.construct();
        this
    }
}

/// Registers this module's live detours. Deliberately does **not** detour the destructor
/// (`MENU_MUSIC_HANDLER_0` in `generated.rs`) - see the module doc comment and the implementation plan's
/// Stage 5 for why.
pub fn init() {
    if let Err(e) = unsafe { menu_music_handler_detours::init_detours() } {
        error!("Failed to initialise ztgamemgr_menumusichandler detours: {e:?}");
    }
}

/// Live-comparison test support for `reimplementation_tests`.
#[cfg(feature = "reimplementation-tests")]
pub(crate) mod live_support {
    use openzt_detour::generated::standalone::{OPERATOR_DELETE, OPERATOR_NEW};

    use super::*;

    /// Allocates a fresh, uninitialized `0x14`-byte block via the real vanilla allocator - callers must
    /// run either the real constructor (`CONSTRUCTOR.original()`) or [`MenuMusicHandler::construct`] on it
    /// before reading any field.
    pub(crate) fn allocate_uninitialized() -> *mut MenuMusicHandler {
        unsafe { OPERATOR_NEW.original()(0x14) as *mut MenuMusicHandler }
    }

    /// Frees a standalone instance built via [`allocate_uninitialized`] plus (real or reimplemented)
    /// construction, **without calling the real vanilla destructor**. `MenuMusicHandler_~MenuMusicHandler.c`
    /// confirms its exported body is corrupted the same way the module doc comment's Stage-5 note
    /// describes: past the real, structurally-sound `sound_ptr` teardown (`if (sound_ptr) { if
    /// (isPlaying()) stop(); release(); }`), the export's tail calls real `operator_delete` on an
    /// `unaff_EDI` register the function's own body never assigns, then writes a `BFMgr` vtable pointer
    /// through an equally-unassigned `unaff_ESI` - genuinely uninitialized-register garbage from a
    /// misattributed tail call, not real `MenuMusicHandler` behavior. Calling `.original()` on this
    /// address would execute that garbage too, with unpredictable memory-corruption risk - so this helper
    /// never does. Safe only because every caller in this module's own tests goes through `construct()`,
    /// which never calls `init()` and therefore always leaves `sound_ptr` at `0` - once a later stage's
    /// tests exercise `init()`, a real `sound_ptr` teardown path (mirroring the export's *sound* first two
    /// lines, not its corrupted tail) will be needed here instead of a bare free.
    pub(crate) fn destroy_standalone(ptr: *mut MenuMusicHandler) {
        if ptr.is_null() {
            return;
        }
        unsafe { OPERATOR_DELETE.original()(ptr as *const u32) };
    }
}
