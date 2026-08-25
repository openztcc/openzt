//! Structs and methods for the vanilla `ZTThoughtMgr`/`ZTThought` classes, which track the "thought
//! bubble" messages guests/animals display over their heads (e.g. "caught prey", template string id
//! `0x280a`) - a simple, small `BFMgr`-derived class owning a single intrusive, sentinel-terminated
//! linked list of `ZTThought` records.
//!
//! Unlike `ZTResearchMgr`/`ZTMarketingMgr`, the persistent list and the UI's temporary output lists
//! both use vanilla's own small-object freelist allocator, which has no confirmed Windows address for
//! its low-level alloc/free helpers. This module's list is therefore *exclusively* Rust-owned
//! (`Box`-allocated nodes) from the point OpenZT loads onward - the only vanilla-allocated survivor is
//! the original sentinel node `CreateZTThoughtMgr` allocates at startup (left un-detoured), which the
//! list helpers below must never attempt to free.

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
/// active `ZTThought`. Allocated as 16 bytes (`operator_new(0x10)`).
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

/// One thought bubble entry. Own 2-entry vtable (`save`/`load`), no base class. Total size `0x24`
/// (36 bytes = 9 x u32/i32 fields). The persistent list's nodes are allocated at `0x30` - 4 bytes
/// more than `0x24`, likely the block allocator's size-class rounding.
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

    /// Called after a save-file load to re-derive the live, non-persistent pointer fields
    /// (`thinker_ptr`/`object_ptr`/`habitat_ptr`, plus a possible `tile_x`/`tile_y` refresh) from the
    /// persisted `thinker_id`/`object_id`/`tile_x`/`tile_y` alone.
    ///
    /// `thinker_id`/`object_id` are unconditionally re-resolved via `ZTWorldMgr::resolve_entity_by_id`.
    ///
    /// If `object_ptr` resolved, [`resolve_object_own_habitat_ptr`] is consulted; if it resolves, its
    /// result becomes `habitat_ptr` and, if non-null, `tile_x`/`tile_y` are refreshed from that
    /// habitat's own gate tile.
    ///
    /// Finally, if `habitat_ptr` is still unresolved and a real tile is set, it's recomputed via
    /// `ZTHabitatMgr::getHabitat`.
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

    /// `habitat_arg` is only accepted into `habitat_ptr` if the pointed-to `ZTHabitat`'s own flag at
    /// `+0x2c` is unset, and, when accepted, `tile_x`/`tile_y` are immediately refreshed from that
    /// habitat's own gate tile. `thinker_id`/`object_id` are resolved from `thinker_ptr`/`object_ptr`'s
    /// own `BFEntity::id` whenever those pointers are non-null. All-zero arguments (as
    /// `ZTThoughtMgr::load` passes before overwriting the fields itself) touch no real memory at all.
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

    /// Writes `string_id`, `thinker_id`, `object_id`, `tile_x`, `tile_y` as five little-endian dwords,
    /// in that order. Every write happens regardless of an earlier one failing; `ok` only reflects
    /// whether all five succeeded.
    pub fn save(&self, file: *const u32) -> bool {
        let mut ok = write_dword(file, self.string_id);
        ok &= write_dword(file, self.thinker_id);
        ok &= write_dword(file, self.object_id);
        ok &= write_dword(file, self.tile_x as u32);
        ok &= write_dword(file, self.tile_y as u32);
        ok
    }

    /// Reads `string_id` first, unconditionally, then branches on `version` for the remaining fields'
    /// read order - a pre-`0x1e` save only ever wrote `string_id`/`object_id`/`thinker_id` (no tile),
    /// so the legacy branch reads exactly those 3 fields in that order and leaves `tile_x`/`tile_y` at
    /// their ctor default of `-1`; `version >= 0x1e` reads all 5 fields in `save`'s own order
    /// (`thinker_id`, `object_id`, `tile_x`, `tile_y`). Every read is attempted regardless of an
    /// earlier one failing.
    ///
    /// If every read succeeded and `version > 0x1d`, re-resolves `thinker_ptr`/`object_ptr` via
    /// `ZTWorldMgr::resolve_entity_by_id`, then `habitat_ptr` directly via `ZTHabitatMgr::getHabitat` if
    /// a real tile is set - a simpler, tile-only resolution than `populate`'s own object-vtable-driven
    /// version.
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

    /// Loads the thought's own template string, then applies at most one `%s` substitution - the
    /// resolved object's name if `object_ptr` is set, else the resolved habitat's exhibit name if
    /// `habitat_ptr` is set (object always takes priority over habitat), else no substitution at all.
    /// See `substitute_thought_string` for the pure substitution logic this delegates to.
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
/// real memory. A missing template returns an empty string; an empty template is returned as-is.
/// Otherwise `%s` is replaced with `substitution` if one was resolved, or the template is returned
/// unmodified if not.
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
/// `ZTThoughtMgr::addThought`'s override of the caller-supplied habitat argument. `object_ptr` must be
/// non-null.
///
/// Calls the object's own entity type's vtable slot `0x1c` with a fixed data-segment argument; if that
/// returns `true`, calls the object itself's vtable slot `0x24c` (no arguments) and returns that result
/// as `Some` - even if it's null, matching vanilla's own unconditional assignment. Returns `None` if
/// the type check fails, meaning "no override" - the caller should fall back to whatever habitat it
/// already had.
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

