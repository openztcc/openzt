mod commands;
//TODO: Split this mod up
mod resource_manager;
mod hooks;
mod bfresourcemgr;

use commands::init_commands;
use hooks::init_hooks;


pub use resource_manager::{add_handler, modify_ztfile_as_animation, modify_ztfile_as_ini, Handler, RunStage, OPENZT_DIR0};

pub fn init() {
    init_hooks();
    init_commands();
}
