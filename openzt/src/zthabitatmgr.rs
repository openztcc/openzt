use nt_time::{time::UtcDateTime, FileTime};
use openzt_detour_macro::detour_mod;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use tracing::{error, info};

use getset::Getters;

use crate::{
    command_console::CommandError,
    globals::globals,
    lua_fn,
    util::{get_from_memory, ref_from_memory, ZTArray, ZTBoundedString, ZTString},
    ztmapview::BFTile,
    ztworldmgr::Direction,
};

/// ZTHabitatMgr struct
#[derive(Debug)]
#[repr(C)]
pub struct ZTHabitatMgr {
    vtable: u32,                       // 0x000
    pad1: [u8; 0x04],                  // ----------------------- padding: 4 bytes
    map_size_x: u32,                   // 0x008
    map_size_y: u32,                   // 0x00c
    zoo_entrance_x: u32,               // 0x010
    zoo_entrance_y: u32,               // 0x014
    pad2: [u8; 0x04],                  // ----------------------- padding: 4 bytes
    exhibit_array: ZTArray<ZTHabitat>, // 0x01c (0xc bytes)
    other_array_start: u32,            // 0x028 //TODO: Use ZTArray; Seems to be some kind of mapping from BFTile to ZTHabitat or a ZTHabitat index
    other_array_end: u32,              // 0x02c
    other_array_buffer_end: u32,       // 0x030
    pad3: [u8; 0x24],                  // ----------------------- padding: 36 bytes
    popularity_scale_factor: f32,
}

impl ZTHabitatMgr {
    // fn get_tank(tile: &BFTile) -> Option<ZTHabitat> {

    // }

    /// The manager's own `exhibit_array` (every habitat in the loaded zoo) - exposed read-only so callers
    /// outside this module (e.g. live reimplementation-tests that need to scan real habitats for one
    /// matching some predicate, like a tank exhibit with `water_level() == 0`) can enumerate it via
    /// `ZTArray`'s own public `len`/`get`/`get_ptr`.
    pub fn exhibit_array(&self) -> &ZTArray<ZTHabitat> {
        &self.exhibit_array
    }

    pub fn get_habitat_by_tile(&self, tile: &BFTile) -> Option<ZTHabitat> {
        self.get_habitat(tile.pos.x, tile.pos.y)
    }

    /// Raw pointer variant of `get_habitat`, for callers that need the address itself (e.g.
    /// `ZTThought::populate`, which stores it verbatim as `habitat_ptr`) rather than a copy of the
    /// pointed-to `ZTHabitat`. Returns `0` (null) if no habitat occupies the tile, matching vanilla's
    /// own `ZTHabitatMgr::getHabitat` return value directly.
    pub fn get_habitat_ptr(&self, pos_x: i32, pos_y: i32) -> u32 {
        let base_ptr = self.other_array_start;
        let offset_1 = pos_x as u32 * 0xc;
        let intermediate_ptr = get_from_memory::<u32>(base_ptr + offset_1);

        let offset_2 = pos_y as u32 * 0x28;
        get_from_memory::<u32>(intermediate_ptr + offset_2)
    }

    // TODO: Should return Option<ZTExhibit> where ZTExhibit is a enum of ZTHabitat or ZTTankExhibit
    pub fn get_habitat(&self, pos_x: i32, pos_y: i32) -> Option<ZTHabitat> {
        let ptr = self.get_habitat_ptr(pos_x, pos_y);

        // TODO: Check vtable ptr and return ZTHabitat or ZTTankExhibit?

        if ptr != 0 {
            return Some(get_from_memory::<ZTHabitat>(ptr));
        }

        None
    }
}

// int index1 = temp_entity->field_0x34;
// int offset1 = index1 * 0xC;  // 12 bytes per entry
// void* basePointer = GLOBAL_ZTHabitatMgr->mbr_0x28;
// int* intermediatePtr = (int*)(basePointer + offset1);

// int index2 = temp_entity->field_0x38;
// int offset2 = index2 * 0x28;  // 40 bytes per entry
// ZTHabitat** habitatPtrPtr = (ZTHabitat**)(*intermediatePtr + offset2);
// ZTHabitat* this_01 = *habitatPtrPtr;

