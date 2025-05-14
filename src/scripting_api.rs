use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
};

use once_cell::sync::Lazy; //TODO: Use std::sync::LazyCell when it becomes stable
use retour_utils::hook_module;
use tracing::{error, info};

#[hook_module("zoo.exe")]
pub mod scripting_api {
    use tracing::error;

    #[hook(unsafe extern "thiscall" ZTApp_updateGame, offset = 0x0001a6d1)]
    fn zoo_zt_app_update_game(_this_ptr: u32, param_2: u32) {
        call_next_command();
        unsafe { ZTApp_updateGame.call(_this_ptr, param_2) }
    }

    pub fn init() {

    }
}

pub fn init() -> windows::core::Result<()> {
    zoo_console::init();


    // Get the handle to the new console's standard output
    // let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }.unwrap();

    // // Write to the new console
    // write_to_console(handle, "Hello from the new console!\n")?;

    // Wait for user input before closing
    // println!("Press Enter to exit...");
    // let mut input = String::new();
    // std::io::stdin().read_line(&mut input).unwrap();

    Ok(())
}
