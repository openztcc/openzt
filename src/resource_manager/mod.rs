mod bfresourcemgr;
mod commands;
mod handlers;
mod hooks;
mod lazyresourcemap;
mod legacy_loading;
mod openzt_mods;
mod ztd;
mod ztfile;

use commands::init_commands;
pub use handlers::{add_handler, Handler, RunStage};
use hooks::init_hooks;
pub use legacy_loading::OPENZT_DIR0;
pub use ztfile::{modify_ztfile_as_animation, modify_ztfile_as_ini};

///Initializes hooks and commands for the resource manager
pub fn init() {
    init_hooks();
    init_commands();
}
