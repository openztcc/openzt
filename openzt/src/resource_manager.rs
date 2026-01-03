mod bfresourcemgr;
mod commands;
mod handlers;
mod hooks;
pub(crate) mod lazyresourcemap;
mod legacy_loading;
pub(crate) mod openzt_mods;
mod ztd;
pub(crate) mod ztfile;

// Export for implementation tests
#[cfg(feature = "implementation-tests")]
pub mod dependency_resolver;
#[cfg(feature = "implementation-tests")]
pub mod mod_config;
#[cfg(feature = "implementation-tests")]
pub mod validation;

// Private modules when not testing
#[cfg(not(feature = "implementation-tests"))]
mod dependency_resolver;
#[cfg(not(feature = "implementation-tests"))]
mod mod_config;
#[cfg(not(feature = "implementation-tests"))]
mod validation;

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