/// Reads a single little-endian dword via the real vanilla read primitive, shared by
/// `ZTThought::load`/`ZTThoughtMgr::load`. `None` on a short/failed read.
fn read_dword(file: *const u32) -> Option<u32> {
    let mut buf = 0u32;
    let ok = unsafe { DEALLOCATE.original()(&mut buf as *mut u32 as *const u32, 4, 1, file as *const u8) };
    (ok == 1).then_some(buf)
}

/// A persistent-list node: 8 bytes of intrusive links followed by the `ZTThought` payload at `+0x8`.
/// The sentinel node vanilla allocates at startup shares this same link layout (its `data` is never
/// read).
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
    /// sentinel node.
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

    /// Splices `thought` in as a new `Box`-owned node at the front of the list, uncapped.
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
    /// `ZTThoughtMgr::load` builds on (the opposite end from `link_front`).
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
    /// sentinel. Shared removal primitive for `removeThoughtsBy{Thinker,Habitat,Object}`.
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

    /// Minus the vanilla temporary-`std::list` construction the decompile builds and then immediately
    /// frees again - a `Vec` collected directly from `iter()` is a drop-in behavioral replacement for
    /// what every caller actually consumes: an ordered, `max_count`-bounded sequence of matching
    /// `ZTThought`s.
    ///
    /// Selection walks `iter()`'s own front-to-back (most-recently-added-first) order with a
    /// `max_count`-then-stop cap, but the final returned order is oldest-of-the-selected-first, the
    /// reverse of the walk/selection order - hence the explicit `.reverse()` below.
    ///
    /// Matches on `thinker_ptr` (the resolved live pointer), not `thinker_id` - vanilla itself never
    /// compares against the persisted id here.
    pub fn get_thoughts_by_thinker(&self, thinker_ptr: u32, max_count: usize) -> Vec<&ZTThought> {
        let mut matches: Vec<&ZTThought> = self.iter().filter(|t| t.thinker_ptr() == thinker_ptr).take(max_count).collect();
        matches.reverse();
        matches
    }

    /// See `get_thoughts_by_thinker`'s doc comment for the shared reasoning, including why the result
    /// is reversed after selection. Matches on `object_ptr`.
    pub fn get_thoughts_by_object(&self, object_ptr: u32, max_count: usize) -> Vec<&ZTThought> {
        let mut matches: Vec<&ZTThought> = self.iter().filter(|t| t.object_ptr() == object_ptr).take(max_count).collect();
        matches.reverse();
        matches
    }

    /// See `get_thoughts_by_thinker`'s doc comment for the shared reasoning, including why the result
    /// is reversed after selection. Matches on `habitat_ptr`.
    pub fn get_thoughts_by_habitat(&self, habitat_ptr: u32, max_count: usize) -> Vec<&ZTThought> {
        let mut matches: Vec<&ZTThought> = self.iter().filter(|t| t.habitat_ptr() == habitat_ptr).take(max_count).collect();
        matches.reverse();
        matches
    }

    /// Uses `insert_front`, which already trims to `max_thoughts` after every insert.
    ///
    /// If `object_ptr` is non-null, [`resolve_object_own_habitat_ptr`] is consulted first: when it
    /// resolves, its result overrides whatever `habitat_ptr` the caller passed - even if that resolves
    /// to null. Otherwise `habitat_ptr` is passed to `ZTThought::new` unchanged.
    pub fn add_thought(&mut self, string_id: u32, thinker_ptr: u32, object_ptr: u32, habitat_ptr: u32) {
        let habitat_arg = if object_ptr != 0 { resolve_object_own_habitat_ptr(object_ptr).unwrap_or(habitat_ptr) } else { habitat_ptr };
        self.insert_front(ZTThought::new(string_id, thinker_ptr, object_ptr, habitat_arg));
    }

    /// Matches on `thinker_ptr`, same field `get_thoughts_by_thinker` matches on.
    pub fn remove_thoughts_by_thinker(&mut self, thinker_ptr: u32) {
        self.remove_where(|t| t.thinker_ptr() == thinker_ptr);
    }

    /// Matches on `object_ptr`, same field `get_thoughts_by_object` matches on.
    pub fn remove_thoughts_by_object(&mut self, object_ptr: u32) {
        self.remove_where(|t| t.object_ptr() == object_ptr);
    }

    /// Unlike `remove_where`-based removal, this doesn't just remove matches: for every thought whose
    /// `habitat_ptr` matches, if `force` is `false` *and* the thought still has a live `object_ptr`,
    /// only the habitat link is cleared (`habitat_ptr = 0`) and the thought itself survives; otherwise
    /// (a forced removal, or the thought has no object of its own to keep it alive) the node is fully
    /// unlinked and freed.
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

    /// Writes the list's own length as a leading dword, then calls `ZTThought::save` on every thought,
    /// front to back. Every item is visited regardless of an earlier write failing; `ok` only reflects
    /// whether everything wrote successfully.
    pub fn save(&self, file: *const u32) -> bool {
        let mut ok = write_dword(file, self.len() as u32);
        for thought in self.iter() {
            ok &= thought.save(file);
        }
        ok
    }

    /// Reads a leading dword count (interpreted as signed - a failed or negative/zero count leaves the
    /// list untouched and returns immediately). For each of `count` records, default-constructs a
    /// fresh `ZTThought` and calls `ZTThought::load` on it. A record is only spliced into the list (via
    /// `link_back` - `load` itself never trims to `max_thoughts`, unlike `addThought`) if the read
    /// succeeded *and* every non-zero id it carries actually resolved to a live pointer (for
    /// `version >= 0x1e` streams, where `ZTThought::load` already attempted that resolution inline); a
    /// record whose reference no longer resolves is silently dropped. Surviving records end up in read
    /// order, oldest-record-first at the front.
    /// Returns the AND of every item's own `load` result (the count read's own success is only a
    /// precondition to entering the loop at all, not folded into the final result).
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

    /// Calls `ZTThought::populate` on every thought in the list, front to back. Called by vanilla's own
    /// `ZTWorldMgr::load` for a specific pre-`0x1e` save-version range - not part of `ZTThoughtMgr::load`
    /// itself, which only performs this resolution inline for `version >= 0x1e` streams.
    pub fn populate_thoughts(&mut self) {
        for thought in self.iter_mut() {
            thought.populate();
        }
    }

    /// Vanilla's destructor calls a `std::list` "erase whole range" helper over the list, i.e. destroys
    /// and frees every real node - `remove_where(|_| true)` is exactly that, reusing the same
    /// `Box`-freeing primitive every other mutator does. The sentinel node and the `ZTThoughtMgr`
    /// struct itself are never freed here, matching vanilla.
    pub fn clear(&mut self) {
        self.remove_where(|_| true);
    }
}

