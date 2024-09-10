mod commands;
mod hooks;
mod bfresourcemgr;
mod lazyresourcemap;
mod ztfile;
mod handlers;
mod openzt_mods;
mod legacy_loading;
mod ztd;

use commands::init_commands;
use hooks::init_hooks;

pub use legacy_loading::OPENZT_DIR0;
pub use ztfile::{modify_ztfile_as_animation, modify_ztfile_as_ini};
pub use handlers::{add_handler, Handler, RunStage};

pub fn init() {
    init_hooks();
    init_commands();
}
