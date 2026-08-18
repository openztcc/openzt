//! Redirects `standalone::WRITE_BYTES_TO_FILE`/`standalone::DEALLOCATE` (the `fwrite`/`fread`-shaped
//! primitives `ZTResearchMgr::save`/`load` - and every other vanilla `*::save`/`*::load` - go through)
//! to in-memory buffers instead of a real file, so the live save/load comparison tests in `mod.rs` can
//! call `SAVE.original()`/`LOAD.original()` without a real save file: `save`'s writes are captured into
//! a `Vec<u8>`, and `load`'s reads are served from a pre-filled `Vec<u8>`.
//!
//! The redirect is only active for the duration of one `begin_capture`/`end_capture` or
//! `begin_replay`/`end_replay` window (tracked per-thread, since detours run on the calling thread
//! synchronously). Outside that window both functions pass straight through to the real
//! implementation - harmless, since nothing else in the `reimplementation-tests` build ever calls
//! either (this feature build never initializes `save_logging`'s own passthrough+log detour on
//! `WRITE_BYTES_TO_FILE`; see `lib.rs`/`DllMain` in `openzt-test-dll`).

use std::cell::{Cell, RefCell};

use openzt_detour::generated::standalone::{DEALLOCATE, WRITE_BYTES_TO_FILE};
use openzt_detour_macro::detour_mod;

thread_local! {
    static REDIRECT_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static CAPTURE_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static REPLAY_CURSOR: RefCell<(Vec<u8>, usize)> = const { RefCell::new((Vec::new(), 0)) };
}

/// Starts redirecting `WRITE_BYTES_TO_FILE` calls into an in-memory buffer instead of a real file.
pub fn begin_capture() {
    CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().clear());
    REDIRECT_ACTIVE.with(|active| active.set(true));
}

/// Stops redirecting and returns everything captured since `begin_capture`.
pub fn end_capture() -> Vec<u8> {
    REDIRECT_ACTIVE.with(|active| active.set(false));
    CAPTURE_BUFFER.with(|buffer| std::mem::take(&mut *buffer.borrow_mut()))
}

/// Starts redirecting `DEALLOCATE` calls to read from `bytes` instead of a real file.
pub fn begin_replay(bytes: Vec<u8>) {
    REPLAY_CURSOR.with(|cursor| *cursor.borrow_mut() = (bytes, 0));
    REDIRECT_ACTIVE.with(|active| active.set(true));
}

/// Stops redirecting.
pub fn end_replay() {
    REDIRECT_ACTIVE.with(|active| active.set(false));
}

#[detour_mod]
mod detours {
    use super::*;

    #[detour(WRITE_BYTES_TO_FILE)]
    unsafe extern "cdecl" fn write_bytes_to_file(source_ptr: *const u32, size_in_bytes: u32, count: u32, file_ptr: *const i8) -> bool {
        if REDIRECT_ACTIVE.with(|active| active.get()) {
            let total = (size_in_bytes as usize) * (count as usize);
            let bytes = unsafe { std::slice::from_raw_parts(source_ptr as *const u8, total) };
            CAPTURE_BUFFER.with(|buffer| buffer.borrow_mut().extend_from_slice(bytes));
            return true;
        }
        unsafe { WRITE_BYTES_TO_FILE_DETOUR.call(source_ptr, size_in_bytes, count, file_ptr) }
    }

    #[detour(DEALLOCATE)]
    unsafe extern "cdecl" fn deallocate(dest_ptr: *const u32, size_in_bytes: u32, count: u32, file_ptr: *const u8) -> u32 {
        if REDIRECT_ACTIVE.with(|active| active.get()) {
            let total = (size_in_bytes as usize) * (count as usize);
            let read_ok = REPLAY_CURSOR.with(|cursor| {
                let mut cursor = cursor.borrow_mut();
                let (bytes, pos) = &mut *cursor;
                if *pos + total > bytes.len() {
                    return false;
                }
                let dest = unsafe { std::slice::from_raw_parts_mut(dest_ptr as *mut u8, total) };
                dest.copy_from_slice(&bytes[*pos..*pos + total]);
                *pos += total;
                true
            });
            return read_ok as u32;
        }
        unsafe { DEALLOCATE_DETOUR.call(dest_ptr, size_in_bytes, count, file_ptr) }
    }
}

pub fn init() {
    if let Err(e) = unsafe { detours::init_detours() } {
        tracing::error!("Failed to initialise io_redirect detours: {e:?}");
    }
}
