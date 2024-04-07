use std::{env, path::Path};

mod bfregistry;
mod debug_dll;
mod load_ini;

fn main() {
    let zoo_directory = Path::new("../zt_files");

    env::set_current_dir(zoo_directory).unwrap();

    let debug_settings = load_ini::load_debug_settings(Path::new("zoo.ini"));

    println!("{:?}", debug_settings);

    debug_dll::debug_logger("Test");
}
