use crate::debug_dll::{get_from_memory, get_string_from_memory};
use crate::add_to_command_register;

use tracing::info;
use std::fmt;
use std::fmt::Display;

const EXPANSION_LIST_START: u32 = 0x00639030;
const EXPANSION_SIZE: u32 = 0x14;


#[derive(Debug)]
#[repr(C)]
struct ExpansionList {
    array_start: u32,
    array_end: u32,
    buffer_end: u32,
}

#[derive(Debug)]
#[repr(C)]
struct Expansion {
    expansion_id: u32,
    name_id: u32,
    name_string_start_ptr: u32,
    name_string_end_ptr: u32,
    name_string_buffer_end_ptr: u32,
}

fn read_expansion_list_from_memory() -> ExpansionList {
    get_from_memory(EXPANSION_LIST_START)
}

fn read_expansion_from_memory(address: u32) -> Expansion {
    get_from_memory(address)
}

fn read_expansions_from_memory() -> Vec<Expansion> {
    let expansion_list = read_expansion_list_from_memory();
    info!("Reading expansions from {:#x} to {:#x}, len {}", expansion_list.array_start, expansion_list.array_end, (expansion_list.array_end - expansion_list.array_start) / EXPANSION_SIZE);
    let mut expansions = Vec::new();
    let mut current_expansion_address = expansion_list.array_start;
    while current_expansion_address < expansion_list.array_end {
        expansions.push(read_expansion_from_memory(current_expansion_address));
        current_expansion_address += EXPANSION_SIZE;
    }
    expansions

}

impl Display for Expansion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Expansion {{ expansion_id: {:#x} name_id: {:#x} name_string: {} }}", self.expansion_id, self.name_id, get_string_from_memory(self.name_string_start_ptr))
    }
}

impl Display for ExpansionList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ExpansionList {{ array_start: {:#x} array_end: {:#x} buffer_end: {:#x} }}", self.array_start, self.array_end, self.buffer_end)
    }
}

fn command_get_expansions(_args: Vec<&str>) -> Result<String, &'static str> {
    let mut string_array = Vec::new();
    for expansion in read_expansions_from_memory() {
        string_array.push(expansion.to_string());
    }

    Ok(string_array.join("\n"))
}

pub fn init() {
    add_to_command_register("list_expansion".to_string(), command_get_expansions);
}