impl fmt::Display for ZTHabitatMgr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ZTHabitatMgr ({:#x}) {{", self.vtable)?;
        writeln!(f, "  map_size_x: {},", self.map_size_x)?;
        writeln!(f, "  map_size_y: {},", self.map_size_y)?;
        writeln!(f, "  zoo_entrance_x: {},", self.zoo_entrance_x)?;
        writeln!(f, "  zoo_entrance_y: {},", self.zoo_entrance_y)?;
        writeln!(f, "  exhibit_array length: {},", self.exhibit_array.len())?;
        writeln!(f, "  other_array_start: {:#x},", self.other_array_start)?;
        writeln!(
            f,
            "  other_array_end: {:#x} ({}),",
            self.other_array_end,
            (self.other_array_end - self.other_array_start) / 12
        )?;
        writeln!(
            f,
            "  other_array_buffer_end: {:#x} ({}),",
            self.other_array_buffer_end,
            (self.other_array_buffer_end - self.other_array_start) / 12
        )?;
        writeln!(f, "  popularity_scale_factor: {},", self.popularity_scale_factor)?;
        write!(f, "}}")
    }
}


#[derive(Debug, Getters)]
#[repr(C)]
#[get = "pub"]
pub struct ZTHabitat {
    vtable: u32,                 // 0x000
    zt_show_info_ptr: u32,       // 0x004
    pad1a: [u8; 0x24],           // ----------------------- padding: 36 bytes
    unknown_flag_0x2c: u8,       // 0x02c // Gates ZTThought::ZTThought's acceptance of a passed-in habitat pointer (see ztthoughtmgr.rs); ZTHabitat::recalculateCharacteristics also early-returns when this is set. Meaning not otherwise confirmed.
    pad1b: [u8; 0x13],           // ----------------------- padding: 19 bytes
    exhibit_tile_ptr: u32,       // 0x040 // Seems incorrect?
    pad2: [u8; 0x48],            // ----------------------- padding: 72 bytes
    entrance_tile_ptr: u32,      // 0x08c
    entrance_rotation: u32,      // 0x090
    pad3: [u8; 0x58],            // ----------------------- padding: 88 bytes
    unknown_u32: u32,            // 0x0ec
    pad4: [u8; 0xc],             // ----------------------- padding: 12 bytes
    current_donactions: f32,     // 0xfc
    last_donactions: f32,        // 0x100
    total_donactions: f32,       // 0x104
    current_upkeep: f32,         // 0x108
    last_upkeep: f32,            // 0x10c
    total_upkeep: f32,           // 0x110
    unknown_u32_2: u32,          // 0x114
    unknown_u32_3: u32,          // 0x118
    unknown_u32_4: u32,          // 0x11c
    created_timestamp: FileTime, // 0x120
    unknown_nt_time: FileTime,   // 0x128
    pad5: [u8; 0x24],            // ----------------------- padding: 36 bytes
    exhibit_name: ZTBoundedString, // 0x154
    pad6: [u8; 0x24],            // ----------------------- padding: 36 bytes
    tank_height: u32,            // 0x184 // Actual structural tank height (ZTTankExhibit::getTankHeight/setTankHeight); not the field checkTankPlacement compares against, see water_level.
    water_level: u32,            // 0x188 // Current water level (ZTTankExhibit::getWaterLevel); this is what checkTankPlacement's height comparisons actually use.
    pad7: [u8; 0xc],             // ----------------------- padding: 12 bytes
    is_filled: bool,             // 0x198 // Set true by ZTTankExhibit::fill(), false by ZTTankExhibit::drain().
}

const _: () = assert!(std::mem::size_of::<ZTHabitat>() == 0x198);

impl ZTHabitat {
    const TANK_VTABLE_PTR: u32 = 0x006312bc;
    pub fn get_gate_tile_in(&self) -> Option<BFTile> {
        if self.entrance_tile_ptr == 0 {
            return None;
        }
        // info!("ZTHabitat: {}", self);
        // info!("Entrance tile ptr: {:#x}", self.entrance_tile_ptr);
        let tile = get_from_memory::<BFTile>(self.entrance_tile_ptr);
        // info!("Entrance tile: {}", tile);

        let zthm = globals().zthabitatmgr();
        if let Some(gate_habitat) = zthm.get_habitat_by_tile(&tile)
            && gate_habitat == *self {
                return Some(tile);
            }
        let ztwm = globals().ztworldmgr();
        ztwm.get_neighbour(&tile, Direction::from(self.entrance_rotation))
    }

    pub fn is_tank(&self) -> bool {
        self.vtable == Self::TANK_VTABLE_PTR
    }

    pub fn is_show_tank(&self) -> bool {
        self.zt_show_info_ptr != 0
    }
}

