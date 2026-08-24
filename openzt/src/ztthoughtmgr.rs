//! Structs and methods for the vanilla `ZTThoughtMgr`/`ZTThought` classes, which track the "thought
//! bubble" messages guests/animals display over their heads (e.g. "caught prey", template string id
//! `0x280a`) - a simple, small `BFMgr`-derived class owning a single intrusive, sentinel-terminated
//! linked list of `ZTThought` records. See `ZTThoughtMgr.md`'s implementation plan for the full
//! confirmed-layout evidence trail.
//!
//! Unlike `ZTResearchMgr`/`ZTMarketingMgr`, the persistent list and the UI's temporary output lists
//! both use vanilla's own small-object freelist allocator, whose low-level alloc/free helpers have no
//! confirmed Windows addresses. Rather than chase those, this module's list is *exclusively*
//! Rust-owned (`Box`-allocated nodes) from the point OpenZT loads onward - the only vanilla-allocated
//! survivor is the original sentinel node `CreateZTThoughtMgr` allocates at startup (left un-detoured;
//! see the plan's "Leave as-is" section), which the list helpers below must never attempt to free.
//!
//! This file covers Phase A (struct layout, the intrusive-list helper, and the globals accessor),
//! Phase B (`ZTThought::populate`/`get_string`, both pure reads), Phase C (the `getThoughtsBy*`
//! accessors, `Vec`-returning per the module doc comment above), Phase D (the `thought_ui_detours`
//! module below, detouring the three UI consumers that used to walk a vanilla-shaped result of
//! `getThoughtsBy*` as a real `std::list`), Phase E (`thought_mutator_detours`: `addThought`/
//! `removeThoughtsBy{Thinker,Object,Habitat}`), Phase F (`thought_save_detours`:
//! `save`/`load`/`populateThoughts`) and Phase G (`thought_dtor_detour`: `~ZTThoughtMgr`) of the
//! plan. Phase H (the live `reimplementation-tests` comparison battery, in
//! `reimplementation_tests::detour_zoo_main`, plus this file's own `live_support` module below) is also
//! complete and passing - it caught and fixed three real bugs no earlier phase's tests exercised live:
//! `get_thoughts_by_thinker`/`_object`/`_habitat` returning selected matches in the wrong order (the
//! real vanilla output is oldest-of-the-selected-first, not most-recent-first - see those methods' own
//! doc comments), `load` linking surviving records at the list's front instead of its back (see
//! `link_back`'s own doc comment), and `ZTWorldMgr::resolve_entity_by_id` reading its target function
//! pointer from the wrong vtable offset entirely (`0x90` instead of the correct `0x24` - see that
//! method's own doc comment in `ztworldmgr.rs`).

use std::mem;

use openzt_detour::generated::standalone::{DEALLOCATE, WRITE_BYTES_TO_FILE};

use crate::{
    globals::{get_module_base, globals},
    string_registry::load_string_by_id,
    util::{get_from_memory, ref_from_memory, ZTString},
    zthabitatmgr::ZTHabitat,
    ztworldmgr::BFEntity,
};

/// The zoo's thought manager - owns the persistent, sentinel-terminated intrusive list of every
/// active `ZTThought`. Confirmed `operator_new(0x10)` - 16 bytes total (`BFMgr` vtable + inherited
/// flag byte + the list's own two fields) - via `_CreateZTThoughtMgr.c`'s allocation and
/// `ZTThoughtMgr_addThought.asm`/every `remove*`/`get*` decompile's field offsets.
#[derive(Debug)]
#[repr(C)]
pub struct ZTThoughtMgr {
    vtable: u32,       // 0x0
    flag: u8,           // 0x4 - inherited BFMgr field, not behaviorally relevant
    _pad: [u8; 3],      // ----- padding: 3 bytes
    sentinel_ptr: u32,  // 0x8 - pointer to the list's sentinel node (not embedded inline)
    max_thoughts: u32,  // 0xc - default 1000, the cap `addThought` trims the list to
}

const _: () = assert!(mem::size_of::<ZTThoughtMgr>() == 0x10);

/// One thought bubble entry. Own 2-entry vtable (`save`/`load`), no base class. Total confirmed size
/// `0x24` (36 bytes = 9 x u32/i32 fields) - matches the temporary-list node's free size seen in
/// `_fillListBox.c` (8 link bytes + `0x24` data = `0x2c`). The *persistent* list's nodes are allocated
/// at `0x30` in `_CreateZTThoughtMgr.c` - 4 bytes more than `0x24`, assumed to be the block allocator's
/// size-class rounding rather than an undiscovered 10th field.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ZTThought {
    vtable: u32,      // 0x0
    string_id: u32,   // 0x4 - message/template id, e.g. 0x280a for "caught prey"
    thinker_id: u32,  // 0x8 - not a raw pointer; copied from thinker_entity's own BFEntity::id at construction
    object_id: u32,   // 0xc - same mechanism, from object_entity's own BFEntity::id
    tile_x: i32,      // 0x10 - -1 sentinel = "none"
    tile_y: i32,      // 0x14 - -1 sentinel = "none"
    thinker_ptr: u32, // 0x18 - resolved live pointer, populated by resolving thinker_id via ZTWorldMgr::resolve_entity_by_id; null if unresolved
    object_ptr: u32,  // 0x1c - same resolution mechanism for object_id
    habitat_ptr: u32, // 0x20 - *ZTHabitat; set directly at construction, or recomputed on populate/load
}

const _: () = assert!(mem::size_of::<ZTThought>() == 0x24);

impl ZTThought {
    pub fn string_id(&self) -> u32 {
        self.string_id
    }

    pub fn thinker_id(&self) -> u32 {
        self.thinker_id
    }

    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    pub fn tile_x(&self) -> i32 {
        self.tile_x
    }

    pub fn tile_y(&self) -> i32 {
        self.tile_y
    }

    pub fn thinker_ptr(&self) -> u32 {
        self.thinker_ptr
    }

    pub fn object_ptr(&self) -> u32 {
        self.object_ptr
    }

    pub fn habitat_ptr(&self) -> u32 {
        self.habitat_ptr
    }

    /// Reimplementation of `OOAnalyzer::ZTThought::populate`, per `ZTThought_populate.c`: called after
    /// a save-file load to re-derive the live, non-persistent pointer fields (`thinker_ptr`/
    /// `object_ptr`/`habitat_ptr`, plus a possible `tile_x`/`tile_y` refresh) from the persisted
    /// `thinker_id`/`object_id`/`tile_x`/`tile_y` alone.
    ///
    /// `thinker_id`/`object_id` are unconditionally re-resolved via `ZTWorldMgr::resolve_entity_by_id`
    /// - vanilla itself never null-guards the id before calling the resolver, so this doesn't either.
    ///
    /// If `object_ptr` resolved, [`resolve_object_own_habitat_ptr`] is consulted (see its own doc
    /// comment for what's confirmed about the vtable-driven check it performs); if it resolves, its
    /// result becomes `habitat_ptr` and, if non-null, `tile_x`/`tile_y` are refreshed from that
    /// habitat's own gate tile (this is presumably how e.g. a "caught prey" thought about a captured
    /// animal, rather than a tile position, ends up with the correct habitat).
    ///
    /// Finally, if `habitat_ptr` is still unresolved and a real tile is set, it's recomputed via
    /// `ZTHabitatMgr::getHabitat` - unconditionally assigned even if that returns null, matching
    /// vanilla's own unconditional store (a no-op here, since the guard already requires `habitat_ptr
    /// == 0`).
    pub fn populate(&mut self) {
        let world_mgr = globals().ztworldmgr();
        self.thinker_ptr = world_mgr.resolve_entity_by_id(self.thinker_id) as u32;
        self.object_ptr = world_mgr.resolve_entity_by_id(self.object_id) as u32;

        if self.object_ptr != 0 {
            if let Some(habitat_ptr) = resolve_object_own_habitat_ptr(self.object_ptr) {
                self.habitat_ptr = habitat_ptr;
                if self.habitat_ptr != 0 {
                    let habitat = unsafe { ref_from_memory::<ZTHabitat>(self.habitat_ptr) };
                    if let Some(tile) = habitat.get_gate_tile_in() {
                        self.tile_x = tile.pos.x;
                        self.tile_y = tile.pos.y;
                    }
                }
            }
        }

        if self.habitat_ptr == 0 && self.tile_x != -1 && self.tile_y != -1 {
            self.habitat_ptr = globals().zthabitatmgr().get_habitat_ptr(self.tile_x, self.tile_y);
        }
    }

