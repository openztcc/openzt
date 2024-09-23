use crate::{
    command_console::{add_to_command_register, CommandError},
    resource_manager::{
        bfresourcemgr::{read_bf_resource_dir_contents_from_memory, read_bf_resource_mgr_from_memory},
        lazyresourcemap::get_file_names,
        openzt_mods::{get_location_habitat_ids, get_mod_ids},
    },
    string_registry::get_string_from_registry,
    util::get_string_from_memory,
};

pub fn init_commands() {
    add_to_command_register("list_resources".to_owned(), command_list_resources);
    add_to_command_register("get_bfresourcemgr".to_owned(), command_get_bf_resource_mgr);
    add_to_command_register("list_resource_strings".to_string(), command_list_resource_strings);
    add_to_command_register("list_openzt_resource_strings".to_string(), command_list_openzt_resource_strings);
    add_to_command_register("list_openzt_mods".to_string(), command_list_openzt_mod_ids);
    add_to_command_register("list_openzt_locations_habitats".to_string(), command_list_openzt_locations_habitats);
}

fn command_list_resource_strings(args: Vec<&str>) -> Result<String, CommandError> {
    if args.len() > 1 {
        return Err(CommandError::new("Too many arguments".to_string()));
    }
    let mut result_string = String::new();
    for resource_string in get_file_names() {
        if args.len() == 1 && !resource_string.starts_with(args[0]) {
            continue;
        }
        result_string.push_str(&format!("{}\n", resource_string));
    }
    Ok(result_string)
}

fn command_list_openzt_resource_strings(_args: Vec<&str>) -> Result<String, CommandError> {
    let mut result_string = String::new();
    for resource_string in get_file_names() {
        if resource_string.starts_with("openzt") {
            result_string.push_str(&format!("{}\n", resource_string));
        }
    }
    Ok(result_string)
}

fn command_list_resources(_args: Vec<&str>) -> Result<String, CommandError> {
    let mut result_string = String::new();
    let bf_resource_dir_contents = read_bf_resource_dir_contents_from_memory();
    for bf_resource_dir_content in bf_resource_dir_contents {
        let bf_resource_dir = bf_resource_dir_content.dir;
        result_string.push_str(&format!(
            "{} ({})\n",
            get_string_from_memory(bf_resource_dir.dir_name_string_start),
            bf_resource_dir.num_child_files
        ));
        let bf_resource_zips = bf_resource_dir_content.zips;
        for bf_resource_zip in bf_resource_zips {
            result_string.push_str(&format!("{}\n", get_string_from_memory(bf_resource_zip.zip_name_string_start)));
        }
    }
    Ok(result_string)
}

fn command_get_bf_resource_mgr(_args: Vec<&str>) -> Result<String, CommandError> {
    let bf_resource_mgr = read_bf_resource_mgr_from_memory();
    Ok(format!("{}", bf_resource_mgr))
}

fn command_list_openzt_mod_ids(_args: Vec<&str>) -> Result<String, CommandError> {
    let mut result_string = String::new();
    for mod_id in get_mod_ids() {
        result_string.push_str(&format!("{}\n", mod_id));
    }
    Ok(result_string)
}

fn command_list_openzt_locations_habitats(_args: Vec<&str>) -> Result<String, CommandError> {
    let mut result_string = String::new();
    for id in get_location_habitat_ids() {
        let name = get_string_from_registry(id).unwrap_or("<error>".to_string());
        result_string.push_str(&format!("{} {}\n", id, name));
    }
    Ok(result_string)
}
