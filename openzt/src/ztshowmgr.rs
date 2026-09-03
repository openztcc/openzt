//! Stage 1 (struct + constructor + store) of the vanilla `ZTShowMgr` reimplementation - see
//! `openzt/plans/ztshowmgr-implementation-plan.md`. `ZTShowMgr` is the show-scheduling manager: it maps
//! show ids to the `ZTShowInfo` objects habitats register with it (`std::map<unsigned short,
//! ZTShowInfo*>` at `+0x28`), owns the config-loaded trick/show satisfaction thresholds
//! `ZTShow::doTrickEvent`/`calculatePercentAdjustment` read off the live global, and embeds a real
//! `ZTShowScriptMgr` inline at `+0x34` (four of its own methods are pure one-line delegations to that
//! sub-object; `ztshowscriptmgr.rs` already detours the sub-object's own addresses, so those wrappers
//! stay real vanilla code - see the plan's "Composition" section).
//!
//! **Style 2 - fully independent Rust store** (CLAUDE.md's decision criteria): a full decompile-corpus
//! grep (58 files referencing `GLOBAL_ZTShowMgr` outside the class's own, plan appendix) found every
//! external caller reaching the registered-shows map through named accessor methods
//! (`getShowInfo`/`getScript`/`getScriptID`/`registerShow`/`unregisterShow`) and never reading
//! `+0x28`/`+0x2c`/`+0x30` raw - so once those accessors are ported (stages 3+), vanilla's own map
//! becomes safely-inert dead weight that no Rust code below ever reads or writes. That choice also
//! sidesteps this class's one genuine allocator hazard outright: the map's `0x18`-byte nodes come from
//! the shared small-object freelist at `DAT_00638008` (`FUN_00402f85`/`FUN_00405002`/`FUN_00401b16`,
//! none of which has a `generated.rs` entry), a size-class freelist plausibly shared with unrelated
//! 24-byte allocations elsewhere in the game - allocating into it from Rust without the matching free
//! helpers would be exactly CLAUDE.md's cross-allocator corruption class.
//!
//! **The constructor is deliberately never detoured.** Vanilla's own ctor must keep running for the live
//! global (nothing else constructs `GLOBAL_ZTShowMgr`), and under style 2 its real work - taking a
//! freelist node for the map header and running the real config-driven `initShowParams` - is exactly
//! what the live global should keep doing. [`ZTShowMgr::construct`] below is therefore not a detour
//! target: it reproduces the constructor's config-independent writes so standalone instances can be
//! built and compared against the real ctor in the live battery (`ZTSHOWMGR_STANDALONE_ROUNDTRIP`),
//! mirroring `ztsoundscape.rs`'s own `construct`/roundtrip precedent. The one part it deliberately does
//! not reproduce is the freelist allocation itself: a standalone instance's `tree_header` stays null
//! (nothing reimplemented ever reads it - see the style-2 note above), so the test compares the real
//! side's node *shape* but excludes the pointer field itself.
//!
//! **Stage 2** adds the constructor's config-driven half: [`ZTShowMgr::init_show_params`] (the eight
//! `shows.cfg` threshold lookups, gated on expansion pack 2 through real
//! `BFApp::getInstalledExpansion`) detoured via `INIT_SHOW_PARAMS`, so the real, un-detoured
//! constructor's tail-call into it now runs the Rust port. Since the port writes the same defaults
//! vanilla writes and overrides from the same config, the live constructor's end state is unchanged.
//!
//! **Stage 3** makes [`SHOW_STORE`] a *shadow/mirror* of vanilla's tree rather than a replacement
//! (`ztshowmgr-implementation-plan.md`'s "Staging decision" - the read path and write path have to
//! agree on where the data lives, so the first-detoured side must mirror until the read cutover):
//! `registerShow`/`unregisterShow` are detoured, but each detour calls the real vanilla body through
//! its `<NAME>_DETOUR.call` trampoline first - vanilla keeps owning the real tree at `+0x28`, the
//! `DAT_0063e480` id counter, the `field_0x70` id writes, and `clearShowScriptStates` - and only on
//! the real body's success byte mirrors the outcome into [`SHOW_STORE`]. Live-game mutation behavior
//! is therefore exactly vanilla, while the store is a live-verified copy of the real tree.
//!
//! **Stage 4** flips the read path onto the store: `getShowInfo`/`getScriptID`
//! ([`ZTShowMgr::get_show_info`]/[`ZTShowMgr::get_script_id`]) are detoured and answer every caller -
//! Rust and un-decompiled vanilla alike - out of [`SHOW_STORE`]. Vanilla's own tree is thereby on its
//! way to inert: every external caller the corpus grep found goes through these two addresses
//! (`unregisterShow`'s own internal clear-target lookup included), so the readers always see the
//! in-sync mirror. From here on `GET_SHOW_INFO.original()`/`GET_SCRIPT_ID.original()` are
//! release-profile re-entry hazards like every other hooked address: in-repo call sites that want the
//! live behavior use `.hooked()` (`ztshow.rs`'s `start`, `ztshowui.rs`'s `show_info_for_habitat`), and
//! the battery's real-side poles go through the [`detours`] trampolines. Lock ordering stays safe: the
//! writers take the store's mutex only *after* their trampoline returns, and the readers'
//! mutex critical sections copy-and-unlock without calling out - even though vanilla bodies reached
//! through those trampolines now *do* re-enter the hooked `GET_SHOW_INFO` address internally. The
//! plan's open item - when (and whether) the writers drop their call-through to become full style-2
//! ports, taking the `DAT_0063e480` counter into Rust ownership together with stage 6's save/load -
//! is deliberately still open; the dual-write state is behavior-safe indefinitely while the mirror
//! stays in sync, which the battery's cross-store oracles assert after every mutation.
//!
//! **Stage 5** ports the two whole-map walks, `enterNewMonth`/`update`
//! ([`ZTShowMgr::enter_new_month`]/[`ZTShowMgr::update`]), onto the store - the first consumers
//! that iterate *everything* rather than probing one id. They stayed real vanilla through the
//! shadow phase precisely because they walked vanilla's still-authoritative tree; post-cutover the
//! `BTreeMap`'s ascending-`u16` order *is* vanilla's in-order sequence, so both walks become plain
//! iterations calling the same real, untouched `ZTShowInfo` callees vanilla calls (directly for
//! `enterNewMonth`, through the show's own vtable slot `+0x20` for `update`). Neither walk holds
//! [`SHOW_STORE`]'s lock across a callee call: the callees can re-enter the detoured
//! reader/writer addresses (a show's own update can start/stop shows), and the lock is only ever
//! taken to copy the value list out.
//!
//! **Stage 6** ports the save/load pair ([`ZTShowMgr::save`]/[`ZTShowMgr::load`]) - vanilla's thin
//! wrapper around two pieces: the embedded `ZTShowScriptMgr`'s own save/load (already
//! `ztshowscriptmgr.rs`'s Rust store, reached through the same direct `CALL` vanilla makes) and the
//! 2-byte `DAT_0063e480` show-id-counter persistence. Because the stage-3 writers still keep their
//! vanilla call-through, that counter stays vanilla-owned, so both ports read/write the global in
//! place - the vanilla-shaped half of the plan's stage-6 counter-ownership note (if the writers'
//! call-through is ever dropped, the counter moves into [`SHOW_STORE`] and these two ports must be
//! repointed at it).

use std::{
    collections::BTreeMap,
    mem::MaybeUninit,
    sync::{LazyLock, Mutex},
};