impl PartialEq for ZTHabitat {
    fn eq(&self, other: &Self) -> bool {
        self.exhibit_tile_ptr == other.exhibit_tile_ptr
            && self.entrance_rotation == other.entrance_rotation
            && self.entrance_tile_ptr == other.entrance_tile_ptr
            && self.exhibit_name.copy_to_string() == other.exhibit_name.copy_to_string()
    }
}

impl fmt::Display for ZTHabitat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ZTHabitat {{",)?;
        writeln!(f, "  vtable: {:#x},", self.vtable)?;
        writeln!(f, "  zt_show_info_ptr: {:#x},", self.zt_show_info_ptr)?;
        writeln!(f, "  exhibit_tile_ptr: {:#x},", self.exhibit_tile_ptr)?;
        writeln!(f, "  entrance_tile_ptr: {:#x},", self.entrance_tile_ptr)?;
        writeln!(f, "  entrance_rotation: {:#x},", self.entrance_rotation)?;
        writeln!(f, "  unknown_u32: {:#x},", self.unknown_u32)?;
        writeln!(f, "  current_donactions: {},", self.current_donactions)?;
        writeln!(f, "  last_donactions: {},", self.last_donactions)?;
        writeln!(f, "  total_donactions: {},", self.total_donactions)?;
        writeln!(f, "  current_upkeep: {},", self.current_upkeep)?;
        writeln!(f, "  last_upkeep: {},", self.last_upkeep)?;
        writeln!(f, "  total_upkeep: {},", self.total_upkeep)?;
        writeln!(f, "  unknown_u32_2: {:#x},", self.unknown_u32_2)?;
        writeln!(f, "  unknown_u32_3: {:#x},", self.unknown_u32_3)?;
        writeln!(f, "  unknown_u32_4: {:#x},", self.unknown_u32_4)?;
        writeln!(f, "  created_timestamp: {},", UtcDateTime::try_from(self.created_timestamp).unwrap())?;
        writeln!(
            f,
            "  unknown_nt_time: {} ({}, {}, {}),",
            UtcDateTime::try_from(self.unknown_nt_time).unwrap(),
            self.unknown_nt_time.to_raw() as f64,
            self.unknown_nt_time.to_raw() as u32,
            (self.unknown_nt_time.to_raw() >> 32) as u32
        )?;
        writeln!(f, "  exhibit_name: {},", self.exhibit_name.copy_to_string())?;

        // writeln!(f, "  entrance_x: {},", self.entrance_x)?;
        // writeln!(f, "  entrance_y: {},", self.entrance_y)?;
        // writeln!(f, "  entrance_rotation: {},", self.entrance_rotation)?;
        // writeln!(f, "  unknown_ptr: {:#x},", self.unknown_ptr)?;
        // writeln!(f, "  unknown_ptr2: {:#x},", self.unknown_ptr2)?;
        // writeln!(f, "  unknown_ptr3: {:#x},", self.unknown_ptr3)?;
        // writeln!(f, "  current_donations: {},", self.current_donations)?;
        // writeln!(f, "  last_donations: {},", self.last_donations)?;
        // writeln!(f, "  total_donations: {},", self.total_donations)?;
        // writeln!(f, "  current_upkeep: {},", self.current_upkeep)?;
        // writeln!(f, "  last_upkeep: {},", self.last_upkeep)?;
        // writeln!(f, "  total_upkeep: {},", self.total_upkeep)?;
        // writeln!(f, " unknown_ptr4: {:#x},", self.unknown_ptr4)?;
        // writeln!(f, " unknown_ptr5: {:#x},", self.unknown_ptr5)?;
        // writeln!(f, " unknown_ptr6: {:#x},", self.unknown_ptr6)?;
        // writeln!(f, " created_timestamp: {:#x},", self.created_timestamp)?;
        writeln!(f, "}}")
    }
}

fn command_get_zt_habitat_mgr(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_habitat_mgr = globals().zthabitatmgr();
    Ok(format!("{}", zt_habitat_mgr))
}