    /// Reimplementation of `OOAnalyzer::ZTThought::ZTThought` (`ZTThought_ZTThought.c`). `habitat_arg`
    /// is only accepted into `habitat_ptr` if the pointed-to `ZTHabitat`'s own flag at `+0x2c`
    /// (`ZTHabitat::unknown_flag_0x2c`) is unset - matching the ctor's own
    /// `*(char*)(param_4+0x2c)=='\0'` guard - and, when accepted, `tile_x`/`tile_y` are immediately
    /// refreshed from that habitat's own gate tile (`get_gate_tile_in`), same as the ctor's own tail.
    /// `thinker_id`/`object_id` are resolved from `thinker_ptr`/`object_ptr`'s own `BFEntity::id`
    /// (`+0x124`) whenever those pointers are non-null. All-zero arguments (as `ZTThoughtMgr::load`
    /// passes before overwriting the fields itself) touch no real memory at all.
    fn new(string_id: u32, thinker_ptr: u32, object_ptr: u32, habitat_arg: u32) -> ZTThought {
        let vtable = get_module_base("zoo.exe") as u32 + 0x0023_5400;
        let mut thought =
            ZTThought { vtable, string_id, thinker_id: 0, object_id: 0, tile_x: -1, tile_y: -1, thinker_ptr, object_ptr, habitat_ptr: 0 };

        if thinker_ptr != 0 {
            thought.thinker_id = *unsafe { ref_from_memory::<BFEntity>(thinker_ptr) }.id();
        }
        if object_ptr != 0 {
            thought.object_id = *unsafe { ref_from_memory::<BFEntity>(object_ptr) }.id();
        }
        if habitat_arg != 0 {
            let habitat = unsafe { ref_from_memory::<ZTHabitat>(habitat_arg) };
            if *habitat.unknown_flag_0x2c() == 0 {
                thought.habitat_ptr = habitat_arg;
            }
            if thought.habitat_ptr != 0 {
                if let Some(tile) = unsafe { ref_from_memory::<ZTHabitat>(thought.habitat_ptr) }.get_gate_tile_in() {
                    thought.tile_x = tile.pos.x;
                    thought.tile_y = tile.pos.y;
                }
            }
        }
        thought
    }

    /// Reimplementation of `OOAnalyzer::ZTThought::save` (`ZTThought_save.c`): writes `string_id`,
    /// `thinker_id`, `object_id`, `tile_x`, `tile_y` as five little-endian dwords, in that order. Every
    /// write happens regardless of an earlier one failing - matches the decompile's own unconditional
    /// sequence of `WriteBytesToFile` calls, ANDed together only at the end - so `ok` only reflects
    /// whether *all five* succeeded.
    pub fn save(&self, file: *const u32) -> bool {
        let mut ok = write_dword(file, self.string_id);
        ok &= write_dword(file, self.thinker_id);
        ok &= write_dword(file, self.object_id);
        ok &= write_dword(file, self.tile_x as u32);
        ok &= write_dword(file, self.tile_y as u32);
        ok
    }

    /// Reimplementation of `OOAnalyzer::ZTThought::load` (`ZTThought_load.c`): reads `string_id` first,
    /// unconditionally, then branches on `version` for the remaining fields' read order - a pre-`0x1e`
    /// save only ever wrote `string_id`/`object_id`/`thinker_id` (no tile), so the legacy branch reads
    /// exactly those 3 fields in *that* order and leaves `tile_x`/`tile_y` at their ctor default of
    /// `-1`; `version >= 0x1e` reads all 5 fields in `save`'s own order (`thinker_id`, `object_id`,
    /// `tile_x`, `tile_y`). Every read is attempted regardless of an earlier one failing, matching the
    /// decompile's own unconditional read sequence.
    ///
    /// If every read succeeded and `version > 0x1d`, re-resolves `thinker_ptr`/`object_ptr` via
    /// `ZTWorldMgr::resolve_entity_by_id`, then `habitat_ptr` directly via `ZTHabitatMgr::getHabitat` if
    /// a real tile is set - a simpler, tile-only resolution than `populate`'s own object-vtable-driven
    /// version (this is *not* a call to `populate`; that's `ZTThoughtMgr::populate_thoughts`'s own job,
    /// for older-format streams that never reach this branch).
    pub fn load(&mut self, file: *const u32, version: u32) -> bool {
        let string_id_ok = match read_dword(file) {
            Some(v) => {
                self.string_id = v;
                true
            }
            None => false,
        };

        let ok = if version < 0x1e {
            let object_id_ok = match read_dword(file) {
                Some(v) => {
                    self.object_id = v;
                    true
                }
                None => false,
            };
            let thinker_id_ok = match read_dword(file) {
                Some(v) => {
                    self.thinker_id = v;
                    true
                }
                None => false,
            };
            string_id_ok && object_id_ok && thinker_id_ok
        } else {
            let thinker_id_ok = match read_dword(file) {
                Some(v) => {
                    self.thinker_id = v;
                    true
                }
                None => false,
            };
            let object_id_ok = match read_dword(file) {
                Some(v) => {
                    self.object_id = v;
                    true
                }
                None => false,
            };
            let tile_x_ok = match read_dword(file) {
                Some(v) => {
                    self.tile_x = v as i32;
                    true
                }
                None => false,
            };
            let tile_y_ok = match read_dword(file) {
                Some(v) => {
                    self.tile_y = v as i32;
                    true
                }
                None => false,
            };
            string_id_ok && thinker_id_ok && object_id_ok && tile_x_ok && tile_y_ok
        };

        if ok && version > 0x1d {
            let world_mgr = globals().ztworldmgr();
            self.thinker_ptr = world_mgr.resolve_entity_by_id(self.thinker_id) as u32;
            self.object_ptr = world_mgr.resolve_entity_by_id(self.object_id) as u32;
            if self.tile_x != -1 && self.tile_y != -1 {
                self.habitat_ptr = globals().zthabitatmgr().get_habitat_ptr(self.tile_x, self.tile_y);
            }
        }
        ok
    }

    /// Reimplementation of `OOAnalyzer::ZTThought::getString`, per `ZTThought_getString.c`: loads the
    /// thought's own template string, then applies at most one `%s` substitution - the resolved
    /// object's name if `object_ptr` is set, else the resolved habitat's exhibit name if `habitat_ptr`
    /// is set (object always takes priority over habitat, matching the decompile's own branch order),
    /// else no substitution at all (the raw template is returned unmodified). See
    /// `substitute_thought_string` for the pure substitution logic this delegates to.
    pub fn get_string(&self) -> String {
        let template = load_string_by_id(self.string_id);
        let substitution: Option<String> = if self.object_ptr != 0 {
            Some(unsafe { ref_from_memory::<BFEntity>(self.object_ptr) }.name().copy_to_string())
        } else if self.habitat_ptr != 0 {
            Some(unsafe { ref_from_memory::<ZTHabitat>(self.habitat_ptr) }.exhibit_name().copy_to_string())
        } else {
            None
        };
        substitute_thought_string(template, substitution.as_deref())
    }
}

/// Pure substitution logic `ZTThought::get_string` delegates to, isolated for testing without touching
/// real memory. A missing template (`BFApp::loadString` failing) returns an empty string; an empty
/// template (loaded successfully but blank) is returned as-is, matching `ZTThought_getString.c`'s own
/// `0 < length` guard around the entire substitution branch. Otherwise, `%s` is replaced with
/// `substitution` if one was resolved, or the template is returned unmodified if not.
fn substitute_thought_string(template: Option<String>, substitution: Option<&str>) -> String {
    let Some(template) = template else {
        return String::new();
    };
    if template.is_empty() {
        return template;
    }
    match substitution {
        Some(name) => template.replacen("%s", name, 1),
        None => template,
    }
}

/// Shared logic behind `ZTThought::populate`'s object-vtable-driven habitat resolution and
/// `ZTThoughtMgr::addThought`'s override of the caller-supplied habitat argument (see each caller's own
/// doc comment for how the result is used) - ported once here since both are byte-for-byte identical in
/// the decompile/asm. `object_ptr` must be non-null.
///
/// Calls the object's own *entity type*'s vtable slot `0x1c` with a fixed data-segment argument
/// (`DAT_00638690` in the decompile, `type_check_arg` here); if that returns `true`, calls the object
/// itself's vtable slot `0x24c` (no arguments) and returns that result as `Some` - even if it's null,
/// matching vanilla's own unconditional assignment. Returns `None` if the type check fails, meaning "no
/// override" - the caller should fall back to whatever habitat it already had. Neither vtable slot's
/// exact semantics nor `DAT_00638690`'s own meaning were independently confirmed (no symbol name for
/// either) - only that vanilla treats a `true` result as "this object type resolves its own habitat
/// directly" (ported mechanically, same technique as `BFEntity::vtable_get_footprint`).
fn resolve_object_own_habitat_ptr(object_ptr: u32) -> Option<u32> {
    let object = unsafe { ref_from_memory::<BFEntity>(object_ptr) };
    let entity_type_ptr = *object.inner_class_ptr();
    let entity_type_vtable = get_from_memory::<u32>(entity_type_ptr);
    let type_check_fn =
        unsafe { mem::transmute::<u32, extern "thiscall" fn(u32, u32) -> bool>(get_from_memory::<u32>(entity_type_vtable + 0x1c)) };
    let type_check_arg = get_module_base("zoo.exe") as u32 + 0x00238690;
    if !type_check_fn(entity_type_ptr, type_check_arg) {
        return None;
    }
    let object_vtable = *object.vtable();
    let resolve_habitat_fn = unsafe { mem::transmute::<u32, extern "thiscall" fn(u32) -> u32>(get_from_memory::<u32>(object_vtable + 0x24c)) };
    Some(resolve_habitat_fn(object_ptr))
}

