use std::string::ToString;
use crate::debug_dll::get_string_from_memory_bounded;


#[derive(Debug)]
#[repr(C)]
pub struct ZTString {
    start_ptr: u32,
    end_ptr: u32,
    buffer_end_ptr: u32,
}

impl ToString for ZTString {
    fn to_string(&self) -> String {
        get_string_from_memory_bounded(self.start_ptr, self.end_ptr, self.buffer_end_ptr)
    }
}