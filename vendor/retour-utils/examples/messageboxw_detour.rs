//! A `MessageBoxW` detour example.
//!
//! Ensure the crate is compiled as a 'cdylib' library to allow C interop.
use retour_utils_impl::hook_module;
use std::error::Error;
use std::ffi::c_void;
use windows::w;
use windows::Win32::Foundation::{BOOL, HMODULE};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OKCANCEL};

#[hook_module("user32.dll")]
mod user32 {
    use windows::{core::PCWSTR, w, Win32::Foundation::HWND};

    #[hook(unsafe extern "system" MessageBoxWHook, symbol = "MessageBoxW")]
    fn messageboxw_detour(hwnd: HWND, text: PCWSTR, _caption: PCWSTR, u_type: u32) -> i32 {
        // Call the original `MessageBoxW`, but replace the caption
        let replaced_caption = w!("Nope, Detoured!");
        unsafe { MessageBoxWHook.call(hwnd, text, replaced_caption, u_type) }
    }
}

/// Called when the DLL is attached to the process.
fn main() -> Result<(), Box<dyn Error>> {
    unsafe { user32::init_detours()? };
    unsafe {
        MessageBoxW(
            None,
            w!("Everything will go as planned, right?"),
            w!("This will be replaced!"),
            MB_OKCANCEL,
        );
    }

    Ok(())
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(
    _module: HMODULE,
    call_reason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    if call_reason == DLL_PROCESS_ATTACH {
        // A console may be useful for printing to 'stdout'
        // winapi::um::consoleapi::AllocConsole();

        // Preferably a thread should be created here instead, since as few
        // operations as possible should be performed within `DllMain`.
        main().is_ok().into()
    } else {
        true.into()
    }
}