use crate::globals::get_module_base;
use crate::util::{get_from_memory, mut_from_memory};
use openzt_detour::generated::bfapp::GET_INSTALLED_EXPANSION;
use openzt_detour::generated::bfconfigfile::{CONSTRUCTOR_0, GET_INT, RELEASE};
use openzt_detour::generated::standalone::{DEALLOCATE, WRITE_BYTES_TO_FILE};
use openzt_detour::generated::ztshowinfo::ENTER_NEW_MONTH as ZTSHOWINFO_ENTER_NEW_MONTH;
use openzt_detour::generated::ztshowmgr::{
    ENTER_NEW_MONTH, GET_SCRIPT_ID, GET_SHOW_INFO, INIT_SHOW_PARAMS, LOAD, REGISTER_SHOW, SAVE, UNREGISTER_SHOW, UPDATE,
};
use openzt_detour::generated::ztshowscriptmgr::{LOAD as ZTSHOWSCRIPTMGR_LOAD, SAVE as ZTSHOWSCRIPTMGR_SAVE};
use openzt_detour_macro::detour_mod;
use tracing::error;

/// `GLOBAL_ZTApp`'s RVA - a raw pointer-typed global (one dereference gives the live `ZTApp*`
/// singleton), read by [`ZTShowMgr::init_show_params`] as the `this` for the expansion-2 gate call.
/// Re-declared per-file after `ztgamemgr.rs`'s own private `GLOBAL_ZTAPP_RVA` (same value, same
/// one-level-of-indirection shape) per the repo's no-shared-consts convention.
const GLOBAL_ZTAPP_RVA: u32 = 0x00638154 - 0x400000;

/// `.rdata` literals `init_show_params` passes to the real `BFConfigFile` functions
/// (`ZTShowMgr_initShowParams.asm`'s `PUSH s_*` targets, string contents confirmed by reading them
/// out of zoo.exe's `.rdata`). Data addresses, so each is stored as its Ghidra VA minus the
/// preferred base and resolved at runtime as `get_module_base("zoo.exe") + RVA` (CLAUDE.md's
/// data-address rule - see `ztsoundscape.rs`'s `CROWD_KEY_RVAS` for the same convention).
const SHOWS_CFG_FILENAME_RVA: u32 = 0x0063e488 - 0x400000; // "shows.cfg"
const TRICK_SATISFACTION_SECTION_RVA: u32 = 0x00642b38 - 0x400000; // "trickSatisfactionThresholds"
const SHOW_SECTION_RVA: u32 = 0x0063e494 - 0x400000; // "show"
const SHOW_SATISFACTION_SECTION_RVA: u32 = 0x00642adc - 0x400000; // "showSatisfactionThresholds"
const BAD_TRICK_KEY_RVA: u32 = 0x00642b54 - 0x400000;
const GOOD_TRICK_KEY_RVA: u32 = 0x00642b2c - 0x400000;
const GREAT_TRICK_KEY_RVA: u32 = 0x00642b20 - 0x400000;
const MIN_IDEAL_LENGTH_KEY_RVA: u32 = 0x00642b10 - 0x400000;
const MAX_IDEAL_LENGTH_KEY_RVA: u32 = 0x00642b00 - 0x400000;
const BAD_SHOW_KEY_RVA: u32 = 0x00642af8 - 0x400000;
const GOOD_SHOW_KEY_RVA: u32 = 0x00642ad0 - 0x400000;
const GREAT_SHOW_KEY_RVA: u32 = 0x00642ac4 - 0x400000;

/// The shared small-object freelist head (`DAT_0063800c`) that vanilla's inlined `~BFConfigFile`
/// tail returns the config's tree-root node to (see [`ZTShowMgr::init_show_params`]'s doc comment).
/// Note this is a *different* head from the `DAT_00638008` freelist backing `ZTShowMgr`'s own map
/// nodes - matching free idiom, different size class.
const CONFIG_FREELIST_HEAD_RVA: u32 = 0x0063800c - 0x400000;

/// `DAT_0063e480` - the show-id counter global behind `registerShow`'s inlined `makeID` logic
/// (16-bit `INC`, assigned id = `(u16)counter % 0xffff`, so `0xffff` is never assigned). Vanilla
/// `ZTShowMgr::save`/`load` persist exactly its low 2 bytes around the embedded `ZTShowScriptMgr`
/// delegation (stage 6). The counter stays vanilla-owned while the stage-3 writers keep their
/// call-through, so both ports read/write the global in place rather than through a Rust-owned
/// copy (see the plan's stage-6 counter-ownership note).
const SHOW_ID_COUNTER_RVA: u32 = 0x0063e480 - 0x400000;

/// `ZTShowMgr`'s real vtable VA (`private/docs/vtables/ZTShowMgr.md` - 2 slots: destructor, `update`),
/// written at `+0x0` by both real constructor paths (the transient `&BFMgr_vftable` placeholder the ctor
/// writes first is always overwritten before it returns, so only the final value is reproduced here).
/// Stored as `get_module_base + RVA` per CLAUDE.md's data-address rule.
const ZTSHOWMGR_VTABLE_RVA: u32 = 0x00635120 - 0x400000;

/// The embedded `ZTShowScriptMgr`'s own vtable VA (`ZTShowScriptMgr_ZTShowScriptMgr.c` names it
/// `ZTShowScriptMgr__vtable_00630d28`; the `.asm` names the same address `vf_returnTrue` - a 1-slot
/// vtable whose only entry returns true). Same data-address handling as [`ZTSHOWMGR_VTABLE_RVA`].
const ZTSHOWSCRIPTMGR_VTABLE_RVA: u32 = 0x00630d28 - 0x400000;

/// The `0x10`-byte `ZTShowScriptMgr` sub-object embedded at `ZTShowMgr+0x34` (its complete real size -
/// `CreateZTShowMgr` allocates the whole `ZTShowMgr` as `operator_new(0x44)`, leaving exactly `0x10`
/// bytes past `+0x34`). This mirrors its vanilla layout for structural fidelity only: the live
/// reimplementation of this sub-object is `ztshowscriptmgr.rs`'s fully independent Rust store, which
/// never reads or writes these bytes - real vanilla code owns them.
#[repr(C)]
pub struct ZTShowScriptMgrSlot {
    /// `+0x34` - `ZTShowScriptMgr`'s vtable (`ZTSHOWSCRIPTMGR_VTABLE_RVA`).
    pub vtable: u32,
    /// `+0x38` - the sub-object's own `std::map` header/nil-node pointer, a `0x18`-byte node taken from
    /// the same shared `DAT_00638008` freelist (confirmed via `ZTShowScriptMgr_ZTShowScriptMgr.asm`).
    /// Standalone constructions leave this null - see [`ZTShowMgr::construct`]'s doc comment.
    pub tree_header: u32,
    /// `+0x3c` - the sub-object's map size (zeroed by the real ctor).
    pub tree_size: u32,
    /// `+0x40` - see [`ZTShowMgr::tag_byte`] for the (odd) value the real ctor writes here.
    pub tag_byte: u8,
    _pad_0x41: [u8; 3],
}

const _: () = assert!(std::mem::size_of::<ZTShowScriptMgrSlot>() == 0x10);