fn command_get_zt_habitats(_args: Vec<&str>) -> Result<String, CommandError> {
    let zt_habitat_mgr = globals().zthabitatmgr();
    let mut result_string = String::new();
    for i in 0..zt_habitat_mgr.exhibit_array.len() {
        let habitat = zt_habitat_mgr.exhibit_array.get(i);
        let habitat_location = zt_habitat_mgr.exhibit_array.get_ptr(i);
        let popularity_scale_factor = zt_habitat_mgr.popularity_scale_factor;
        result_string.push_str(&format!("Habitat {} ({:#x}): ", i, habitat_location));
        result_string.push_str(&format!(
            "  exhibit_popularity?: {}, {}, {}),\n",
            (habitat.unknown_nt_time.to_raw() as f64) / popularity_scale_factor as f64,
            (habitat.unknown_nt_time.to_raw() as f32) / popularity_scale_factor,
            ((habitat.unknown_nt_time.to_raw() >> 32) as f32) / popularity_scale_factor
        ));
        result_string.push_str(&format!("{}\n", habitat));
    }
    // zt_habitat_mgr.exhibit_array.get_vec().iter().enumerate().for_each(|(i, habitat)| {
    //     result_string.push_str(&format!("Habitat {}: ", i));
    //     result_string.push_str(&format!("{}\n", habitat));
    // });
    Ok(result_string)
}

#[detour_mod]
pub mod hooks_zthabitatmgr {
    use super::*;
    use openzt_detour::generated::zthabitat::GET_GATE_TILE_IN;

    // 00410349 BFTile * __thiscall OOAnalyzer::ZTHabitat::getGateTileIn(ZTHabitat *this)
    #[detour(GET_GATE_TILE_IN)]
    unsafe extern "thiscall" fn get_gate_tile_in(_this: *const u32) -> *const u32 {
        let habitat = unsafe { ref_from_memory::<ZTHabitat>(_this) };
        match habitat.get_gate_tile_in() {
            Some(tile) => globals().ztworldmgr().get_ptr_from_bftile(&tile) as *const u32,
            None => std::ptr::null(),
        }
    }
}

/// Diagnosing a real save-corruption report (see
/// `openzt/plans/ztshow-save-corruption-investigation.md`): real, un-reimplemented `ZTHabitatMgr::save`/
/// `load` is the very first thing `ZTWorldMgr::save`/`load` calls, ahead of every already-instrumented
/// manager (`ZTShowMgr` etc., whose own `DIAG` lines proved a real reload desyncs somewhere upstream of
/// them). Passthrough-only (`<NAME>_DETOUR.call(...)`, never altering behavior) counting of every real
/// `WriteBytesToFile`/read call bracketed around `ZTHabitatMgr::save`/`load` gives an exact real
/// bytes-written-on-save vs. bytes-consumed-on-load count for this manager, without re-invoking it
/// out-of-band the way the live test battery's byte-count tests do for `ZTShowInfo`: unlike `ZTShowInfo`
/// (whose real `load` rewrites the same already-existing object in place), `ZTHabitatMgr::load`'s own
/// decompile (`ZTHabitatMgr_load.asm`) shows it allocating fresh `ZTShowInfo`/habitat-history objects and
/// growing a global std::vector-shaped structure (`DAT_0063917c`/`_78`/`_80`) - calling it a second time
/// against an already-loaded live manager would duplicate/corrupt that state, not just leak. So this only
/// ever observes one real save-then-reload cycle the user triggers normally in-game; it never calls
/// `SAVE`/`LOAD` itself. `grep DIAG openzt.log` after a real save+reload shows whether `ZTHabitatMgr`'s own
/// byte counts already disagree - narrowing the desync to before/inside this manager's block, or ruling it
/// out in favor of something later in `ZTWorldMgr::save`/`load`'s walk.
///
/// Gated off entirely under `reimplementation-tests`: that build's `io_redirect.rs` already detours these
/// same two low-level `WRITE_BYTES_TO_FILE`/`DEALLOCATE` addresses (for its own synthetic capture/replay
/// tests), and `experimental` (which this module's own `init()` runs under) is active in that build too -
/// detouring the same address twice in one binary would conflict with that battery.
#[cfg(not(feature = "reimplementation-tests"))]
mod save_load_diag {
    use super::*;
    use openzt_detour::generated::bfevent::LOAD as BFEVENT_LOAD;
    use openzt_detour::generated::standalone::{DEALLOCATE, WRITE_BYTES_TO_FILE};
    use openzt_detour::generated::ztshowinfo::LOAD as ZTSHOWINFO_LOAD;
    use openzt_detour::generated::zthabitatmgr::{LOAD, SAVE};