/// Writes `value` as a single little-endian dword via the real vanilla `WriteBytesToFile`, shared by
/// `ZTThought::save`/`ZTThoughtMgr::save`.
fn write_dword(file: *const u32, value: u32) -> bool {
    let bytes = value.to_le_bytes();
    unsafe { WRITE_BYTES_TO_FILE.original()(bytes.as_ptr() as *const u32, 4, 1, file as *const i8) }
}

/// Reads a single little-endian dword via the real vanilla read primitive (`DEALLOCATE` in
/// `generated.rs` - a decompiler-artifact name for what is actually `fread`-shaped; see
/// `ztresearch::research_save_reimplementation::stream_io`'s own doc comment), shared by
/// `ZTThought::load`/`ZTThoughtMgr::load`. `None` on a short/failed read.
fn read_dword(file: *const u32) -> Option<u32> {
    let mut buf = 0u32;
    let ok = unsafe { DEALLOCATE.original()(&mut buf as *mut u32 as *const u32, 4, 1, file as *const u8) };
    (ok == 1).then_some(buf)
}

/// A persistent-list node: 8 bytes of intrusive links followed by the `ZTThought` payload at `+0x8`,
/// matching every `ZTThoughtMgr` list-walking method's exact shape (see e.g.
/// `ZTThoughtMgr_removeThoughtsByThinker.c`). The sentinel node vanilla allocates in
/// `_CreateZTThoughtMgr.c` shares this same link layout (its `data` is never read).
#[repr(C)]
struct ThoughtNode {
    next: *mut ThoughtNode,
    prev: *mut ThoughtNode,
    data: ZTThought,
}

/// Borrowing iterator over `ZTThoughtMgr`'s persistent list, front (most-recently-inserted) to back,
/// skipping the sentinel. Returned as `impl Iterator` from `ZTThoughtMgr::iter` so this type itself
/// never needs to be public.
struct ThoughtIter<'a> {
    sentinel: *const ThoughtNode,
    current: *const ThoughtNode,
    _marker: std::marker::PhantomData<&'a ZTThoughtMgr>,
}

impl<'a> Iterator for ThoughtIter<'a> {
    type Item = &'a ZTThought;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.sentinel {
            return None;
        }
        let node = unsafe { &*self.current };
        self.current = node.next;
        Some(&node.data)
    }
}

/// Mutable counterpart to [`ThoughtIter`], used by `ZTThoughtMgr::populate_thoughts`.
struct ThoughtIterMut<'a> {
    sentinel: *mut ThoughtNode,
    current: *mut ThoughtNode,
    _marker: std::marker::PhantomData<&'a mut ZTThoughtMgr>,
}

impl<'a> Iterator for ThoughtIterMut<'a> {
    type Item = &'a mut ZTThought;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.sentinel {
            return None;
        }
        let node = unsafe { &mut *self.current };
        self.current = node.next;
        Some(&mut node.data)
    }
}

impl ZTThoughtMgr {
    pub fn max_thoughts(&self) -> u32 {
        self.max_thoughts
    }

    fn sentinel(&self) -> *mut ThoughtNode {
        self.sentinel_ptr as *mut ThoughtNode
    }

    /// Walks the persistent thought list front-to-back (most-recently-inserted first), skipping the
    /// sentinel node - the Rust-owned replacement for ever constructing a vanilla-shaped
    /// `std::list`-style walk of this list.
    pub fn iter(&self) -> impl Iterator<Item = &ZTThought> {
        let sentinel = self.sentinel() as *const ThoughtNode;
        let current = unsafe { (*sentinel).next as *const ThoughtNode };
        ThoughtIter { sentinel, current, _marker: std::marker::PhantomData }
    }

    /// Mutable counterpart to `iter` - used by `populate_thoughts`.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ZTThought> {
        let sentinel = self.sentinel();
        let current = unsafe { (*sentinel).next };
        ThoughtIterMut { sentinel, current, _marker: std::marker::PhantomData }
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Splices `thought` in as a new `Box`-owned node at the front of the list, uncapped - the raw
    /// primitive `insert_front` (`addThought`'s own insertion behavior) builds on.
    fn link_front(&mut self, thought: ZTThought) {
        let sentinel = self.sentinel();
        let old_front = unsafe { (*sentinel).next };
        let node = Box::into_raw(Box::new(ThoughtNode { next: old_front, prev: sentinel, data: thought }));
        unsafe {
            (*old_front).prev = node;
            (*sentinel).next = node;
        }
    }

    /// Splices `thought` in as a new `Box`-owned node at the *back* of the list, uncapped - what
    /// `ZTThoughtMgr::load` actually builds on (confirmed live: `ZTTHOUGHTMGR_LOAD` in
    /// `reimplementation_tests` failed under `link_front` with reversed order; confirmed independently
    /// via the decompile - `FUN_004230a6(&local_28, this->mbr_0x8, local_24)` passes the sentinel's own
    /// address directly as the insert-before position, unlike `addThought`'s
    /// `FUN_004230a6(&param_3, *(int*)*pdVar1, puVar10)`, which dereferences one level further to pass
    /// `sentinel->next`. "Insert before the sentinel" is "insert at the back", not "insert before the
    /// front" - the opposite end from `link_front`).
    fn link_back(&mut self, thought: ZTThought) {
        let sentinel = self.sentinel();
        let old_back = unsafe { (*sentinel).prev };
        let node = Box::into_raw(Box::new(ThoughtNode { next: sentinel, prev: old_back, data: thought }));
        unsafe {
            (*old_back).next = node;
            (*sentinel).prev = node;
        }
    }

    /// Inserts `thought` as a new `Box`-owned node at the front of the list (matching `addThought`'s
    /// own insertion point - most-recent-first), then trims from the back until the list is at most
    /// `max_thoughts` long, freeing every trimmed node.
    pub(crate) fn insert_front(&mut self, thought: ZTThought) {
        self.link_front(thought);
        self.trim_to_cap();
    }

    fn trim_to_cap(&mut self) {
        while self.len() > self.max_thoughts as usize {
            let sentinel = self.sentinel();
            let last = unsafe { (*sentinel).prev };
            if last == sentinel {
                break;
            }
            self.unlink_and_free(last);
        }
    }

    /// Unlinks `node` from the list and frees it as a `Box`. `node` must be a real (non-sentinel) node
    /// currently linked into this list.
    fn unlink_and_free(&mut self, node: *mut ThoughtNode) {
        unsafe {
            let prev = (*node).prev;
            let next = (*node).next;
            (*prev).next = next;
            (*next).prev = prev;
            drop(Box::from_raw(node));
        }
    }