/// Full, `#[repr(C)]` mirror of vanilla `ZTShowMgr` - real size `0x44` (`CreateZTShowMgr.c`'s own
/// `operator_new(0x44)`), size-asserted. Only ever *read* by the reimplementations that need the live
/// global's threshold fields (`ztshow.rs`'s `do_trick_event`); the Rust store below never reads or
/// writes any of it. The two `tag_byte` fields reproduce, byte-faithfully but unexplained, what both
/// real constructors write there: the **high byte of the instance's own address** (the `.asm` reads it
/// off the stack as `[ESP+0xb]`/`[ESP+0x7]` - the saved `ECX`, i.e. `this` - in both
/// `ZTShowMgr_ZTShowMgr.asm` and `ZTShowScriptMgr_ZTShowScriptMgr.asm`; semantics unknown).
#[repr(C)]
pub struct ZTShowMgr {
    /// `+0x0` - `ZTShowMgr`'s vtable.
    pub vtable: u32,
    /// `+0x4` - zeroed by the real ctor; semantics unknown.
    pub field_0x4: u8,
    _pad_0x5: [u8; 3],
    /// `+0x8` - bad-trick satisfaction threshold (`initShowParams`: `[trickSatisfactionThresholds]
    /// badTrick`, default `0`). Read as `threshold_a` by `ztshow.rs`'s `do_trick_event`.
    pub threshold_a: u32,
    /// `+0xc` - good-trick satisfaction threshold (`goodTrick`, default `3`). Read as `threshold_b` by
    /// `do_trick_event`.
    pub threshold_b: u32,
    /// `+0x10` - great-trick satisfaction threshold (`greatTrick`, default `6`). Read as `threshold_c`
    /// by `do_trick_event`.
    pub threshold_c: u32,
    /// `+0x14` - bad-show satisfaction threshold (`[showSatisfactionThresholds] badShow`, default `0x19`).
    pub bad_show: u32,
    /// `+0x18` - good-show satisfaction threshold (`goodShow`, default `0x32`).
    pub good_show: u32,
    /// `+0x1c` - great-show satisfaction threshold (`greatShow`, default `0x4b`).
    pub great_show: u32,
    /// `+0x20` - minimum ideal show length (`[show] minIdealLength`, default `6`). Both ideal-length
    /// fields bound `ZTShow::calculatePercentAdjustment`'s trick-count adjustment (read raw at these
    /// offsets by `ztshow.rs`).
    pub min_ideal_length: u32,
    /// `+0x24` - maximum ideal show length (`maxIdealLength`, default `6`).
    pub max_ideal_length: u32,
    /// `+0x28` - the registered-shows `std::map`'s header/nil-node pointer (a `0x18`-byte node from the
    /// shared `DAT_00638008` freelist; self-referential when empty). The Rust store below replaces every
    /// consumer of this tree - standalone constructions leave the field null.
    pub tree_header: u32,
    /// `+0x2c` - registered-shows map size (the destructor's and `update`'s own empty-check field).
    pub tree_size: u32,
    /// `+0x30` - high byte of this instance's own address, per the real ctor's stack read. Semantics
    /// unknown; reproduced faithfully, never read by anything reimplemented.
    pub tag_byte: u8,
    _pad_0x31: [u8; 3],
    /// `+0x34` - the embedded, real-vanilla-owned `ZTShowScriptMgr` sub-object.
    pub show_script_mgr: ZTShowScriptMgrSlot,
}

const _: () = assert!(std::mem::size_of::<ZTShowMgr>() == 0x44);

impl ZTShowMgr {
    /// Reimplementation of `ZTShowMgr::ZTShowMgr`'s config-independent parts, per
    /// `ZTShowMgr_ZTShowMgr.c`/`.asm` (field writes verified against the `.asm`, not just the decompiled
    /// `.c`): zeroing `+0x4`, the eight `initShowParams` defaults (which the real `initShowParams`
    /// writes *before* its expansion-gated `BFConfigFile` override - stage 2 ports that override), both
    /// map sizes, both vtable writes, and both `tag_byte` self-address writes.
    ///
    /// Deliberately **not** reproduced: the freelist allocations backing `+0x28`/`+0x38` (this instance's
    /// map headers stay null - no reimplemented code ever reads them, and allocating a lookalike node
    /// without the matching freelist free helpers would set up the cross-allocator hazard the style-2
    /// decision exists to avoid). Callers get a fully zeroed struct first (the real ctor runs over
    /// un-zeroed `operator_new` memory and simply leaves its own padding untouched; zeroing the padding
    /// here is the deterministic equivalent for a test instance, matching the roundtrip test's
    /// pre-zeroed-buffer precedent). Not a detour target - see the module doc comment.
    pub fn construct(&mut self) -> &mut Self {
        let base = get_module_base("zoo.exe") as u32;
        let this_addr = self as *const ZTShowMgr as u32;

        self.vtable = base + ZTSHOWMGR_VTABLE_RVA;
        self.field_0x4 = 0;
        self.threshold_a = 0;
        self.threshold_b = 3;
        self.threshold_c = 6;
        self.bad_show = 0x19;
        self.good_show = 0x32;
        self.great_show = 0x4b;
        self.min_ideal_length = 6;
        self.max_ideal_length = 6;
        self.tree_header = 0;
        self.tree_size = 0;
        self.tag_byte = (this_addr >> 24) as u8;

        self.show_script_mgr.vtable = base + ZTSHOWSCRIPTMGR_VTABLE_RVA;
        self.show_script_mgr.tree_header = 0;
        self.show_script_mgr.tree_size = 0;
        self.show_script_mgr.tag_byte = (this_addr >> 24) as u8;

        self
    }