    static BYTE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    /// Per-call `(size_in_bytes, count, first_4_bytes_as_u32)` for every real `WriteBytesToFile`/read call
    /// seen while inside the current `ZTHabitatMgr::save`/`load` bracket, in call order - lets a save's
    /// sequence be diffed directly against a load's sequence (see the `save_load_diag` module doc comment)
    /// to find exactly which Nth field's size first disagrees, instead of only seeing the two totals
    /// disagree. `size_in_bytes`/`count` are kept separate (not pre-multiplied) so a `(2,2)` two-element
    /// array read can't be confused in the log with a single `(4,1)` scalar read - both have the same
    /// total but are genuinely different calls. The captured value is always the first up-to-4 bytes
    /// actually transferred, zero-padded if the field is shorter - lets a divergence be cross-referenced
    /// directly against known field values (e.g. a small count) without separately hex-dumping the file.
    // 4th element: the call's raw source/dest address - lets a field be identified by its exact byte
    // offset from a known base (e.g. the nested ZTShowInfo::load's own `this`) instead of by counting
    // call positions by hand against a decompile, which this investigation has repeatedly gotten wrong.
    static CALL_SIZES: Mutex<Vec<(u32, u32, u32, u32)>> = Mutex::new(Vec::new());

    fn peek_u32(ptr: *const u8, len: usize) -> u32 {
        let mut buf = [0u8; 4];
        let n = len.min(4);
        unsafe { std::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), n) };
        u32::from_le_bytes(buf)
    }

    #[detour_mod]
    mod detours {
        use super::*;

        #[detour(WRITE_BYTES_TO_FILE)]
        unsafe extern "cdecl" fn write_bytes_to_file(source_ptr: *const u32, size_in_bytes: u32, count: u32, file_ptr: *const i8) -> i32 {
            let total = (size_in_bytes as usize) * (count as usize);
            let value = peek_u32(source_ptr as *const u8, total);
            BYTE_COUNTER.fetch_add(total, Ordering::Relaxed);
            CALL_SIZES.lock().unwrap().push((size_in_bytes, count, value, source_ptr as u32));
            unsafe { WRITE_BYTES_TO_FILE_DETOUR.call(source_ptr, size_in_bytes, count, file_ptr) }
        }

        #[detour(DEALLOCATE)]
        unsafe extern "cdecl" fn deallocate(dest_ptr: *const u32, size_in_bytes: u32, count: u32, file_ptr: *const u8) -> u32 {
            let total = (size_in_bytes as usize) * (count as usize);
            let result = unsafe { DEALLOCATE_DETOUR.call(dest_ptr, size_in_bytes, count, file_ptr) };
            let value = peek_u32(dest_ptr as *const u8, total);
            BYTE_COUNTER.fetch_add(total, Ordering::Relaxed);
            CALL_SIZES.lock().unwrap().push((size_in_bytes, count, value, dest_ptr as u32));
            result
        }

        #[detour(SAVE)]
        unsafe extern "thiscall" fn save(this: *const u32, file: *const i8) -> u32 {
            BYTE_COUNTER.store(0, Ordering::Relaxed);
            CALL_SIZES.lock().unwrap().clear();
            error!("DIAG SAVE_ENTER ZTHabitatMgr");
            let ok = unsafe { SAVE_DETOUR.call(this, file) };
            let bytes = BYTE_COUNTER.load(Ordering::Relaxed);
            let calls = CALL_SIZES.lock().unwrap();
            error!("DIAG SAVE_RESULT ZTHabitatMgr ok={ok} bytes={bytes} calls={} sizes={:?}", calls.len(), &*calls);
            ok
        }

        #[detour(LOAD)]
        unsafe extern "thiscall" fn load(this: *const u32, file: *const u32, version: u32) -> u32 {
            BYTE_COUNTER.store(0, Ordering::Relaxed);
            CALL_SIZES.lock().unwrap().clear();
            BFEVENT_LOAD_FIRE_COUNT.store(0, Ordering::Relaxed);
            error!("DIAG LOAD_ENTER ZTHabitatMgr version={version}");
            let ok = unsafe { LOAD_DETOUR.call(this, file, version) };
            let bytes = BYTE_COUNTER.load(Ordering::Relaxed);
            let calls = CALL_SIZES.lock().unwrap();
            let bfevent_fires = BFEVENT_LOAD_FIRE_COUNT.load(Ordering::Relaxed);
            error!(
                "DIAG LOAD_RESULT ZTHabitatMgr ok={ok} bytes={bytes} calls={} bfevent_load_fires={bfevent_fires} sizes={:?}",
                calls.len(),
                &*calls
            );
            ok
        }

        /// Passive, passthrough-only trace on the *nested* `ZTShowInfo::load` call real vanilla
        /// `ZTHabitatMgr::load` makes once per habitat that has a show attached (confirmed via the macOS
        /// `ZTHabitatMgr_load.c` decompile, line ~240 - no Windows `.c` exists for this function, only
        /// `.asm`, so this is the first authoritative confirmation that this nested call exists at all).
        /// `ztshowinfo::LOAD` has no other production caller anywhere in this codebase (grepped) - every
        /// real invocation during a real save/reload cycle is one of these nested per-habitat calls, so
        /// this needs no bracket/gating logic of its own. Logs the outer `CALL_SIZES` length *at the
        /// moment of entry* so a nested call's position can be cross-referenced directly against the
        /// `LOAD_RESULT ZTHabitatMgr` call-index where the byte-level divergence documented in
        /// `ztshow-save-corruption-investigation.md` was found (call #150) - answering whether the
        /// divergence falls inside the first nested call, a later one, or before any nested call has even
        /// started.
        #[detour(ZTSHOWINFO_LOAD)]
        unsafe extern "thiscall" fn ztshowinfo_load(this: *const u32, file: *const u32, version: u32) -> u8 {
            let at_call_index = CALL_SIZES.lock().unwrap().len();
            error!("DIAG NESTED_SHOWINFO_LOAD_ENTER this={this:?} version={version} at_call_index={at_call_index}");
            let ok = unsafe { ZTSHOWINFO_LOAD_DETOUR.call(this, file, version) };
            let at_call_index_after = CALL_SIZES.lock().unwrap().len();
            // Live re-walk of the just-loaded tree, straight off the stack-local object `ZTHabitatMgr::load`
            // just populated, before the vector_pod copy or any tick processing touches it - isolates
            // whether a real, on-disk node-count regression happens during LOAD itself versus afterward.
            let live_node_count = crate::ztshow::pending_script_node_count(this as u32);
            error!(
                "DIAG NESTED_SHOWINFO_LOAD_RESULT this={this:?} ok={ok} calls_consumed={} live_node_count={live_node_count}",
                at_call_index_after - at_call_index
            );
            ok
        }

        /// Passive, passthrough-only trace on `BFEvent::load` - `ZTShowInfo::load`'s own decompile
        /// (`ZTShowInfo_load.c` lines 254-318) calls this once per element of a `local_e4`-driven loop
        /// gated on `param_2 > 0x60` (true for the real save version 106), reading a *third*, distinct
        /// reuse of the same `local_e4` local as an event count. If the byte-level read-size divergence
        /// this module's `NESTED_SHOWINFO_LOAD_ENTER`/`_RESULT` pair narrowed down to (see
        /// `ztshow-save-corruption-investigation.md`) is really this count being misread as a huge/garbage
        /// value, this loop would start firing far earlier (lower `at_call_index`) - or an implausible
        /// number of times - on the corrupted reload versus a working load. No other production caller of
        /// `bfevent::LOAD` exists in this codebase (grepped), so every firing during a save-reload cycle is
        /// one of these iterations.
        #[detour(BFEVENT_LOAD)]
        unsafe extern "thiscall" fn bfevent_load(this: *const u32, file: *const u32, version: u32) -> bool {
            let at_call_index = CALL_SIZES.lock().unwrap().len();
            let count = BFEVENT_LOAD_FIRE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            if count <= 5 || count % 5000 == 0 {
                error!("DIAG BFEVENT_LOAD_ENTER this={this:?} version={version} at_call_index={at_call_index} fire_count={count}");
            }
            unsafe { BFEVENT_LOAD_DETOUR.call(this, file, version) }
        }
    }

    static BFEVENT_LOAD_FIRE_COUNT: AtomicUsize = AtomicUsize::new(0);

    pub fn init() {
        if let Err(e) = unsafe { detours::init_detours() } {
            error!("Failed to initialise ZTHabitatMgr save/load DIAG detours: {e:?}");
        }
    }
}

pub fn init() {
    // get_zthabitatmgr() - no args
    lua_fn!("get_zthabitatmgr", "Returns ZTHabitatMgr debug info", "get_zthabitatmgr()", || {
        match command_get_zt_habitat_mgr(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string()))),
        }
    });

    // list_exhibits() - no args
    lua_fn!("list_exhibits", "Lists all zoo exhibits/habitats", "list_exhibits()", || {
        match command_get_zt_habitats(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string()))),
        }
    });

    if let Err(e) = unsafe { hooks_zthabitatmgr::init_detours() } {
        info!("Error initialising zthabitatmgr detours: {}", e);
    }

    #[cfg(not(feature = "reimplementation-tests"))]
    save_load_diag::init();
}
