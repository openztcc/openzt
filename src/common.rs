use std::fmt;

use crate::debug_dll::{get_string_from_memory_bounded, save_string_to_memory};

#[derive(Debug)]
#[repr(C)]
pub struct ZTString {
    start_ptr: u32,
    end_ptr: u32,
    buffer_end_ptr: u32,
}

impl fmt::Display for ZTString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            get_string_from_memory_bounded(self.start_ptr, self.end_ptr, self.buffer_end_ptr)
        )
    }
}

impl ZTString {
    pub fn len(&self) -> usize {
        (self.end_ptr - self.start_ptr) as usize
    }

    pub fn capacity(&self) -> usize {
        (self.buffer_end_ptr - self.start_ptr) as usize
    }

    pub fn replace(&mut self, new_string: String) -> Result<(), String> {
        if new_string.len() + 1 > self.capacity() {
            Err("New string is too long".to_string())
        } else {
            let new_end_ptr = self.start_ptr + new_string.len() as u32;
            save_string_to_memory(self.start_ptr, &new_string);
            self.end_ptr = new_end_ptr;
            Ok(())
        }
    }
}

// impl ToString for ZTString {
//     fn to_string(&self) -> String {
//         get_string_from_memory_bounded(self.start_ptr, self.end_ptr, self.buffer_end_ptr)
//     }
// }

impl From<String> for ZTString {
    fn from(_s: String) -> ZTString {
        panic!("Not possible to convert from string to ZTString")
    }
}

// TODO: Fix, should take ptr? Or get ptr from ZTString?
// pub fn set_ztstring(ztstring: ZTString, new_string: String) -> Result<(), String> {
//     if new_string.len() + 1 > ztstring.capacity() {
//         Err("New string is too long".to_string())
//     } else {
//         let new_end_ptr = ztstring.start_ptr + new_string.len() as u32;
//         save_string_to_memory(ztstring.start_ptr, &new_string);
//         save_to_memory(address, value)
//         Ok(())
//     }
// }