    /// Reimplementation of `ZTShowMgr::initShowParams` (`0x0051f59b`), per
    /// `ZTShowMgr_initShowParams.asm` (call targets confirmed against the binary itself: the ctor
    /// call lands on `bfconfigfile::CONSTRUCTOR_0` at `0x004b4516`, all eight key lookups on
    /// `GET_INT` at `0x00409c14`, and the teardown on `RELEASE` at `0x0040a5bc`):
    ///
    /// 1. Write the eight config defaults (same values [`ZTShowMgr::construct`] writes - vanilla
    ///    writes them here too, so the defaults survive untouched whenever the config half is
    ///    skipped below).
    /// 2. Gate on "expansion pack 2 installed" - ported as a call-through to the real
    ///    `BFApp::getInstalledExpansion` (`GET_INSTALLED_EXPANSION`, `0x004ab32c`) on the live
    ///    `GLOBAL_ZTApp` singleton, per the plan's finding that the Windows `.asm`'s inlined
    ///    `word ptr [app + 0x444] & 2` read is exactly that method's own body. Real vanilla reads
    ///    `GLOBAL_ZTApp` with a "if null, lazily assign it a bogus `ZTApp::handleMessages`
    ///    function-pointer sentinel before re-reading" defensive branch first - deliberately not
    ///    reproduced, same precedent as `ztgamemgr.rs`'s `stop` (a null app global here means
    ///    "app not ready"; this port treats that as "skip the config override" rather than writing
    ///    a code address into live global state to match a never-taken vanilla path).
    /// 3. If gated in: construct a **stack-local** real `BFConfigFile` over `shows.cfg` (the same
    ///    `CONSTRUCTOR_0`/`RELEASE` pair the live-test harness already uses), and only if it loaded,
    ///    run all eight `GET_INT` lookups in vanilla's exact call order into the eight threshold
    ///    fields.
    /// 4. Release the config, then reproduce `~BFConfigFile`'s inlined dtor tail
    ///    (`ZTShowMgr_initShowParams.asm`'s `.180` block): return the config ctor's tree-root node
    ///    to the shared small-object freelist it came from (`DAT_0063800c`, plain
    ///    `*node = head; head = node`). Skipping this would leak that node every call; the push is
    ///    vanilla's own free idiom on a vanilla-allocated node, so no cross-allocator hazard (see
    ///    the module doc - the hazard this module avoids is *allocating* Rust objects into a
    ///    freelist, never returning vanilla's own nodes through vanilla's own stores). The asm's
    ///    post-release key-list walk is dead code (its flag was already cleared by `release`'s own
    ///    walk, which `BFConfigFile_release.c` shows frees that same list) and is not reproduced.
    ///
    /// Returns `1` - the real body's only return write is `MOV AL, 0x1` (upper EAX bits are leftover
    /// register garbage there; this port returns a clean `1`, which no caller distinguishes - its one
    /// caller, the constructor, ignores the return).
    pub fn init_show_params(&mut self) -> u32 {
        self.threshold_a = 0;
        self.threshold_b = 3;
        self.threshold_c = 6;
        self.bad_show = 0x19;
        self.good_show = 0x32;
        self.great_show = 0x4b;
        self.min_ideal_length = 6;
        self.max_ideal_length = 6;

        let base = get_module_base("zoo.exe") as u32;
        let ztapp_ptr: u32 = get_from_memory(base + GLOBAL_ZTAPP_RVA);
        let expansion_2_installed =
            ztapp_ptr != 0 && unsafe { GET_INSTALLED_EXPANSION.original()(ztapp_ptr as *const u32, 2) } != 0;

        if expansion_2_installed {
            let config = MaybeUninit::<crate::bfconfigfile::BFConfigFile>::uninit();
            let config_ptr = config.as_ptr() as *const u32;
            unsafe {
                CONSTRUCTOR_0.original()(config_ptr, (base + SHOWS_CFG_FILENAME_RVA) as *const u8);
            }

            // `BFConfigFile`'s "has data" flag at +0x4 (see `crate::bfconfigfile::BFConfigFile`) -
            // vanilla skips every lookup when the file didn't load.
            if get_from_memory::<i32>(config_ptr as u32 + 0x4) != 0 {
                unsafe {
                    GET_INT.original()(
                        config_ptr,
                        base + TRICK_SATISFACTION_SECTION_RVA,
                        base + BAD_TRICK_KEY_RVA,
                        &raw mut self.threshold_a as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + TRICK_SATISFACTION_SECTION_RVA,
                        base + GOOD_TRICK_KEY_RVA,
                        &raw mut self.threshold_b as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + TRICK_SATISFACTION_SECTION_RVA,
                        base + GREAT_TRICK_KEY_RVA,
                        &raw mut self.threshold_c as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + SHOW_SECTION_RVA,
                        base + MIN_IDEAL_LENGTH_KEY_RVA,
                        &raw mut self.min_ideal_length as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + SHOW_SECTION_RVA,
                        base + MAX_IDEAL_LENGTH_KEY_RVA,
                        &raw mut self.max_ideal_length as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + SHOW_SATISFACTION_SECTION_RVA,
                        base + BAD_SHOW_KEY_RVA,
                        &raw mut self.bad_show as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + SHOW_SATISFACTION_SECTION_RVA,
                        base + GOOD_SHOW_KEY_RVA,
                        &raw mut self.good_show as *const u32,
                    );
                    GET_INT.original()(
                        config_ptr,
                        base + SHOW_SATISFACTION_SECTION_RVA,
                        base + GREAT_SHOW_KEY_RVA,
                        &raw mut self.great_show as *const u32,
                    );
                }
            }

            unsafe { RELEASE.original()(config_ptr) };

            // ~BFConfigFile's inlined dtor tail: hand the ctor's tree-root node back to the
            // freelist head it was popped from (see this method's doc comment).
            let tree_root: u32 = get_from_memory(config_ptr as u32);
            if tree_root != 0 {
                let freelist_head = (base + CONFIG_FREELIST_HEAD_RVA) as *mut u32;
                unsafe {
                    let head = *freelist_head;
                    *(tree_root as *mut u32) = head;
                    *freelist_head = tree_root;
                }
            }
        }

        1
    }

    /// Stage 3 shadow/mirror of `ZTShowMgr::registerShow` (`0x005abb26`, per
    /// `ZTShowMgr_registerShow.asm`; `ztshowmgr-implementation-plan.md` stage 3): calls the real
    /// vanilla body through the [`detours::call_real_register_show`] trampoline - vanilla keeps
    /// owning the real tree at `+0x28`, the `DAT_0063e480` id counter, and the `field_0x70` id write,
    /// all deliberately not reimplemented this phase - then, only on the real body's success byte,
    /// mirrors the resulting id -> show pair into [`SHOW_STORE`].
    ///
    /// Vanilla shape, `.asm`-verified: a null `show` returns `AL=0` before touching anything;
    /// otherwise a find on the show's *current* `field_0x70` runs first (already present -> `AL=0`,
    /// nothing written, even with the force flag set); only then does the force flag **or**
    /// `field_0x70 == 0` assign a fresh id (`INC word ptr DAT_0063e480`, id =
    /// `(u16)counter % 0xffff` - so `0xffff` is never assigned, wrapping to `0`) through the real
    /// `field_0x70` setter; the final tree write is insert-or-assign, so a force re-register under a
    /// *different* key leaves the show's previous (stale) entry in place - [`SHOW_STORE`]'s own
    /// `BTreeMap::insert` mirrors that exactly. The real body reports success in `AL` only (upper
    /// EAX bits are leftover register garbage there, its `MOV %AL,%BL` exit); this returns it cleaned
    /// to a full `0`/`1`, which every caller observes through the low byte alone.
    pub fn register_show(&mut self, show: *const u32, force: bool) -> u32 {
        let real_return = detours::call_real_register_show(self as *const ZTShowMgr as *const u32, show, force);
        let succeeded = real_return & 0xff != 0;
        if succeeded {
            // `AL == 1` implies `show` is non-null (vanilla's null-show path exits before anything
            // else runs), and both success paths end with the inserted key sitting in the show's own
            // `field_0x70` (a preset id keeps it; a fresh id is written there by the real setter) -
            // so reading it back here yields exactly the key vanilla inserted.
            let show_addr = show as u32;
            let id = get_from_memory::<u16>(show_addr + 0x70);
            SHOW_STORE.lock().unwrap().registered_shows.insert(id, show_addr);
        }
        succeeded as u32
    }

    /// Stage 3 shadow/mirror of `ZTShowMgr::unregisterShow` (`0x005aaa95`, per
    /// `ZTShowMgr_unregisterShow.asm`/`.c`): calls the real vanilla body through the
    /// [`detours::call_real_unregister_show`] trampoline - vanilla keeps owning the tree erase and
    /// the `clearShowScriptStates` call - then, only on the real body's success byte, removes the
    /// effective id from [`SHOW_STORE`].
    ///
    /// Vanilla's path matrix, all verified against the `.c`/`.asm`: null `show` + null `id` returns
    /// `AL=0` (nothing mirrored); `show != 0 && id == 0` - the one path that never reads
    /// `GLOBAL_ZTShowMgr` - derives the id from the show's own `field_0x70` (deliberately *stale*
    /// after a prior unregister, since vanilla never zeroes that field, so the mirror derives the
    /// same way) and, with the clear flag set, runs the real `clearShowScriptStates` on the show's
    /// embedded `ZTShow` directly; both `id != 0` paths (with or without a show pointer) end in an
    /// unconditional `getShowInfo(GLOBAL_ZTShowMgr, id)` used only to pick the clear target - where
    /// the `show != 0` variant first calls `getShowInfo(this, id)` and *discards* the result, a
    /// verified vanilla wart - and that lookup faults if the global is null (`getShowInfo` has no
    /// null guard), exactly as real vanilla would. Success is `AL=1` even when the key was absent
    /// (a silent no-op erase), which `BTreeMap::remove`'s own absent-key behavior mirrors. Upper
    /// EAX bits are garbage; returned cleaned to `0`/`1` like [`ZTShowMgr::register_show`].
    pub fn unregister_show(&mut self, id: u16, show: *const u32, clear: bool) -> u32 {
        let real_return = detours::call_real_unregister_show(self as *const ZTShowMgr as *const u32, id, show, clear);
        let succeeded = real_return & 0xff != 0;
        if succeeded {
            // Success excludes the null-show + null-id case (that path exits `AL=0`), so when `id`
            // is zero here `show` is necessarily non-null and vanilla's own derivation applies:
            // remove the key the show's `field_0x70` still carries.
            let effective_id = if id != 0 { id } else { get_from_memory::<u16>(show as u32 + 0x70) };
            SHOW_STORE.lock().unwrap().registered_shows.remove(&effective_id);
        }
        succeeded as u32
    }