    /// Removes and frees every node whose `ZTThought` matches `predicate`, never touching the
    /// sentinel. The shared removal primitive `removeThoughtsBy{Thinker,Habitat,Object}` (Phase E)
    /// will each call with their own id-matching predicate.
    pub(crate) fn remove_where(&mut self, predicate: impl Fn(&ZTThought) -> bool) {
        let sentinel = self.sentinel();
        let mut current = unsafe { (*sentinel).next };
        while current != sentinel {
            let next = unsafe { (*current).next };
            if predicate(unsafe { &(*current).data }) {
                self.unlink_and_free(current);
            }
            current = next;
        }
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::getThoughtsByThinker`
    /// (`ZTThoughtMgr_getThoughtsByThinker.c`), minus the vanilla temporary-`std::list` construction
    /// the decompile builds and then immediately frees again - a `Vec` collected directly from
    /// `iter()` is a drop-in behavioral replacement for what every caller actually consumes: an
    /// ordered, `max_count`-bounded sequence of matching `ZTThought`s.
    ///
    /// Selection walks `iter()`'s own front-to-back (most-recently-added-first) order, matching the
    /// decompile's own walk and its identical `max_count`-then-stop cap - but the *final returned
    /// order* is oldest-of-the-selected-first, the reverse of the walk/selection order: confirmed live
    /// (`ZTTHOUGHTMGR_GET_THOUGHTS_BY_THINKER` in `reimplementation_tests`) and independently via the
    /// decompile - every match found during the walk is spliced onto the *front* of a scratch list
    /// (`FUN_004230a6(&param_2, *local_4.mbr_0x0, puVar1+2)`, where `*local_4.mbr_0x0` is that scratch
    /// list's own current front), so the most-recently-*walked* (= most-recently-added) match ends up
    /// at the scratch list's own back, and the whole scratch list (unreversed) is then spliced directly
    /// into the output (`FUN_004eda55`). Hence the explicit `.reverse()` below - `filter().take()` alone
    /// reproduces the right *selection*, not the right *order*.
    ///
    /// Matches on `thinker_ptr` (the *resolved live pointer*, confirmed against the decompile's node
    /// field offset `puVar1[8]` = data offset `0x18`), not `thinker_id` - vanilla itself never
    /// compares against the persisted id here.
    pub fn get_thoughts_by_thinker(&self, thinker_ptr: u32, max_count: usize) -> Vec<&ZTThought> {
        let mut matches: Vec<&ZTThought> = self.iter().filter(|t| t.thinker_ptr() == thinker_ptr).take(max_count).collect();
        matches.reverse();
        matches
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::getThoughtsByObject`
    /// (`ZTThoughtMgr_getThoughtsByObject.c`) - see `get_thoughts_by_thinker`'s doc comment for the
    /// shared `Vec`-vs-vanilla-list reasoning, including why the result is reversed after
    /// selection. Matches on `object_ptr` (node field offset `puVar1[9]` = data offset `0x1c`).
    pub fn get_thoughts_by_object(&self, object_ptr: u32, max_count: usize) -> Vec<&ZTThought> {
        let mut matches: Vec<&ZTThought> = self.iter().filter(|t| t.object_ptr() == object_ptr).take(max_count).collect();
        matches.reverse();
        matches
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::getThoughtsByHabitat`
    /// (`ZTThoughtMgr_getThoughtsByHabitat.c`) - see `get_thoughts_by_thinker`'s doc comment for the
    /// shared `Vec`-vs-vanilla-list reasoning, including why the result is reversed after
    /// selection. Matches on `habitat_ptr` (node field offset `puVar1[10]` = data offset `0x20`).
    pub fn get_thoughts_by_habitat(&self, habitat_ptr: u32, max_count: usize) -> Vec<&ZTThought> {
        let mut matches: Vec<&ZTThought> = self.iter().filter(|t| t.habitat_ptr() == habitat_ptr).take(max_count).collect();
        matches.reverse();
        matches
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::addThought` (`ZTThoughtMgr_addThought.c`/`.asm`).
    /// Uses `insert_front`, which already trims to `max_thoughts` after every insert - the exact same
    /// cap-check the decompile performs inline after splicing in the new node.
    ///
    /// If `object_ptr` is non-null, [`resolve_object_own_habitat_ptr`] is consulted first: when it
    /// resolves (the object's entity type passes its own vtable-driven check), *its* result overrides
    /// whatever `habitat_ptr` the caller passed - even if that resolves to null - exactly mirroring
    /// `addThought.asm`'s override of the constructor's 4th argument. Otherwise `habitat_ptr` is passed
    /// to `ZTThought::new` unchanged.
    pub fn add_thought(&mut self, string_id: u32, thinker_ptr: u32, object_ptr: u32, habitat_ptr: u32) {
        let habitat_arg = if object_ptr != 0 { resolve_object_own_habitat_ptr(object_ptr).unwrap_or(habitat_ptr) } else { habitat_ptr };
        self.insert_front(ZTThought::new(string_id, thinker_ptr, object_ptr, habitat_arg));
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::removeThoughtsByThinker`
    /// (`ZTThoughtMgr_removeThoughtsByThinker.c`). Matches on `thinker_ptr` (node data offset `0x18`),
    /// same field `get_thoughts_by_thinker` matches on.
    pub fn remove_thoughts_by_thinker(&mut self, thinker_ptr: u32) {
        self.remove_where(|t| t.thinker_ptr() == thinker_ptr);
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::removeThoughtsByObject`
    /// (`ZTThoughtMgr_removeThoughtsByObject.c`). Matches on `object_ptr` (node data offset `0x1c`),
    /// same field `get_thoughts_by_object` matches on.
    pub fn remove_thoughts_by_object(&mut self, object_ptr: u32) {
        self.remove_where(|t| t.object_ptr() == object_ptr);
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::removeThoughtsByHabitat`
    /// (`ZTThoughtMgr_removeThoughtsByHabitat.c`/`.asm`; the `.c` decompile's own two condition-variable
    /// names are a Ghidra stack-splitting artifact - cross-checked against `.asm`'s literal `[ESP+n]`
    /// offsets to confirm the real field pairing, since the `.c` file's own naming isn't independently
    /// meaningful). Unlike `remove_where`-based removal, this doesn't just remove matches: for every
    /// thought whose `habitat_ptr` matches, if `force` is `false` *and* the thought still has a live
    /// `object_ptr`, only the habitat link is cleared (`habitat_ptr = 0`) and the thought itself
    /// survives; otherwise (a forced removal - e.g. `ZTHabitatMgr::removeAllHabitats` passes `true` -
    /// or the thought has no object of its own to keep it alive) the node is fully unlinked and freed.
    pub fn remove_thoughts_by_habitat(&mut self, habitat_ptr: u32, force: bool) {
        let sentinel = self.sentinel();
        let mut current = unsafe { (*sentinel).next };
        while current != sentinel {
            let next = unsafe { (*current).next };
            let node = unsafe { &mut *current };
            if node.data.habitat_ptr == habitat_ptr {
                if !force && node.data.object_ptr != 0 {
                    node.data.habitat_ptr = 0;
                } else {
                    self.unlink_and_free(current);
                }
            }
            current = next;
        }
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::save` (`ZTThoughtMgr_save.c`): writes the list's
    /// own length as a leading dword, then calls `ZTThought::save` on every thought, front to back -
    /// the same order `iter()` walks the list in, matching the decompile's own walk. Every item is
    /// visited regardless of an earlier write failing; `ok` only reflects whether *everything*
    /// (the count, and every thought) wrote successfully.
    pub fn save(&self, file: *const u32) -> bool {
        let mut ok = write_dword(file, self.len() as u32);
        for thought in self.iter() {
            ok &= thought.save(file);
        }
        ok
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::load` (`ZTThoughtMgr_load.c`/`.asm`): reads a
    /// leading dword count (interpreted as signed - a failed or negative/zero count leaves the list
    /// untouched and returns immediately, matching the decompile's own `0 < local_30` signed guard).
    /// For each of `count` records, default-constructs a fresh `ZTThought` (`ZTThought::new(0,0,0,0)`,
    /// matching the decompile's own per-iteration `ZTThought::ZTThought(local_24,0,0,0,0)`) and calls
    /// `ZTThought::load` on it. A record is only spliced into the list (via `link_back` - `load`
    /// itself never trims to `max_thoughts`, unlike `addThought`) if the read succeeded *and* every
    /// non-zero id it carries actually resolved to a live pointer (`object_id == 0 || object_ptr != 0`,
    /// same for `thinker_id`/`thinker_ptr`) - for `version >= 0x1e` streams, where `ZTThought::load`
    /// already attempted that resolution inline; a record whose reference no longer resolves is
    /// silently dropped. Surviving records end up in read order, oldest-record-first at the front -
    /// see `link_back`'s own doc comment for the confirmed-live/decompile evidence that `load`, unlike
    /// `addThought`, appends at the back rather than inserting at the front.
    /// Returns the AND of every item's own `load` result (the count read's own success is only a
    /// precondition to entering the loop at all, not folded into the final result - matching the
    /// decompile exactly).
    pub fn load(&mut self, file: *const u32, version: u32) -> bool {
        let Some(count) = read_dword(file) else { return false };
        let count = count as i32;
        if count <= 0 {
            return true;
        }

        let mut ok = true;
        for _ in 0..count {
            let mut thought = ZTThought::new(0, 0, 0, 0);
            let loaded_ok = thought.load(file, version);
            ok &= loaded_ok;
            if loaded_ok && (thought.object_id == 0 || thought.object_ptr != 0) && (thought.thinker_id == 0 || thought.thinker_ptr != 0) {
                self.link_back(thought);
            }
        }
        ok
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::populateThoughts`
    /// (`ZTThoughtMgr_populateThoughts.c`): calls `ZTThought::populate` on every thought in the list,
    /// front to back. Called by vanilla's own `ZTWorldMgr::load` for a specific pre-`0x1e` save-version
    /// range - not part of `ZTThoughtMgr::load` itself, which only performs this resolution inline for
    /// `version >= 0x1e` streams (see `ZTThought::load`'s own doc comment).
    pub fn populate_thoughts(&mut self) {
        for thought in self.iter_mut() {
            thought.populate();
        }
    }

    /// Reimplementation of `OOAnalyzer::ZTThoughtMgr::~ZTThoughtMgr`'s real body - confirmed via the
    /// macOS decompile (`ZTThoughtMgr_~ZTThoughtMgr.c`), since the Windows decompile only shows the
    /// outer scalar-deleting-destructor wrapper (see [`thought_dtor_detour`]'s own doc comment) calling
    /// this as an un-decompiled inner function: vanilla's version calls a `std::list` "erase whole
    /// range" helper over the list, i.e. destroys and frees every real node - `remove_where(|_| true)`
    /// is exactly that, reusing the same `Box`-freeing primitive every other mutator does. Matches the
    /// module doc comment's own claim that the sentinel node and the `ZTThoughtMgr` struct itself are
    /// never freed here (vanilla's own version leaves both alone too - the sentinel/`end` node is never
    /// part of an `erase(begin(), end())` range, and freeing `this` is the *caller's* job, gated by the
    /// deleting-destructor wrapper's own flag byte).
    pub fn clear(&mut self) {
        self.remove_where(|_| true);
    }
}

/// Registers this module's live detours: Phase D's three UI-consumer detours, Phase E's four mutator
/// detours (`addThought`/`removeThoughtsBy{Thinker,Object,Habitat}`), Phase F's three save/load-family
/// detours (`save`/`load`/`populateThoughts`), and Phase G's destructor detour.
pub fn init() {
    thought_ui_detours::init();
    thought_mutator_detours::init();
    thought_save_detours::init();
    thought_dtor_detour::init();
}

/// Phase D: detours the three UI functions that used to be `getThoughtsBy*`'s *only* consumers of the
/// vanilla `std::list` those built - `_fillListBox` (both instantiations) and `_refillThoughtsList`.
/// Rewritten to call Phase C's `Vec`-returning accessors directly and drive
/// `BFUIMgr::getElement`/`UIListBox::clear`/`addString`/`restoreState` (all real vanilla functions,
/// called via `.original()` - never detoured themselves) in a loop instead. This is what makes Phase
/// C's `Vec` return type viable at all: once these three are detoured, nothing vanilla-side ever
/// constructs or walks a `getThoughtsBy*` result as a real intrusive list again, closing the loop
/// the module doc comment opened (no vanilla freelist allocator involvement anywhere in this file).
mod thought_ui_detours {
    use openzt_detour::generated::{
        bfuimgr::GET_ELEMENT_0,
        standalone::{FILL_LIST_BOX_0, FILL_LIST_BOX_1, REFILL_THOUGHTS_LIST},
        uilistbox::{ADD_STRING_0, CLEAR, RESTORE_STATE},
    };
    use openzt_detour_macro::detour_mod;
    use tracing::error;

    use super::*;
    use crate::{
        encoding_utils::encode_to_ansi,
        util::{save_to_memory, ZTBufferString},
    };

    /// `GLOBAL_BFUIMgr`'s own fixed address, `0x00638de0` - see `ztresearch::global_bfuimgr`'s doc
    /// comment for the full confirmation trail (including why `0x00635c54`, used here until a live
    /// crash traced it down, is wrong - that's `BFUIMgr`'s read-only vtable address, not the object);
    /// duplicated here rather than shared since it's the only other current caller and there's no
    /// existing shared home for it.
    fn global_bfuimgr() -> *const u32 {
        (get_module_base("zoo.exe") as u32 + 0x0023_8de0) as *const u32
    }

    /// The `BFUIMgr::getElement` ids the three detoured functions each look up, confirmed by reading
    /// the installed `zoo.exe` directly: disassembling both `_fillListBox` addresses side-by-side
    /// shows them as byte-identical twins parameterized only by this id and which `getThoughtsBy*`
    /// they call (`FILL_LIST_BOX_0`/`0x00467a33` pushes `0xc35` and calls `GET_THOUGHTS_BY_OBJECT` at
    /// its confirmed address `0x00467b37`; `FILL_LIST_BOX_1`/`0x0046a040` pushes `0xd8d` and calls
    /// `GET_THOUGHTS_BY_THINKER` at its confirmed address `0x0046a7c8` - the exact match against
    /// `generated.rs`'s own addresses for those two is what confirms which twin is which, resolving
    /// the plan's own "confirm `FILL_LIST_BOX_1` is genuinely a twin" open question).
    /// `_refillThoughtsList`'s id (`0x10ea`) comes directly from its own decompile.
    const OBJECT_THOUGHTS_LIST_ELEMENT_ID: i32 = 0xc35;
    const THINKER_THOUGHTS_LIST_ELEMENT_ID: i32 = 0xd8d;
    const HABITAT_THOUGHTS_LIST_ELEMENT_ID: i32 = 0x10ea;

    /// Per-call-site match caps, read directly off each decompile's own third argument to
    /// `getThoughtsBy*`: both `_fillListBox` instantiations request at most 5, `_refillThoughtsList`
    /// requests at most `0x14` (20).
    const OBJECT_OR_THINKER_THOUGHTS_MAX_COUNT: usize = 5;
    const HABITAT_THOUGHTS_MAX_COUNT: usize = 20;

    /// `DAT_0063915c` (RVA `0x0023915c`) - the "habitat info" UI window's own currently-displayed
    /// habitat, a plain `*ZTHabitat` global entirely outside `ZTThoughtMgr`'s own state (set by
    /// `habitatinfo_setHabitat`, cleared by `habitatinfo_remove{Habitat,AllHabitats}` - none of which
    /// this plan reimplements). `_refillThoughtsList` reads it directly rather than taking it as a
    /// parameter (unlike the two `_fillListBox` instantiations, which take their filter pointer as
    /// their own single argument) - confirmed via the same direct-`zoo.exe`-read technique
    /// `ztresearch`'s own `DAT_*` constants use, decoding the label's embedded address the same way
    /// `GLOBAL_ZTThoughtMgr`'s own `0x00639090` was decoded elsewhere in this file's plan.
    const CURRENT_HABITAT_INFO_HABITAT_PTR_ADDR: u32 = 0x0023_915c;

    fn current_habitat_info_habitat_ptr() -> u32 {
        get_from_memory::<u32>(get_module_base("zoo.exe") as u32 + CURRENT_HABITAT_INFO_HABITAT_PTR_ADDR)
    }

    /// The fixed styling args every `addString` call site in this file passes, ported byte-for-byte
    /// from each decompile's own `uVar*` values (`_fillListBox.c`'s `uVar6..uVar13` and
    /// `_refillThoughtsList.c`'s `uVar8..uVar15` are, once matched to `ADD_STRING_0`'s confirmed
    /// `generated.rs` signature, the exact same eight values at every call site: four zeroed
    /// `*const i32` slots, a zero `u8` flag, a `1`-valued `*const i32` slot (a raw sentinel value, not
    /// a real pointer - ported as-is since neither its type nor meaning were independently confirmed),
    /// the `0x00ff00ff` display color, and a final null `*const i32`).
    const THOUGHT_LIST_ITEM_COLOR: u32 = 0x00ff_00ff;

    /// Builds a temporary `ZTBufferString`-shaped buffer for `text` and hands it to the real
    /// `UIListBox::addString` - the write-side counterpart to `ZTThought::getString`'s own
    /// `ZTBufferString`-shaped out-param (see that method's doc comment for the confirmation that
    /// vanilla's `getString` really does build exactly this 3-pointer shape). Unlike vanilla, which
    /// heap-allocates the buffer via its own small-object allocator and frees it again with
    /// `FUN_00401b16` once `addString` returns (`addString` copies the string into its own
    /// permanently-owned storage before returning - confirmed by reading `UIListBox::addString`'s own
    /// disassembly - so the source buffer is safe to free/drop immediately after), this just uses a
    /// plain Rust `Vec` for the temporary buffer and lets it drop normally - no vanilla allocator
    /// involvement needed for a buffer that never outlives this one call.
    #[allow(clippy::manual_dangling_ptr)] // literal sentinel value `1`, not a real pointer - `ptr::dangling` would substitute a different bit pattern (alignment-sized, not `1`)
    fn add_thought_to_list_box(list_box: *const u32, text: &str) {
        let mut encoded = encode_to_ansi(text);
        let len = encoded.len() as u32;
        encoded.push(0);
        let start = encoded.as_ptr() as u32;
        let text_buffer = ZTBufferString::from_raw_parts(start, start + len, start + encoded.len() as u32);
        unsafe {
            ADD_STRING_0.original()(
                list_box,
                &text_buffer as *const ZTBufferString as *const u32,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                1 as *const i32,
                THOUGHT_LIST_ITEM_COLOR,
                std::ptr::null(),
            );
        }
    }

    #[detour_mod]
    mod detours {
        use super::*;

        /// Reimplementation of the `_fillListBox` instantiation at `0x00467a33` - direct port of
        /// `_fillListBox.c`, minus the vanilla temporary-list construction (see the module doc
        /// comment), using [`OBJECT_THOUGHTS_LIST_ELEMENT_ID`]/[`get_thoughts_by_object`] per this
        /// twin's own confirmed element id/`getThoughtsBy*` call target.
        #[detour(FILL_LIST_BOX_0)]
        unsafe extern "cdecl" fn fill_object_thoughts_list_box(object_ptr: *const i32) {
            let element = unsafe { GET_ELEMENT_0.original()(global_bfuimgr(), OBJECT_THOUGHTS_LIST_ELEMENT_ID) };
            if element.is_null() {
                return;
            }
            unsafe { CLEAR.original()(element) };
            for thought in globals().ztthoughtmgr().get_thoughts_by_object(object_ptr as u32, OBJECT_OR_THINKER_THOUGHTS_MAX_COUNT) {
                add_thought_to_list_box(element, &thought.get_string());
            }
        }

        /// Reimplementation of the `_fillListBox` instantiation at `0x0046a040` - see
        /// `fill_object_thoughts_list_box`'s doc comment; this twin uses
        /// [`THINKER_THOUGHTS_LIST_ELEMENT_ID`]/[`get_thoughts_by_thinker`] instead.
        #[detour(FILL_LIST_BOX_1)]
        unsafe extern "cdecl" fn fill_thinker_thoughts_list_box(thinker_ptr: *const i32) {
            let element = unsafe { GET_ELEMENT_0.original()(global_bfuimgr(), THINKER_THOUGHTS_LIST_ELEMENT_ID) };
            if element.is_null() {
                return;
            }
            unsafe { CLEAR.original()(element) };
            for thought in globals().ztthoughtmgr().get_thoughts_by_thinker(thinker_ptr as u32, OBJECT_OR_THINKER_THOUGHTS_MAX_COUNT) {
                add_thought_to_list_box(element, &thought.get_string());
            }
        }

        /// Direct port of `_refillThoughtsList.c`, minus the vanilla temporary-list construction (see
        /// the module doc comment). `preserve_scroll` mirrors the decompile's own `param_1`: when set,
        /// the current scroll position is snapshotted into the list box's own save-slot fields
        /// (raw offsets `+0x36c`/`+0x37c` read, `+0x398`/`+0x39c`/`+0x3a0` written - confirmed directly
        /// from `_refillThoughtsList.asm`'s literal byte offsets, not the decompile's unreliable
        /// `this[4]`/`this[5]`-indexed rendering of the same fields) *before* the list is cleared and
        /// repopulated, so that the real `UIListBox::restoreState` call afterward has something to
        /// restore.
        #[detour(REFILL_THOUGHTS_LIST)]
        unsafe extern "cdecl" fn refill_thoughts_list(preserve_scroll: i8) {
            let element = unsafe { GET_ELEMENT_0.original()(global_bfuimgr(), HABITAT_THOUGHTS_LIST_ELEMENT_ID) };
            if element.is_null() {
                return;
            }

            let habitat_ptr = current_habitat_info_habitat_ptr();
            if habitat_ptr == 0 {
                unsafe { CLEAR.original()(element) };
                return;
            }

            if preserve_scroll != 0 {
                let element_addr = element as u32;
                let scroll_offset = get_from_memory::<u32>(element_addr + 0x36c);
                let scroll_extent = get_from_memory::<u32>(element_addr + 0x37c);
                save_to_memory::<u8>(element_addr + 0x398, 1);
                save_to_memory::<u32>(element_addr + 0x39c, scroll_offset);
                save_to_memory::<u32>(element_addr + 0x3a0, scroll_extent);
            }

            unsafe { CLEAR.original()(element) };
            for thought in globals().ztthoughtmgr().get_thoughts_by_habitat(habitat_ptr, HABITAT_THOUGHTS_MAX_COUNT) {
                add_thought_to_list_box(element, &thought.get_string());
            }
            if preserve_scroll != 0 {
                unsafe { RESTORE_STATE.original()(element) };
            }
        }
    }

    pub fn init() {
        if let Err(e) = unsafe { detours::init_detours() } {
            error!("Failed to initialise ztthoughtmgr UI-consumer detours: {e:?}");
        }
    }
}

/// Phase E: detours `ZTThoughtMgr`'s four mutating entry points - `addThought` and
/// `removeThoughtsBy{Thinker,Object,Habitat}` - onto the `impl ZTThoughtMgr` methods of the same name.
/// This is what makes the module doc comment's "exclusively Rust-owned from the point OpenZT loads
/// onward" claim actually hold: any vanilla code path that inserts or removes a node (not just the
/// three UI consumers Phase D closed the read side for) must go through `insert_front`/`remove_where`'s
/// `Box`-based allocator, never vanilla's own freelist one - so, unlike Phase C's `getThoughtsBy*`
/// (left un-detoured; read-only walks are safe against either allocator), these four *must* be detoured
/// at their real addresses, since detouring is the only way to redirect every caller, not just the ones
/// this plan happened to catalogue.
mod thought_mutator_detours {
    use openzt_detour::generated::ztthoughtmgr::{ADD_THOUGHT, REMOVE_THOUGHTS_BY_HABITAT, REMOVE_THOUGHTS_BY_OBJECT, REMOVE_THOUGHTS_BY_THINKER};
    use openzt_detour_macro::detour_mod;
    use tracing::error;

    use super::*;
    use crate::util::mut_from_memory;

    #[detour_mod]
    mod detours {
        use super::*;

        #[detour(ADD_THOUGHT)]
        unsafe extern "thiscall" fn add_thought(this: *const u32, string_id: u32, thinker_ptr: *const u32, object_ptr: *const u32, habitat_ptr: *const u32) {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.add_thought(string_id, thinker_ptr as u32, object_ptr as u32, habitat_ptr as u32);
        }

        #[detour(REMOVE_THOUGHTS_BY_THINKER)]
        unsafe extern "thiscall" fn remove_thoughts_by_thinker(this: *const u32, thinker_ptr: *const u32) {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.remove_thoughts_by_thinker(thinker_ptr as u32);
        }

        #[detour(REMOVE_THOUGHTS_BY_OBJECT)]
        unsafe extern "thiscall" fn remove_thoughts_by_object(this: *const u32, object_ptr: *const u32) {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.remove_thoughts_by_object(object_ptr as u32);
        }

        #[detour(REMOVE_THOUGHTS_BY_HABITAT)]
        unsafe extern "thiscall" fn remove_thoughts_by_habitat(this: *const u32, habitat_ptr: *const i32, force: i8) {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.remove_thoughts_by_habitat(habitat_ptr as u32, force != 0);
        }
    }

    pub fn init() {
        if let Err(e) = unsafe { detours::init_detours() } {
            error!("Failed to initialise ztthoughtmgr mutator detours: {e:?}");
        }
    }
}

/// Phase F: detours `ZTThoughtMgr::save`/`load`/`populateThoughts` onto the `impl ZTThoughtMgr` methods
/// of the same name - see those methods' own doc comments for the ported behavior. Like Phase E's
/// mutators, these must be real detours rather than plain Rust helpers: `ZTWorldMgr::load` (out of this
/// plan's scope, left as vanilla) calls `ZTThoughtMgr::load`/`populateThoughts` itself depending on save
/// version, so redirecting those calls is only possible by detouring their real addresses.
mod thought_save_detours {
    use openzt_detour::generated::ztthoughtmgr::{LOAD, POPULATE_THOUGHTS, SAVE};
    use openzt_detour_macro::detour_mod;
    use tracing::error;

    use super::*;
    use crate::util::mut_from_memory;

    #[detour_mod]
    mod detours {
        use super::*;

        #[detour(SAVE)]
        unsafe extern "thiscall" fn save(this: *const u32, file: *const u32) -> bool {
            unsafe { ref_from_memory::<ZTThoughtMgr>(this) }.save(file)
        }

        #[detour(LOAD)]
        unsafe extern "thiscall" fn load(this: *const u32, file: *const u32, version: u32) -> bool {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.load(file, version)
        }

        #[detour(POPULATE_THOUGHTS)]
        unsafe extern "thiscall" fn populate_thoughts(this: *const u32) {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.populate_thoughts();
        }
    }

    pub fn init() {
        if let Err(e) = unsafe { detours::init_detours() } {
            error!("Failed to initialise ztthoughtmgr save/load detours: {e:?}");
        }
    }
}

/// Phase G: detours `ZTThoughtMgr`'s vtable destructor slot - the scalar deleting destructor at
/// `0x0057d852` (`ZTTHOUGHT_MGR_1` in `generated.rs`; confirmed as the vtable's own dtor entry per the
/// plan's "Confirmed layout" section) - onto [`ZTThoughtMgr::clear`]. Per `ZTThoughtMgr_~ZTThoughtMgr.c`/
/// `.asm`, vanilla's own version of this function calls the real (un-decompiled on Windows, confirmed
/// via the macOS decompile - see `clear`'s own doc comment) destructor body, then conditionally calls
/// `operator delete` on `this` if the caller-supplied flag byte's low bit is set. Since `ZTThoughtMgr` is
/// a process-lifetime singleton (see the module doc comment) and no address for the real vanilla
/// `operator delete` this class would use is known or needed (mirroring the same "don't chase addresses
/// we don't need" reasoning the module doc comment already applies to the freelist allocator), this
/// reimplementation only ever frees the list's own `Box`-allocated nodes and never the flag-gated `this`
/// itself - deliberately not replicated here, matching the module doc comment's own claim that the outer
/// struct is never freed by this. `ZTTHOUGHT_MGR_0` (`0x0057d815`, the real destructor's own address,
/// only ever reached indirectly through this wrapper per the `.asm`) is intentionally left un-detoured:
/// nothing else in vanilla calls it directly.
mod thought_dtor_detour {
    use openzt_detour::generated::ztthoughtmgr::ZTTHOUGHT_MGR_1;
    use openzt_detour_macro::detour_mod;
    use tracing::error;

    use super::*;
    use crate::util::mut_from_memory;

    #[detour_mod]
    mod detours {
        use super::*;

        #[detour(ZTTHOUGHT_MGR_1)]
        unsafe extern "thiscall" fn ztthoughtmgr_dtor(this: *const u32, _flags: u8) -> *const u32 {
            unsafe { mut_from_memory::<ZTThoughtMgr>(this) }.clear();
            this
        }
    }

    pub fn init() {
        if let Err(e) = unsafe { detours::init_detours() } {
            error!("Failed to initialise ztthoughtmgr destructor detour: {e:?}");
        }
    }
}

/// Phase H: live-comparison test support for `reimplementation_tests` - builds/tears down standalone,
/// heap-allocated `ZTThoughtMgr`/`ZTThought` instances not spliced into the real singleton, and wraps
/// vanilla-allocated temporary list output for reading. Mirrors `ztmarketing::live_support`'s own
/// shape/gating (`#[cfg(feature = "reimplementation-tests")]`, `pub(crate)`), but - unlike research/
/// marketing's `ZTArray`-backed structs - has to build/walk `ZTThoughtMgr`'s own Box-owned intrusive
/// list shape instead (see the module doc comment's own "Rust-owned from the point OpenZT loads
/// onward" claim, which every helper here is careful to uphold: nothing here ever frees vanilla-
/// allocated memory through `Box`, or `Box`-allocated memory through vanilla's own freelist).
#[cfg(feature = "reimplementation-tests")]
pub(crate) mod live_support {
    use super::*;

    /// Builds a `ZTThought` with every field directly settable - unlike `ZTThought::new`, which
    /// dereferences `thinker_ptr`/`object_ptr`/`habitat_arg` when non-null to resolve
    /// `thinker_id`/`object_id`/the habitat-flag gate. Several live comparisons need fields set
    /// directly (e.g. a specific `thinker_id`/`object_id` pair with no live pointer behind it at all)
    /// without running that resolution logic.
    ///
    /// `vtable` is set to the same real `ZTThought` vtable address `ZTThought::new` itself uses, not
    /// `0` - confirmed live (a null-vtable node crashed `ZTTHOUGHTMGR_SAVE` outright) and via
    /// `ZTThoughtMgr_save.c`: `ZTThoughtMgr::save` doesn't call `ZTThought::save` directly, it dispatches
    /// through each node's own `data.vtable` slot 0 (`(**(code**)piVar4[2])(param_1)`, where
    /// `piVar4[2]` is the node's embedded `ZTThought`'s first field) - so every node reachable from a
    /// real vanilla `save`/`load`/anything-else-that-might-add-a-vtable-dispatch-later call needs a
    /// genuinely valid vtable, not just correct data fields.
    pub(crate) fn new_thought(
        string_id: u32,
        thinker_id: u32,
        object_id: u32,
        tile_x: i32,
        tile_y: i32,
        thinker_ptr: u32,
        object_ptr: u32,
        habitat_ptr: u32,
    ) -> ZTThought {
        let vtable = get_module_base("zoo.exe") as u32 + 0x0023_5400;
        ZTThought { vtable, string_id, thinker_id, object_id, tile_x, tile_y, thinker_ptr, object_ptr, habitat_ptr }
    }

    /// Builds a standalone `ZTThoughtMgr` with a freshly heap-allocated, self-referencing sentinel node
    /// - not spliced into the real singleton. Mirrors `tests::build_test_mgr`'s own construction, but
    /// heap-allocates the `ZTThoughtMgr` itself too (returned as a raw pointer, for passing to real
    /// vanilla `.original()()` calls, which `build_test_mgr`'s by-value return can't do).
    pub(crate) fn build_standalone_mgr(max_thoughts: u32) -> *mut ZTThoughtMgr {
        let sentinel = Box::into_raw(Box::new(ThoughtNode {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
            data: new_thought(0, 0, 0, -1, -1, 0, 0, 0),
        }));
        unsafe {
            (*sentinel).next = sentinel;
            (*sentinel).prev = sentinel;
        }
        Box::into_raw(Box::new(ZTThoughtMgr { vtable: 0, flag: 0, _pad: [0; 3], sentinel_ptr: sentinel as u32, max_thoughts }))
    }

    /// Frees every real node (via `clear`, the same primitive the real destructor detour uses), then
    /// the sentinel node and the `ZTThoughtMgr` allocation itself - the full teardown `build_standalone_mgr`
    /// needs (unlike the real singleton, a standalone test instance's own struct allocation and sentinel
    /// node both need freeing here, since nothing else ever will).
    pub(crate) fn destroy_standalone_mgr(ptr: *mut ZTThoughtMgr) {
        if ptr.is_null() {
            return;
        }
        let mgr = unsafe { &mut *ptr };
        mgr.clear();
        drop(unsafe { Box::from_raw(mgr.sentinel()) });
        drop(unsafe { Box::from_raw(ptr) });
    }

    /// Frees only the sentinel node and the `ZTThoughtMgr` allocation itself, WITHOUT walking/freeing any
    /// linked list nodes - use this instead of `destroy_standalone_mgr` whenever real vanilla code (the
    /// real, undetoured `ADD_THOUGHT`/`LOAD` - both confirmed via their own decompiles to splice newly
    /// allocated nodes into the list through `FUN_004230a6`, the same vanilla small-object freelist
    /// allocator `read_only_wrap_vanilla_list`'s own doc comment describes) may have linked nodes it
    /// allocated through that allocator into this manager's list: those nodes must never be freed through
    /// `Box` (a cross-allocator free is undefined behavior / heap corruption - exactly the class of bug
    /// this comparison harness exists to avoid triggering itself), so this deliberately leaks them. A
    /// one-time, per-proptest-case leak, reclaimed at process exit - mirrors
    /// `read_only_wrap_vanilla_list`'s own "never free vanilla-allocated memory through Box" rule. The
    /// sentinel node itself is still safe to free normally here: `ADD_THOUGHT`/`LOAD` only ever relink its
    /// `next`/`prev` fields to point at newly inserted nodes, never reallocate or hand its own address to
    /// the freelist, so it remains the same `Box` allocation `build_standalone_mgr` created throughout.
    pub(crate) fn destroy_standalone_mgr_leaking_nodes(ptr: *mut ZTThoughtMgr) {
        if ptr.is_null() {
            return;
        }
        let mgr = unsafe { &*ptr };
        drop(unsafe { Box::from_raw(mgr.sentinel()) });
        drop(unsafe { Box::from_raw(ptr) });
    }

    /// Wraps a vanilla-allocated sentinel pointer - the out-param `getThoughtsBy*` writes - as a
    /// throwaway `ZTThoughtMgr` purely so `.iter()` can walk it. Confirmed safe via
    /// `ZTThoughtMgr_getThoughtsByObject.c`: the temporary list's own sentinel is self-referencing
    /// (`extraout_EAX[0] = extraout_EAX; extraout_EAX[1] = extraout_EAX`) and its matched-entry nodes
    /// are `{next, prev, ZTThought}` at stride `0x2c` (`FUN_00401b16(...,0x2c)`) - byte-for-byte
    /// identical to our own `ThoughtNode`/persistent-list shape; only the allocator differs (vanilla's
    /// own small-object freelist, confirmed via that same decompile's `FUN_0040107f`/`FUN_004230a6`
    /// calls, never `Box`). Never call any mutating method (`insert_front`/`remove_where`/`clear`/...)
    /// on the result - freeing or writing through `Box`-shaped assumptions here would corrupt the
    /// freelist heap. The vanilla list itself is deliberately never freed by this test harness at all -
    /// there's no confirmed address for the freelist's own free-node routine (see the module doc
    /// comment's own "whose low-level alloc/free helpers we don't have Windows addresses for") - a
    /// one-time, per-proptest-case leak of a handful of `0x2c`-byte nodes, reclaimed at process exit.
    pub(crate) fn read_only_wrap_vanilla_list(sentinel_ptr: u32) -> ZTThoughtMgr {
        ZTThoughtMgr { vtable: 0, flag: 0, _pad: [0; 3], sentinel_ptr, max_thoughts: u32::MAX }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thought_fixture(string_id: u32, thinker_id: u32) -> ZTThought {
        ZTThought {
            vtable: 0,
            string_id,
            thinker_id,
            object_id: 0,
            tile_x: -1,
            tile_y: -1,
            thinker_ptr: 0,
            object_ptr: 0,
            habitat_ptr: 0,
        }
    }

    fn thought_fixture_with_ptrs(string_id: u32, thinker_ptr: u32, object_ptr: u32, habitat_ptr: u32) -> ZTThought {
        ZTThought {
            vtable: 0,
            string_id,
            thinker_id: 0,
            object_id: 0,
            tile_x: -1,
            tile_y: -1,
            thinker_ptr,
            object_ptr,
            habitat_ptr,
        }
    }

    /// Builds a standalone `ZTThoughtMgr` with a freshly heap-allocated, self-referencing sentinel
    /// node - never spliced into the real singleton. Leaks the sentinel (acceptable for short-lived
    /// unit tests, mirrors `zthabitatmgr`/`ztworldmgr`'s own `new_for_test` fixtures).
    fn build_test_mgr(max_thoughts: u32) -> ZTThoughtMgr {
        let sentinel = Box::into_raw(Box::new(ThoughtNode {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
            data: thought_fixture(0, 0),
        }));
        unsafe {
            (*sentinel).next = sentinel;
            (*sentinel).prev = sentinel;
        }
        ZTThoughtMgr { vtable: 0, flag: 0, _pad: [0; 3], sentinel_ptr: sentinel as u32, max_thoughts }
    }

    #[test]
    fn new_mgr_is_empty() {
        let mgr = build_test_mgr(1000);
        assert_eq!(mgr.len(), 0);
        assert!(mgr.is_empty());
        assert!(mgr.iter().next().is_none());
    }

    #[test]
    fn insert_front_adds_most_recent_first() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture(1, 0));
        mgr.insert_front(thought_fixture(2, 0));
        mgr.insert_front(thought_fixture(3, 0));

        let ids: Vec<u32> = mgr.iter().map(|t| t.string_id()).collect();
        assert_eq!(ids, vec![3, 2, 1]);
        assert_eq!(mgr.len(), 3);
    }

    #[test]
    fn insert_front_trims_oldest_once_over_cap() {
        let mut mgr = build_test_mgr(2);
        mgr.insert_front(thought_fixture(1, 0));
        mgr.insert_front(thought_fixture(2, 0));
        mgr.insert_front(thought_fixture(3, 0));

        let ids: Vec<u32> = mgr.iter().map(|t| t.string_id()).collect();
        assert_eq!(ids, vec![3, 2]);
        assert_eq!(mgr.len(), 2);
    }

    #[test]
    fn insert_front_with_zero_cap_keeps_list_empty() {
        let mut mgr = build_test_mgr(0);
        mgr.insert_front(thought_fixture(1, 0));
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn remove_where_frees_matching_nodes_only() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture(1, 10));
        mgr.insert_front(thought_fixture(2, 20));
        mgr.insert_front(thought_fixture(3, 10));

        mgr.remove_where(|t| t.thinker_id() == 10);

        let ids: Vec<u32> = mgr.iter().map(|t| t.string_id()).collect();
        assert_eq!(ids, vec![2]);
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn remove_where_matching_nothing_is_a_no_op() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture(1, 10));
        mgr.remove_where(|t| t.thinker_id() == 999);
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn remove_where_can_empty_the_list() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture(1, 10));
        mgr.insert_front(thought_fixture(2, 10));
        mgr.remove_where(|t| t.thinker_id() == 10);
        assert!(mgr.is_empty());
    }

    #[test]
    fn get_thoughts_by_thinker_filters_and_caps() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 10, 0, 0));
        mgr.insert_front(thought_fixture_with_ptrs(2, 20, 0, 0));
        mgr.insert_front(thought_fixture_with_ptrs(3, 10, 0, 0));
        mgr.insert_front(thought_fixture_with_ptrs(4, 10, 0, 0));

        let ids: Vec<u32> = mgr.get_thoughts_by_thinker(10, 2).iter().map(|t| t.string_id()).collect();
        // Selection is capped at 2 even though 3 thoughts match (most-recently-inserted first: 4, then
        // 3), but the returned order is reversed relative to selection order - see
        // `get_thoughts_by_thinker`'s own doc comment for the confirmed-live reasoning why.
        assert_eq!(ids, vec![3, 4]);
    }

    #[test]
    fn get_thoughts_by_object_filters_by_object_ptr_only() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 99, 0));
        mgr.insert_front(thought_fixture_with_ptrs(2, 0, 100, 0));

        let ids: Vec<u32> = mgr.get_thoughts_by_object(99, 10).iter().map(|t| t.string_id()).collect();
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn get_thoughts_by_habitat_filters_by_habitat_ptr_only() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 0, 55));
        mgr.insert_front(thought_fixture_with_ptrs(2, 0, 0, 55));
        mgr.insert_front(thought_fixture_with_ptrs(3, 0, 0, 56));

        let ids: Vec<u32> = mgr.get_thoughts_by_habitat(55, 10).iter().map(|t| t.string_id()).collect();
        // Selection order (most-recently-inserted first) is [2, 1]; returned order is reversed - see
        // `get_thoughts_by_thinker`'s own doc comment.
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn get_thoughts_by_thinker_with_no_match_is_empty() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 10, 0, 0));
        assert!(mgr.get_thoughts_by_thinker(999, 10).is_empty());
    }

    #[test]
    fn get_thoughts_by_thinker_zero_max_count_returns_empty() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 10, 0, 0));
        assert!(mgr.get_thoughts_by_thinker(10, 0).is_empty());
    }

    #[test]
    fn substitution_uses_provided_name_when_present() {
        assert_eq!(substitute_thought_string(Some("Caught a %s!".to_string()), Some("Zebra")), "Caught a Zebra!");
    }

    #[test]
    fn substitution_falls_back_to_template_when_none_available() {
        assert_eq!(substitute_thought_string(Some("Bored".to_string()), None), "Bored");
    }

    #[test]
    fn substitution_missing_template_returns_empty_string() {
        assert_eq!(substitute_thought_string(None, Some("Zebra")), "");
    }

    #[test]
    fn substitution_empty_template_returned_as_is() {
        assert_eq!(substitute_thought_string(Some(String::new()), Some("Zebra")), "");
    }

    #[test]
    fn substitution_only_replaces_first_occurrence() {
        assert_eq!(substitute_thought_string(Some("%s and %s".to_string()), Some("X")), "X and %s");
    }

    #[test]
    fn remove_thoughts_by_thinker_removes_only_matching() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 10, 0, 0));
        mgr.insert_front(thought_fixture_with_ptrs(2, 20, 0, 0));
        mgr.insert_front(thought_fixture_with_ptrs(3, 10, 0, 0));

        mgr.remove_thoughts_by_thinker(10);

        let ids: Vec<u32> = mgr.iter().map(|t| t.string_id()).collect();
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn remove_thoughts_by_object_removes_only_matching() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 99, 0));
        mgr.insert_front(thought_fixture_with_ptrs(2, 0, 100, 0));

        mgr.remove_thoughts_by_object(99);

        let ids: Vec<u32> = mgr.iter().map(|t| t.string_id()).collect();
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn remove_thoughts_by_habitat_force_true_always_removes() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 99, 55)); // has an object, but force wins anyway
        mgr.insert_front(thought_fixture_with_ptrs(2, 0, 0, 55));

        mgr.remove_thoughts_by_habitat(55, true);

        assert!(mgr.is_empty());
    }

    #[test]
    fn remove_thoughts_by_habitat_force_false_with_object_only_clears_habitat_link() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 99, 55));

        mgr.remove_thoughts_by_habitat(55, false);

        assert_eq!(mgr.len(), 1);
        let survivor = mgr.iter().next().unwrap();
        assert_eq!(survivor.object_ptr(), 99);
        assert_eq!(survivor.habitat_ptr(), 0);
    }

    #[test]
    fn remove_thoughts_by_habitat_force_false_without_object_removes_node() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 0, 55));

        mgr.remove_thoughts_by_habitat(55, false);

        assert!(mgr.is_empty());
    }

    #[test]
    fn remove_thoughts_by_habitat_ignores_non_matching() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture_with_ptrs(1, 0, 99, 55));

        mgr.remove_thoughts_by_habitat(56, true);

        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn clear_frees_every_node_and_leaves_list_empty() {
        let mut mgr = build_test_mgr(1000);
        mgr.insert_front(thought_fixture(1, 10));
        mgr.insert_front(thought_fixture(2, 20));
        mgr.insert_front(thought_fixture(3, 30));

        mgr.clear();

        assert!(mgr.is_empty());
        assert!(mgr.iter().next().is_none());
    }

    #[test]
    fn clear_on_empty_list_is_a_no_op() {
        let mut mgr = build_test_mgr(1000);
        mgr.clear();
        assert!(mgr.is_empty());
    }
}