/// Registers this module's live detours: the UI-consumer detours, the mutator detours
/// (`addThought`/`removeThoughtsBy{Thinker,Object,Habitat}`), the save/load-family detours
/// (`save`/`load`/`populateThoughts`), and the destructor detour.
pub fn init() {
    thought_ui_detours::init();
    thought_mutator_detours::init();
    thought_save_detours::init();
    thought_dtor_detour::init();
}

/// Detours the three UI functions that used to be `getThoughtsBy*`'s only consumers of the vanilla
/// `std::list` those built - `_fillListBox` (both instantiations) and `_refillThoughtsList`. Rewritten
/// to call the `Vec`-returning accessors above directly and drive
/// `BFUIMgr::getElement`/`UIListBox::clear`/`addString`/`restoreState` (all real vanilla functions,
/// called via `.original()`, never detoured themselves) in a loop instead. This is what makes the
/// `Vec` return type viable: once these three are detoured, nothing vanilla-side ever constructs or
/// walks a `getThoughtsBy*` result as a real intrusive list again.
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

    /// `GLOBAL_BFUIMgr`'s own fixed address.
    fn global_bfuimgr() -> *const u32 {
        (get_module_base("zoo.exe") as u32 + 0x0023_8de0) as *const u32
    }

    /// The `BFUIMgr::getElement` ids the three detoured functions each look up: the two `_fillListBox`
    /// instantiations are byte-identical twins parameterized only by this id and which `getThoughtsBy*`
    /// they call.
    const OBJECT_THOUGHTS_LIST_ELEMENT_ID: i32 = 0xc35;
    const THINKER_THOUGHTS_LIST_ELEMENT_ID: i32 = 0xd8d;
    const HABITAT_THOUGHTS_LIST_ELEMENT_ID: i32 = 0x10ea;

    /// Per-call-site match caps: both `_fillListBox` instantiations request at most 5,
    /// `_refillThoughtsList` requests at most `0x14` (20).
    const OBJECT_OR_THINKER_THOUGHTS_MAX_COUNT: usize = 5;
    const HABITAT_THOUGHTS_MAX_COUNT: usize = 20;

    /// The "habitat info" UI window's own currently-displayed habitat, a plain `*ZTHabitat` global
    /// entirely outside `ZTThoughtMgr`'s own state (set by `habitatinfo_setHabitat`, cleared by
    /// `habitatinfo_remove{Habitat,AllHabitats}` - neither reimplemented here). `_refillThoughtsList`
    /// reads it directly rather than taking it as a parameter.
    const CURRENT_HABITAT_INFO_HABITAT_PTR_ADDR: u32 = 0x0023_915c;

    fn current_habitat_info_habitat_ptr() -> u32 {
        get_from_memory::<u32>(get_module_base("zoo.exe") as u32 + CURRENT_HABITAT_INFO_HABITAT_PTR_ADDR)
    }

    /// The fixed styling args every `addString` call site in this file passes: four zeroed `*const i32`
    /// slots, a zero `u8` flag, a `1`-valued `*const i32` slot (a raw sentinel value, not a real
    /// pointer), the display color, and a final null `*const i32`.
    const THOUGHT_LIST_ITEM_COLOR: u32 = 0x00ff_00ff;

    /// Builds a temporary `ZTBufferString`-shaped buffer for `text` and hands it to the real
    /// `UIListBox::addString`. Unlike vanilla, which heap-allocates the buffer via its own
    /// small-object allocator and frees it once `addString` returns (`addString` copies the string
    /// into its own permanently-owned storage before returning), this just uses a plain Rust `Vec` for
    /// the temporary buffer and lets it drop normally.
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

        /// The `_fillListBox` instantiation at `0x00467a33`, using
        /// [`OBJECT_THOUGHTS_LIST_ELEMENT_ID`]/[`get_thoughts_by_object`].
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

        /// The `_fillListBox` instantiation at `0x0046a040`, using
        /// [`THINKER_THOUGHTS_LIST_ELEMENT_ID`]/[`get_thoughts_by_thinker`].
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

        /// `preserve_scroll`: when set, the current scroll position is snapshotted into the list box's
        /// own save-slot fields before the list is cleared and repopulated, so that the real
        /// `UIListBox::restoreState` call afterward has something to restore.
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