    /// Stage 4 read cutover of `ZTShowMgr::getShowInfo` (`0x0041ebfd`, per
    /// `ZTShowMgr_getShowInfo.asm`): the registered-shows lookup, answered out of [`SHOW_STORE`]
    /// instead of vanilla's `+0x28` tree. Vanilla shape, `.asm`-verified: a standard MSVC lower-bound
    /// walk over the u16-keyed tree returning the found node's `+0x14` value, clean `0` on a miss.
    /// A registered-but-null value is indistinguishable from a miss in vanilla; the store can never
    /// hold a null value (only a real body's success byte inserts), so
    /// `get().copied().unwrap_or(0)` reproduces the mapping exactly - unsigned `u16` key order
    /// matches vanilla's unsigned compare, which is what makes `BTreeMap<u16>` a drop-in.
    ///
    /// `this` is deliberately unread - one real instance, one process-global store (see
    /// [`SHOW_STORE`]). Where vanilla's own body faults on a null `this` (unguarded `[ECX+0x28]`
    /// read), this reader has no `this` to fault on, which is also what keeps `unregisterShow`'s
    /// global-targeted internal lookups safe against the battery's standalone instances.
    pub fn get_show_info(id: u16) -> u32 {
        SHOW_STORE.lock().unwrap().registered_shows.get(&id).copied().unwrap_or(0)
    }

    /// Stage 4 read cutover of `ZTShowMgr::getScriptID` (`0x005a2665`, per
    /// `ZTShowMgr_getScriptID.asm`): [`ZTShowMgr::get_show_info`] plus the found `ZTShowInfo`'s own
    /// assigned-script-id field at `+0x8` (the same field `ztshow.rs`'s `check_unit_type` reads).
    /// The real body's found path (`MOV %AX, word ptr [EAX+0x8]`, no `movzx`) leaves EAX's upper
    /// half holding the upper half of the `getShowInfo` return - live-verified in
    /// `ZTSHOWMGR_GET_SHOW_INFO_GET_SCRIPT_ID` - which is unobservable by every caller (its only
    /// chain, `ZTUnit::getShowItem` -> `ZTUnitType::getShowItem` -> `ZTShowMgr::getScript`, ends in
    /// `ZTShowScriptMgr::getScript`, which masks the id back to 16 bits before its own tree walk),
    /// so this returns the u16 cleanly zero-extended. `0` when the show isn't registered -
    /// indistinguishable there from a found show whose `+0x8` is `0`, exactly like vanilla.
    pub fn get_script_id(id: u16) -> u32 {
        let show_info = Self::get_show_info(id);
        if show_info != 0 {
            get_from_memory::<u16>(show_info + 0x8) as u32
        } else {
            0
        }
    }

    /// Stage 5 port of `ZTShowMgr::enterNewMonth` (`0x004842a2`, per `ZTShowMgr_enterNewMonth.asm`/
    /// `.c`): the monthly tick (`ZooStatus::financeChecks` is the corpus's one caller) running real,
    /// untouched `ZTShowInfo::enterNewMonth` (`0x0048b57e`) on every registered, non-null show.
    /// Vanilla traverses its `+0x28` tree with the standard `_Tree::_Inc` successor walk starting at
    /// the leftmost node; iterating [`SHOW_STORE`]'s `BTreeMap` yields that same in-order sequence
    /// (ascending unsigned `u16` key - the property that made it a stage-4 drop-in), with vanilla's
    /// `left == header` empty case covered by the empty vector.
    ///
    /// The callee call reproduces vanilla's direct `CALL ZTShowInfo::enterNewMonth` via `.hooked()`
    /// rather than `.original()`: a direct CALL executes whatever sits at the raw address, so this
    /// walk must route like vanilla's own callers in every build profile - identical today (the
    /// address is un-detoured; `.original()` is also a raw cast here in release), and correctly
    /// re-routed through the detour everywhere if `ztshowinfo::ENTER_NEW_MONTH` is ever hooked
    /// (`.original()`'s debug-build trampoline routing would silently diverge from vanilla callers).
    pub fn enter_new_month() {
        for show in Self::registered_show_values() {
            if show != 0 {
                unsafe { ZTSHOWINFO_ENTER_NEW_MONTH.hooked()(show as *const u32) };
            }
        }
    }

    /// Stage 5 port of `ZTShowMgr::update` (`0x00434e1e`, per `ZTShowMgr_update.asm`/`.c` - the
    /// decompile shows the full body; the `.asm` is just the guard tail-jumping to the shared walk
    /// code at `0x0059e6e3`): `update`'s own `0 < (int)mbr_0x2c` size guard (map sizes are never
    /// negative, so `!is_empty()` is the exact equivalent, and also subsumes vanilla's second
    /// `left == header` empty check), then the same in-order walk as
    /// [`ZTShowMgr::enter_new_month`] - reached through `ZTShowMgr`'s own 2-slot vtable
    /// (`private/docs/vtables/ZTShowMgr.md`) by the AI manager's periodic update loop.
    ///
    /// The per-show call is reproduced as the same **virtual dispatch** vanilla does
    /// (`(**(code **)(**value + 0x20))()`): the callee comes out of the show object's *own* vtable
    /// pointer at slot `+0x20` (`ztshowinfo::UPDATE`, `0x0059e725`, confirmed against
    /// `private/docs/vtables/ZTShowInfo.md`), not from a hardcoded address - so a subclass slot
    /// override or a future detour of `ztshowinfo::UPDATE` is picked up exactly as a vanilla
    /// caller would pick it up.
    pub fn update() {
        let shows = Self::registered_show_values();
        if shows.is_empty() {
            return;
        }
        for show in shows {
            if show != 0 {
                let vtable: u32 = get_from_memory(show);
                let slot: u32 = get_from_memory(vtable + 0x20);
                let update_fn: unsafe extern "thiscall" fn(*const u32) = unsafe { std::mem::transmute(slot) };
                unsafe { update_fn(show as *const u32) };
            }
        }
    }

