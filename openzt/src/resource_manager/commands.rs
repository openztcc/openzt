use crate::{
    command_console::CommandError,
    lua_fn,
    resource_manager::{
        bfresourcemgr::{read_bf_resource_dir_contents_from_memory, read_bf_resource_mgr_from_memory},
        lazyresourcemap::get_file_names,
        openzt_mods::{get_location_habitat_ids, get_mod_ids},
    },
    string_registry::get_string_from_registry,
    util::ZTString,
};

pub fn init_commands() {
    // list_resources() - no args
    lua_fn!("list_resources", "Lists all BF resource directories and files", "list_resources()", || {
        match command_list_resources(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string())))
        }
    });

    // get_bfresourcemgr() - no args
    lua_fn!("get_bfresourcemgr", "Returns BF resource manager details", "get_bfresourcemgr()", || {
        match command_get_bf_resource_mgr(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string())))
        }
    });

    // list_resource_strings([prefix]) - optional string arg
    lua_fn!("list_resource_strings", "Lists resource strings, optionally filtered by prefix", "list_resource_strings([prefix])", |prefix: Option<String>| {
        match prefix {
            Some(p) => {
                match command_list_resource_strings(vec![&p]) {
                    Ok(result) => Ok((Some(result), None::<String>)),
                    Err(e) => Ok((None::<String>, Some(e.to_string())))
                }
            },
            None => {
                match command_list_resource_strings(vec![]) {
                    Ok(result) => Ok((Some(result), None::<String>)),
                    Err(e) => Ok((None::<String>, Some(e.to_string())))
                }
            }
        }
    });

    // list_openzt_resource_strings() - no args
    lua_fn!("list_openzt_resource_strings", "Lists all OpenZT resource strings", "list_openzt_resource_strings()", || {
        match command_list_openzt_resource_strings(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string())))
        }
    });

    // list_openzt_mods() - no args
    lua_fn!("list_openzt_mods", "Lists all OpenZT mod IDs", "list_openzt_mods()", || {
        match command_list_openzt_mod_ids(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string())))
        }
    });

    // list_openzt_locations_habitats() - no args
    lua_fn!("list_openzt_locations_habitats", "Lists all OpenZT location and habitat IDs", "list_openzt_locations_habitats()", || {
        match command_list_openzt_locations_habitats(vec![]) {
            Ok(result) => Ok((Some(result), None::<String>)),
            Err(e) => Ok((None::<String>, Some(e.to_string())))
        }
    });
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
            bf_resource_dir.dir_name.copy_to_string(),
            bf_resource_dir.num_child_files,
        ));
        let bf_resource_zips = bf_resource_dir_content.zips;
        for bf_resource_zip in bf_resource_zips {
            result_string.push_str(&format!("{}\n", bf_resource_zip.zip_name.copy_to_string()));
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