/// Detours `ZTThoughtMgr`'s four mutating entry points - `addThought` and
/// `removeThoughtsBy{Thinker,Object,Habitat}` - onto the `impl ZTThoughtMgr` methods of the same name.
/// Any vanilla code path that inserts or removes a node must go through `insert_front`/`remove_where`'s
/// `Box`-based allocator, never vanilla's own freelist one, so these four are detoured at their real
/// addresses (unlike the read-only `getThoughtsBy*` accessors, which are safe against either
/// allocator and left un-detoured).
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

/// Detours `ZTThoughtMgr::save`/`load`/`populateThoughts` onto the `impl ZTThoughtMgr` methods of the
/// same name. Like the mutator detours, these must be real detours rather than plain Rust helpers:
/// `ZTWorldMgr::load` (out of scope here, left as vanilla) calls `ZTThoughtMgr::load`/`populateThoughts`
/// itself depending on save version.
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

/// Detours `ZTThoughtMgr`'s vtable destructor slot - the scalar deleting destructor at `0x0057d852`
/// (`ZTTHOUGHT_MGR_1` in `generated.rs`) - onto [`ZTThoughtMgr::clear`]. Vanilla's own version of this
/// function calls the real destructor body, then conditionally calls `operator delete` on `this` if the
/// caller-supplied flag byte's low bit is set. Since `ZTThoughtMgr` is a process-lifetime singleton and
/// no address for the real vanilla `operator delete` this class would use is known or needed, this
/// reimplementation only ever frees the list's own `Box`-allocated nodes and never the flag-gated
/// `this` itself. `ZTTHOUGHT_MGR_0` (`0x0057d815`, the real destructor's own address, only ever reached
/// indirectly through this wrapper) is intentionally left un-detoured: nothing else in vanilla calls it
/// directly.
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