    /// Stage 6 port of `ZTShowMgr::save` (`0x00479fa4`, per `ZTShowMgr_save.asm`/`.c`; macOS's
    /// `ZTShowMgr_save.c` agrees): delegates to the embedded `ZTShowScriptMgr`'s own save, then
    /// persists the 2-byte show-id counter global ([`SHOW_ID_COUNTER_RVA`]) straight off its
    /// address through real `WriteBytesToFile`. The registered-shows map itself is deliberately
    /// *not* serialized - every `ZTShowInfo` re-registers through its own separate load path - so
    /// the counter bytes are this method's only own payload. Vanilla runs the counter write even
    /// when the delegation fails and combines the two with plain `&` (no short-circuit), which is
    /// reproduced. The delegation goes through a direct `CALL ZTShowScriptMgr::save`, so it is
    /// routed via `.hooked()` (same direct-address rule as [`ZTShowMgr::enter_new_month`]): a
    /// direct CALL executes whatever sits at the raw address, which is `ztshowscriptmgr.rs`'s
    /// Rust detour once installed. Returns `0`/`1` - the real body only guarantees its
    /// `SETZ`-produced `AL`; upper EAX is leftover register garbage there.
    pub fn save(&mut self, file: *const i8) -> u32 {
        let script_ok =
            unsafe { ZTSHOWSCRIPTMGR_SAVE.hooked()(&raw const self.show_script_mgr as *const u32, file) } & 0xff != 0;
        let counter_addr = (get_module_base("zoo.exe") as u32 + SHOW_ID_COUNTER_RVA) as *const u32;
        let write_ok = unsafe { WRITE_BYTES_TO_FILE.hooked()(counter_addr, 2, 1, file) };
        (script_ok & write_ok) as u32
    }

    /// Stage 6 port of `ZTShowMgr::load` (`0x004c6f54`, per `ZTShowMgr_load.asm`/`.c`; macOS's
    /// `ZTShowMgr_load.c` agrees): the mirror of [`ZTShowMgr::save`] - same `ZTShowScriptMgr`
    /// delegation through `.hooked()` (same direct-CALL routing) - then, only for
    /// `version > 0x60` (the `.asm` gates on unsigned `CMP %ESI, 0x61` / `JC`), reads the 2-byte
    /// counter back into [`SHOW_ID_COUNTER_RVA`] through real `deallocate` and ANDs the read's
    /// success into the result. The read is attempted whenever the version gate passes - even
    /// after a failed delegation - exactly like vanilla. Returns `0`/`1` cleaned like
    /// [`ZTShowMgr::save`].
    pub fn load(&mut self, file: *const u32, version: u32) -> u32 {
        let mut ok =
            unsafe { ZTSHOWSCRIPTMGR_LOAD.hooked()(&raw const self.show_script_mgr as *const u32, file, version) } & 0xff
                != 0;
        if version > 0x60 {
            let counter_addr = (get_module_base("zoo.exe") as u32 + SHOW_ID_COUNTER_RVA) as *const u32;
            let read_ok = unsafe { DEALLOCATE.hooked()(counter_addr, 2, 1, file as *const u8) } == 1;
            ok &= read_ok;
        }
        ok as u32
    }

    /// Copy of the store's registered values, in the `BTreeMap`'s ascending-`u16`-key order. The
    /// lock is held only for the copy - never across the walks' callee calls, which can re-enter
    /// the detoured `GET_SHOW_INFO`/`REGISTER_SHOW`/`UNREGISTER_SHOW` addresses (event processing
    /// inside a show's own `update` can start/stop shows) and would deadlock against the writers'
    /// post-trampoline mirroring. A store mutation that happens mid-walk (from a callee) is
    /// therefore picked up on the *next* walk, exactly as vanilla's own tree walk would have been
    /// at worst similarly stale - no vanilla caller depends on same-walk visibility of a
    /// callee-triggered registration change.
    fn registered_show_values() -> Vec<u32> {
        SHOW_STORE.lock().unwrap().registered_shows.values().copied().collect()
    }
}

/// Stage 2's detour: `INIT_SHOW_PARAMS` (`0x0051f59b`). The real constructor
/// (`0x0051f73a`, never detoured - see the module doc) tail-calls this address
/// (`CALL 0x0051f59b` at `0x0051f793`, binary-confirmed), so hooking it routes the live global's
/// config-driven construction through the Rust port; it writes the same defaults vanilla writes
/// and overrides from the same `shows.cfg`, so the live constructor's end state is unchanged.
/// The decompile corpus has no other caller (`ZTShowMgr_ZTShowMgr.c` is the only reference).
///
/// Stage 3 adds the registered-shows write path to the same block: `REGISTER_SHOW`/`UNREGISTER_SHOW`
/// (`0x005abb26`/`0x005aaa95`, no other `openzt/src` callers of either address) as shadow/mirror
/// detours - see [`ZTShowMgr::register_show`]/[`ZTShowMgr::unregister_show`]. Vanilla keeps owning
/// the real tree; the store mirrors it.
///
/// Stage 4 adds the read cutover: `GET_SHOW_INFO`/`GET_SCRIPT_ID` (`0x0041ebfd`/`0x005a2665`)
/// route every caller - including un-decompiled vanilla code and the real `unregisterShow` body's
/// own internal lookups, which reach `getShowInfo` by raw address - onto the store-backed readers.
///
/// Stage 5 adds the two whole-map walks: `ENTER_NEW_MONTH`/`UPDATE` (`0x004842a2`/`0x00434e1e`)
/// iterate [`SHOW_STORE`] and call the same real `ZTShowInfo` callees vanilla's own walks call -
/// see [`ZTShowMgr::enter_new_month`]/[`ZTShowMgr::update`]. The live callers this puts on the
/// Rust path are `ZooStatus::financeChecks` (monthly, direct call) and the AI manager's periodic
/// vtable dispatch of `update`.
///
/// Stage 6 adds the save/load pair: `SAVE`/`LOAD` (`0x00479fa4`/`0x004c6f54`) - the real
/// save-game flow (`ZTWorldMgr::save`/`load`, the corpus's only callers) now runs the Rust script
/// store's save/load through the same direct `CALL` vanilla makes and persists the vanilla-owned
/// show-id counter around it - see [`ZTShowMgr::save`]/[`ZTShowMgr::load`].
#[detour_mod]
mod detours {
    use super::*;

    #[detour(INIT_SHOW_PARAMS)]
    unsafe extern "thiscall" fn init_show_params_detour(this: *const u32) -> u32 {
        unsafe { mut_from_memory::<ZTShowMgr>(this).init_show_params() }
    }

    #[detour(REGISTER_SHOW)]
    unsafe extern "thiscall" fn register_show_detour(this: *const u32, show: *const u32, force: bool) -> u32 {
        unsafe { mut_from_memory::<ZTShowMgr>(this).register_show(show, force) }
    }

    #[detour(UNREGISTER_SHOW)]
    unsafe extern "thiscall" fn unregister_show_detour(this: *const u32, id: u16, show: *const u32, clear: bool) -> u32 {
        unsafe { mut_from_memory::<ZTShowMgr>(this).unregister_show(id, show, clear) }
    }

    /// Stage 4 read cutover - see [`ZTShowMgr::get_show_info`]. The instance pointer is deliberately
    /// unread (single process-global store), so unlike the other detours here it is dropped rather
    /// than mapped onto a `ZTShowMgr` - a null `this`, which vanilla's own body would fault on,
    /// resolves through the same store as any other.
    #[detour(GET_SHOW_INFO)]
    unsafe extern "thiscall" fn get_show_info_detour(_this: *const u32, id: u16) -> u32 {
        ZTShowMgr::get_show_info(id)
    }

    /// Stage 4 read cutover - see [`ZTShowMgr::get_script_id`]. Same dropped-`this` reasoning as
    /// [`get_show_info_detour`].
    #[detour(GET_SCRIPT_ID)]
    unsafe extern "thiscall" fn get_script_id_detour(_this: *const u32, id: u16) -> u32 {
        ZTShowMgr::get_script_id(id)
    }

