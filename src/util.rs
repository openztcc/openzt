#[cfg(target_os = "windows")]
use windows::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS};

use std::{mem::transmute, path::PathBuf, ptr};

use tracing::debug;

pub fn map_from_memory<T>(address: u32) -> &'static mut T {
    unsafe { transmute::<u32, &mut T>(address) }
}

pub fn get_from_memory<T>(address: u32) -> T {
    unsafe { ptr::read(address as *const T) }
}

pub fn save_to_memory<T>(address: u32, value: T) {
    unsafe { ptr::write(address as *mut T, value) };
}

pub fn save_to_protected_memory<T>(address: u32, value: T) -> anyhow::Result<()> {
    unsafe {
        {
            let mut old_protect: PAGE_PROTECTION_FLAGS = PAGE_PROTECTION_FLAGS(0);
            VirtualProtect(address as *mut _, std::mem::size_of::<T>(), PAGE_EXECUTE_READWRITE, &mut old_protect)?;
            ptr::write(address as *mut _, value);
            VirtualProtect(address as *mut _, std::mem::size_of::<T>(), old_protect, &mut old_protect)?;
        }
    }
    Ok(())
}

pub fn get_base_path() -> PathBuf {
    let mut exe_location = std::env::current_exe().unwrap();
    exe_location.pop();
    exe_location
}

pub fn get_ini_path() -> PathBuf {
    let mut exe_location = get_base_path();
    exe_location.push("zoo.ini");
    exe_location
}

pub fn patch_nop(address: u32) -> anyhow::Result<()> {
    unsafe {
        {
            let mut old_protect: PAGE_PROTECTION_FLAGS = PAGE_PROTECTION_FLAGS(0);
            VirtualProtect(address as *mut _, 1, PAGE_EXECUTE_READWRITE, &mut old_protect)?;
            ptr::write(address as *mut _, 0x90u8);
            VirtualProtect(address as *mut _, 1, old_protect, &mut old_protect)?;
        }
    }
    Ok(())
}

pub fn get_string_from_memory_with_size(address: u32, size: u32) -> String {
    get_string_from_memory_bounded(address, address + size, address + size)
}

pub fn get_string_from_memory_bounded(start: u32, end: u32, buffer_end: u32) -> String {
    let mut string = String::new();
    let mut char_address = start;
    while {
        let byte = get_from_memory::<u8>(char_address);
        byte != 0 && char_address < end && char_address < buffer_end
    } {
        string.push(get_from_memory::<u8>(char_address) as char);
        char_address += 1;
    }
    string
}

pub fn get_string_from_memory(address: u32) -> String {
    debug!("decoding string at address: {:p}", address as *const ());
    let mut string = String::new();
    let mut char_address = address;
    while {
        let byte = get_from_memory::<u8>(char_address);
        byte != 0
    } {
        string.push(get_from_memory::<u8>(char_address) as char);
        char_address += 1;
    }
    debug!("decoded: {}", string);
    string
}

pub fn save_string_to_memory(address: u32, string: &str) {
    let mut char_address = address;
    for c in string.chars() {
        save_to_memory::<u8>(char_address, c as u8);
        char_address += 1;
    }
    save_to_memory::<u8>(char_address, 0);
}