/// Live-comparison test support for `reimplementation_tests` - builds/tears down standalone,
/// heap-allocated `ZTThoughtMgr`/`ZTThought` instances not spliced into the real singleton, and wraps
/// vanilla-allocated temporary list output for reading. Nothing here ever frees vanilla-allocated
/// memory through `Box`, or `Box`-allocated memory through vanilla's own freelist.
#[cfg(feature = "reimplementation-tests")]
pub(crate) mod live_support {
    use super::*;

    /// Builds a `ZTThought` with every field directly settable - unlike `ZTThought::new`, which
    /// dereferences `thinker_ptr`/`object_ptr`/`habitat_arg` when non-null to resolve
    /// `thinker_id`/`object_id`/the habitat-flag gate. Several live comparisons need fields set
    /// directly without running that resolution logic.
    ///
    /// `vtable` is set to the same real `ZTThought` vtable address `ZTThought::new` itself uses, not
    /// `0`: `ZTThoughtMgr::save` dispatches through each node's own `data.vtable` slot 0 rather than
    /// calling `ZTThought::save` directly, so every node reachable from a real vanilla call needs a
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
    /// - not spliced into the real singleton. Heap-allocates the `ZTThoughtMgr` itself too (returned as
    /// a raw pointer, for passing to real vanilla `.original()()` calls).
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
    /// the sentinel node and the `ZTThoughtMgr` allocation itself.
    pub(crate) fn destroy_standalone_mgr(ptr: *mut ZTThoughtMgr) {
        if ptr.is_null() {
            return;
        }
        let mgr = unsafe { &mut *ptr };
        mgr.clear();
        drop(unsafe { Box::from_raw(mgr.sentinel()) });
        drop(unsafe { Box::from_raw(ptr) });
    }

    /// Frees only the sentinel node and the `ZTThoughtMgr` allocation itself, without walking/freeing
    /// any linked list nodes - use this instead of `destroy_standalone_mgr` whenever real vanilla code
    /// (the real, undetoured `ADD_THOUGHT`/`LOAD`) may have linked nodes it allocated through vanilla's
    /// own small-object freelist into this manager's list: those nodes must never be freed through
    /// `Box` (a cross-allocator free is undefined behavior / heap corruption), so this deliberately
    /// leaks them - a one-time, per-proptest-case leak, reclaimed at process exit. The sentinel node
    /// itself is still safe to free normally here: `ADD_THOUGHT`/`LOAD` only ever relink its
    /// `next`/`prev` fields to point at newly inserted nodes, never reallocate or hand its own address
    /// to the freelist.
    pub(crate) fn destroy_standalone_mgr_leaking_nodes(ptr: *mut ZTThoughtMgr) {
        if ptr.is_null() {
            return;
        }
        let mgr = unsafe { &*ptr };
        drop(unsafe { Box::from_raw(mgr.sentinel()) });
        drop(unsafe { Box::from_raw(ptr) });
    }

    /// Wraps a vanilla-allocated sentinel pointer - the out-param `getThoughtsBy*` writes - as a
    /// throwaway `ZTThoughtMgr` purely so `.iter()` can walk it. The temporary list's own sentinel is
    /// self-referencing and its matched-entry nodes are `{next, prev, ZTThought}` at stride `0x2c` -
    /// byte-for-byte identical to our own `ThoughtNode`/persistent-list shape; only the allocator
    /// differs (vanilla's own small-object freelist, never `Box`). Never call any mutating method
    /// (`insert_front`/`remove_where`/`clear`/...) on the result - freeing or writing through
    /// `Box`-shaped assumptions here would corrupt the freelist heap. The vanilla list itself is
    /// deliberately never freed by this test harness - there's no confirmed address for the freelist's
    /// own free-node routine - a one-time, per-proptest-case leak of a handful of `0x2c`-byte nodes,
    /// reclaimed at process exit.
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
    /// unit tests).
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
        // 3), but the returned order is reversed relative to selection order.
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
        // Selection order (most-recently-inserted first) is [2, 1]; returned order is reversed.
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