    /// Stage 5 walk - see [`ZTShowMgr::enter_new_month`]. Dropped `this`, same single-store
    /// reasoning as [`get_show_info_detour`].
    #[detour(ENTER_NEW_MONTH)]
    unsafe extern "thiscall" fn enter_new_month_detour(_this: *const u32) {
        ZTShowMgr::enter_new_month()
    }

    /// Stage 5 walk - see [`ZTShowMgr::update`]. Dropped `this`, same single-store reasoning as
    /// [`get_show_info_detour`].
    #[detour(UPDATE)]
    unsafe extern "thiscall" fn update_detour(_this: *const u32) {
        ZTShowMgr::update()
    }

    /// Stage 6 save - see [`ZTShowMgr::save`]. The instance is used only to derive the embedded
    /// `ZTShowScriptMgr` sub-object address the delegation receives (its own detour ignores it -
    /// the Rust store is process-global), matching vanilla's `ADD %ECX, 0x34` hand-off.
    #[detour(SAVE)]
    unsafe extern "thiscall" fn save_detour(this: *const u32, file: *const i8) -> u32 {
        unsafe { mut_from_memory::<ZTShowMgr>(this).save(file) }
    }

    /// Stage 6 load - see [`ZTShowMgr::load`]. Same sub-object hand-off as [`save_detour`].
    #[detour(LOAD)]
    unsafe extern "thiscall" fn load_detour(this: *const u32, file: *const u32, version: u32) -> u32 {
        unsafe { mut_from_memory::<ZTShowMgr>(this).load(file, version) }
    }

    /// The real vanilla body through the detour's trampoline - the only release-safe way back to
    /// vanilla once this detour has patched the address (`FunctionDef::original()` is a raw address
    /// cast in release and would silently re-enter [`init_show_params_detour`]). Used by the live
    /// battery's `ZTSHOWMGR_INIT_SHOW_PARAMS` real-side pole.
    pub(super) fn call_real(this: *const u32) -> u32 {
        unsafe { INIT_SHOW_PARAMS_DETOUR.call(this) }
    }

    /// The real vanilla `registerShow` through the detour's trampoline - same release-safety
    /// reasoning as [`call_real`]. This is [`ZTShowMgr::register_show`]'s shadow/mirror call-through.
    pub(super) fn call_real_register_show(this: *const u32, show: *const u32, force: bool) -> u32 {
        unsafe { REGISTER_SHOW_DETOUR.call(this, show, force) }
    }

    /// Same mechanism for the real `unregisterShow` - see [`call_real_register_show`].
    pub(super) fn call_real_unregister_show(this: *const u32, id: u16, show: *const u32, clear: bool) -> u32 {
        unsafe { UNREGISTER_SHOW_DETOUR.call(this, id, show, clear) }
    }

    /// The real vanilla `getShowInfo` through the detour's trampoline - the only route back to the
    /// genuine tree walk now that stage 4 hooks the address. This is the battery's real-side pole
    /// (`ZTSHOWMGR_REGISTER_UNREGISTER_SHOW`'s diff oracle and
    /// `ZTSHOWMGR_GET_SHOW_INFO_GET_SCRIPT_ID`'s vanilla pole): during the dual-write phase it must
    /// agree with [`SHOW_STORE`] after every mutation, which is exactly what those tests assert.
    pub(super) fn call_real_get_show_info(this: *const u32, id: u16) -> u32 {
        unsafe { GET_SHOW_INFO_DETOUR.call(this, id) }
    }

    /// Same mechanism for the real `getScriptID`. Note its real body reaches `getShowInfo` by raw
    /// address (`CALL ZTShowMgr::getShowInfo` in `ZTShowMgr_getScriptID.asm`), so post-cutover this
    /// pole is vanilla glue around the *detoured* reader - it verifies the real ABI glue and the
    /// real `+0x8` read on top of [`SHOW_STORE`]'s answer, not vanilla's tree. Its found path also
    /// leaves EAX's upper half holding the show-info pointer's high bits (no `movzx`), so callers
    /// comparing it against the port's clean return must mask to 16 bits.
    pub(super) fn call_real_get_script_id(this: *const u32, id: u16) -> u32 {
        unsafe { GET_SCRIPT_ID_DETOUR.call(this, id) }
    }

    /// The real vanilla `enterNewMonth` walk through the detour's trampoline - the battery's
    /// `ZTSHOWMGR_ENTER_NEW_MONTH` real-side pole: it still traverses the dual-written standalone
    /// vanilla tree, so its visits prove the vanilla walk sees exactly the registrations the store
    /// does (plus any made through the raw trampoline that the store deliberately never mirrored).
    pub(super) fn call_real_enter_new_month(this: *const u32) {
        unsafe { ENTER_NEW_MONTH_DETOUR.call(this) }
    }

    /// Same mechanism for the real `update` walk - the `ZTSHOWMGR_UPDATE` real-side pole. Its
    /// per-show virtual dispatch is vanilla's own, so routing a sentinel vtable slot through it
    /// pins the dispatch convention (slot `+0x20`, `this` = the show pointer) the port reproduces.
    pub(super) fn call_real_update(this: *const u32) {
        unsafe { UPDATE_DETOUR.call(this) }
    }

    /// The real vanilla `save` through the detour's trampoline - the `ZTSHOWMGR_SAVE_LOAD`
    /// real-side pole. Its delegation still reaches `ZTShowScriptMgr::save` through a direct
    /// `CALL`, i.e. the same Rust script-store save the port's `.hooked()` call reaches, so what
    /// the pole isolates is vanilla's own tail: reading [`SHOW_ID_COUNTER_RVA`] in place and the
    /// exact write shape (address, size 2, count 1) it hands `WriteBytesToFile` - the capture
    /// must come out byte-identical to the port's.
    pub(super) fn call_real_save(this: *const u32, file: *const i8) -> u32 {
        unsafe { SAVE_DETOUR.call(this, file) }
    }

    /// Same mechanism for the real `load` - the `ZTSHOWMGR_SAVE_LOAD` real-side pole, pinning
    /// vanilla's unsigned `version > 0x60` gate and the 2-byte read target against the port's.
    pub(super) fn call_real_load(this: *const u32, file: *const u32, version: u32) -> u32 {
        unsafe { LOAD_DETOUR.call(this, file, version) }
    }
}

/// Enables the stage-2 through stage-6 detours (`INIT_SHOW_PARAMS`, the registered-shows
/// shadow/mirror pair, the `getShowInfo`/`getScriptID` read cutover, the
/// `enterNewMonth`/`update` walk ports, and the `save`/`load` pair). Called from `lib.rs`'s
/// `experimental` block and from `reimplementation_tests::init` (the test harness never runs
/// `openztlib::init()`).
pub fn init() {
    if let Err(e) = unsafe { detours::init_detours() } {
        error!("Failed to initialise ztshowmgr detours: {e:?}");
    }
}

/// Process-global store backing the one real `ZTShowMgr` singleton, keyed by nothing (one real
/// instance, same shape as `ztawardmgr.rs`'s store). Style 2's end state is for this to be the *only*
/// live copy of the registered-shows map. Stage 3 landed it as a live **mirror** of the real tree the
/// call-through vanilla `registerShow`/`unregisterShow` bodies keep maintaining at the live global's
/// `+0x28`; stage 4's read cutover then flipped `getShowInfo`/`getScriptID` onto this map, making it
/// the source of truth every reader - Rust and un-decompiled vanilla alike - answers from. Vanilla's
/// own tree stays dual-written by the trampoline call-throughs until the plan's open item (dropping
/// the writers' call-through, moving the `DAT_0063e480` id counter into Rust with stage 6's
/// save/load) resolves, at which point it goes safely-inert dead weight.
///
/// Values are real, vanilla-owned `ZTShowInfo*` addresses - **never owned, never freed, never
/// dereferenced-freed** by this module (`ZTShowInfo` objects are constructed/owned by their real
/// callers, e.g. `ZTHabitat::setIsShowExhibit`'s own `new(0xb0)`; same un-owned-pointer rule as
/// everything `ztshow.rs` already does).
#[derive(Debug, Default)]
struct ZTShowMgrState {
    registered_shows: BTreeMap<u16, u32>,
}

static SHOW_STORE: LazyLock<Mutex<ZTShowMgrState>> = LazyLock::new(|| Mutex::new(ZTShowMgrState::default()));

/// Number of shows currently registered in the Rust store. Test-support: `ZTSHOWMGR_STANDALONE_
/// ROUNDTRIP` asserts it stays empty across standalone construction, and the stage-3 mirror test
/// (`ZTSHOWMGR_REGISTER_UNREGISTER_SHOW`) asserts its own ops drain it back to zero.
#[cfg(feature = "reimplementation-tests")]
pub(crate) fn registered_show_count() -> usize {
    SHOW_STORE.lock().unwrap().registered_shows.len()
}

/// The store's answer for one show id - the store side of the battery's cross-store diff oracles
/// (`ZTSHOWMGR_REGISTER_UNREGISTER_SHOW` and `ZTSHOWMGR_GET_SHOW_INFO_GET_SCRIPT_ID`). Since stage
/// 4's read cutover this is also the live implementation: [`ZTShowMgr::get_show_info`] and the
/// `GET_SHOW_INFO` detour answer every in-game caller from the same map this reads.
#[cfg(feature = "reimplementation-tests")]
pub(crate) fn registered_show_for_id(id: u16) -> Option<u32> {
    SHOW_STORE.lock().unwrap().registered_shows.get(&id).copied()
}

#[cfg(feature = "reimplementation-tests")]
pub(crate) mod live_support {
    use super::*;
    use openzt_detour::generated::standalone::OPERATOR_NEW;

    /// Allocates a standalone, zero-initialized `0x44`-byte `ZTShowMgr` via the real vanilla allocator
    /// (`standalone::OPERATOR_NEW`) for the live battery's `ZTSHOWMGR_STANDALONE_ROUNDTRIP` test.
    ///
    /// Deliberately never freed: instances built through the *real* constructor (the test runs both
    /// constructors side by side) own `DAT_00638008`-freelist nodes at `+0x28`/`+0x38` that have no safe
    /// Rust-side return path (the matching free helpers have no `generated.rs` entries), so per
    /// CLAUDE.md's leak-only-teardown precedent (`ztthoughtmgr.rs`'s
    /// `destroy_standalone_mgr_leaking_nodes`) both buffers are just left allocated for the rest of the
    /// one-shot test process's lifetime.
    pub(crate) fn allocate_uninitialized() -> *mut ZTShowMgr {
        unsafe { OPERATOR_NEW.original()(0x44) as *mut ZTShowMgr }
    }

    /// Calls the real vanilla `ZTShowMgr::initShowParams` through the detour's trampoline, for the
    /// live battery's `ZTSHOWMGR_INIT_SHOW_PARAMS` real-side pole - see
    /// [`detours::call_real`]'s doc comment for why `INIT_SHOW_PARAMS.original()` can't be used for
    /// this now that stage 2 hooks the address (a release build's raw-cast `.original()` would
    /// silently re-enter the Rust detour and degenerate the test into Rust-vs-Rust).
    pub(crate) fn call_real_init_show_params(this: *const u32) -> u32 {
        detours::call_real(this)
    }

    /// Calls the real vanilla `ZTShowMgr::getShowInfo` through the detour's trampoline - the
    /// battery's genuine-vanilla-tree pole (`ZTSHOWMGR_REGISTER_UNREGISTER_SHOW`'s diff oracle and
    /// `ZTSHOWMGR_GET_SHOW_INFO_GET_SCRIPT_ID`'s real side) - see
    /// [`detours::call_real_get_show_info`]; `.original()` has been a release-profile re-entry
    /// hazard on this address since stage 4 hooked it.
    pub(crate) fn call_real_get_show_info(this: *const u32, id: u16) -> u32 {
        detours::call_real_get_show_info(this, id)
    }

    /// Calls the real vanilla `ZTShowMgr::getScriptID` through the detour's trampoline - see
    /// [`detours::call_real_get_script_id`] for why this pole is only half-real post-cutover.
    pub(crate) fn call_real_get_script_id(this: *const u32, id: u16) -> u32 {
        detours::call_real_get_script_id(this, id)
    }

    /// Calls the real vanilla `ZTShowMgr::enterNewMonth` through the detour's trampoline - the
    /// `ZTSHOWMGR_ENTER_NEW_MONTH` real-side pole; `.original()` has been a release-profile
    /// re-entry hazard on this address since stage 5 hooked it.
    pub(crate) fn call_real_enter_new_month(this: *const u32) {
        detours::call_real_enter_new_month(this)
    }

    /// Calls the real vanilla `ZTShowMgr::update` through the detour's trampoline - the
    /// `ZTSHOWMGR_UPDATE` real-side pole, same `.original()` hazard as
    /// [`call_real_enter_new_month`].
    pub(crate) fn call_real_update(this: *const u32) {
        detours::call_real_update(this)
    }

    /// Runtime address of the vanilla show-id counter global ([`SHOW_ID_COUNTER_RVA`]) - lets
    /// `ZTSHOWMGR_SAVE_LOAD` seed/clobber/restore the counter without its own RVA redeclaration.
    pub(crate) fn show_id_counter_addr() -> u32 {
        get_module_base("zoo.exe") as u32 + SHOW_ID_COUNTER_RVA
    }

    /// Calls the real vanilla `ZTShowMgr::save` through the detour's trampoline - the
    /// `ZTSHOWMGR_SAVE_LOAD` real-side pole; `.original()` has been a release-profile re-entry
    /// hazard on this address since stage 6 hooked it.
    pub(crate) fn call_real_save(this: *const u32, file: *const i8) -> u32 {
        detours::call_real_save(this, file)
    }

    /// Same for the real `ZTShowMgr::load` - see [`call_real_save`].
    pub(crate) fn call_real_load(this: *const u32, file: *const u32, version: u32) -> u32 {
        detours::call_real_load(this, file, version)
    }

    /// Calls the real vanilla `ZTShowMgr::registerShow` through the detour's trampoline,
    /// **bypassing the stage-3 mirroring** - the stage-5 walk tests use it to plant a registration
    /// that exists only in the standalone vanilla tree, proving the Rust walks read [`SHOW_STORE`]
    /// and not the tree (a hooked register would dual-write and make the differential invisible).
    /// The raw body only guarantees its success byte in `AL` (upper EAX is register garbage -
    /// callers must mask `& 0xff`); the hooked path is the one returning a cleaned `0`/`1`.
    pub(crate) fn call_real_register_show(this: *const u32, show: *const u32, force: bool) -> u32 {
        detours::call_real_register_show(this, show, force)
    }

    /// The matching raw-body `unregisterShow` - drains a [`call_real_register_show`]-planted
    /// registration from the standalone vanilla tree without touching the store (a hooked
    /// unregister's mirror step would be a no-op remove for an id the store never held, but going
    /// through the raw body keeps the store-tree asymmetry explicit).
    pub(crate) fn call_real_unregister_show(this: *const u32, id: u16, show: *const u32, clear: bool) -> u32 {
        detours::call_real_unregister_show(this, id, show, clear)
    }
}
