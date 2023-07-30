#![feature(abi_thiscall)]

use configparser::ini::Ini;

use std::net::TcpStream;

use std::string;
use std::sync::Mutex;

use tracing::{info, error, Level};

use retour_utils::hook_module;

mod bfregistry;

mod capture_ztlog;

mod console;

mod ztaimgr;

mod ztworldmgr;

#[cfg(target_os = "windows")]
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH, DLL_THREAD_ATTACH, DLL_THREAD_DETACH};
use crate::bfregistry::list_registry;
use crate::debug_dll::{patch_calls, get_string_from_memory};

use crate::console::{add_to_command_register, call_command, start_server};

use crate::bfregistry::command_list_registry;

use crate::debug_dll::{command_show_settings, command_get_setting, command_set_setting};

#[cfg(not(target_os = "windows"))]
mod linux {
    const DLL_PROCESS_DETACH: u32 = 0;
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_THREAD_ATTACH: u32 = 2;
    const DLL_THREAD_DETACH: u32 = 3;
}

mod debug_dll;
mod load_ini;


#[no_mangle]
pub fn dll_first_load() {

    let stream = TcpStream::connect("127.0.0.1:1492").unwrap();

    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(stream))
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap_or_else(|error| {
        panic!("Failed to init tracing: {error}")
    });

    info!("openzt.dll Loaded");
}


#[hook_module("zoo.exe")]
mod zoo_ini {
    use tracing::info;

    use crate::debug_dll::{get_string_from_memory, get_from_memory};

    use crate::load_debug_settings_from_ini;

    use crate::cls_0x40190a;

    #[hook(unsafe extern "cdecl" LoadDebugSettingsFromIniHook, offset = 0x00179f4c)]
    fn load_debug_settings_from_ini_detour() {
        info!("Detour via macro (Debug Ini Settings)");
        // unsafe { LoadDebugSettingsFromIniHook.call() }
        load_debug_settings_from_ini();
    }

    // #[hook(unsafe extern "cdecl" LoadIniValueMaybeHook, offset = 0x0001b4bc)]
    // // fn load_ini_value_log_detour(class_ptr: u32, ini_section: LPCSTR, ini_key: LPCSTR, ini_default: LPCSTR) -> u32 {
    // fn load_ini_value_log_detour(class_ptr: u32, ini_section: u32, ini_key: u32, ini_default: u32) -> u32 {
    //     info!("Detour via macro (LoadIniValueMaybeHook)");
    //     let deref_class_ptr = get_from_memory::<u32>(class_ptr);
    //     info!("class_ptr: {:#08x}", class_ptr);
    //     info!("deref_class_ptr: {:#08x}", deref_class_ptr);

    //     let cls_0x40190a_addr: *const cls_0x40190a = class_ptr as *const _;
    //     let cls_0x40190a_obj = unsafe { &*cls_0x40190a_addr };
    //     info!("cls_0x40190a_obj: {:#?}", cls_0x40190a_obj);

    //     info!("ini_section: {}", get_string_from_memory(get_from_memory::<u32>(ini_section)));
    //     info!("ini_key: {}", get_string_from_memory(get_from_memory::<u32>(ini_key)));
    //     info!("ini_default: {}", get_string_from_memory(get_from_memory::<u32>(ini_default)));
    //     let return_value = unsafe { LoadIniValueMaybeHook.call(class_ptr, ini_section, ini_key, ini_default) };
    //     info!("return_value: {}", get_string_from_memory(get_from_memory::<u32>(return_value)));

    //     let cls_0x40190a_addr: *const cls_0x40190a = class_ptr as *const _;
    //     let cls_0x40190a_obj = unsafe { &*cls_0x40190a_addr };
    //     info!("cls_0x40190a_obj: {:#?}", cls_0x40190a_obj);

    //     return_value
    // }
}

#[hook_module("zoo.exe")]
mod zoo_bf_registry {
    use crate::debug_dll::{get_string_from_memory, get_from_memory};
    use crate::bfregistry::{get_from_registry, add_to_registry};

    #[hook(unsafe extern "thiscall" BFRegistry_prtGetHook, offset = 0x000bdd22)]
    fn prt_get(_this_prt: u32, class_name: u32, _delimeter_maybe: u8) -> u32 {
        let result = match get_from_registry(get_string_from_memory(get_from_memory::<u32>(class_name))) {
            Some(x) => x,
            None => 0x0
        };
        result
    }

    #[hook(unsafe extern "cdecl" BFRegistry_AddHook, offset = 0x001770e5)]
    fn add_to_bfregistry(param_1: u32, param_2: u32) -> u32 {
        let param_1_string = get_string_from_memory(get_from_memory::<u32>(param_1));
        add_to_registry(&param_1_string, param_2);
        0x638001
    }

    #[hook(unsafe extern "cdecl" BFRegistry_AddUIHook, offset = 0x001774bf)]
    fn add_to_bfregistry_ui(param_1: u32, param_2: u32) -> u32  {
        let param_1_string = get_string_from_memory(get_from_memory::<u32>(param_1));
        add_to_registry(&param_1_string, param_2);
        0x638001
    }

}

#[hook_module("zoo.exe")]
mod zoo_zt_world_mgr {
    use tracing::info;
    use crate::debug_dll::{get_string_from_memory, get_from_memory};
    // use crate::console::add_to_console;    

}

#[hook_module("zoo.exe")]
mod zoo_console {
    use tracing::info;
    use crate::debug_dll::{get_string_from_memory, get_from_memory};
    use crate::console::call_next_command;
    // use crate::console::add_to_console;    
    #[hook(unsafe extern "thiscall" ZTApp_updateGame, offset = 0x0001a6d1)]
    fn zoo_zt_app_update_game(_this_ptr: u32, param_2: u32) {
        call_next_command();
        unsafe { ZTApp_updateGame.call(_this_ptr, param_2) }
    }

}

#[hook_module("zoo.exe")]
mod zoo_zt_app {
    use tracing::info;
    use crate::debug_dll::get_from_memory;

    #[hook(unsafe extern "thiscall" ZTApp_updateGame, offset = 0x0001a6d1)]
    fn zoo_zt_app_update_game(_this_ptr: u32, param_2: u32) {
        info!("ZTApp({:#08x})::updateGame({})", _this_ptr, param_2);
        info!("DAT_638048 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638048), get_from_memory::<u32>(get_from_memory::<u32>(0x638048)));
        info!("DAT_638098 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638098), get_from_memory::<u32>(get_from_memory::<u32>(0x638098)));
        info!("DAT_638040 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638040), get_from_memory::<u32>(get_from_memory::<u32>(0x638040)));
        info!("DAT_638ff8 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638ff8), get_from_memory::<u32>(get_from_memory::<u32>(0x638ff8)));
        info!("DAT_639010 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x639010), get_from_memory::<u32>(get_from_memory::<u32>(0x639010)));
        info!("DAT_639000 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x639000), get_from_memory::<u32>(get_from_memory::<u32>(0x639000)));
        info!("OTHER");
        info!("DAT_638ddb -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638de0), get_from_memory::<u32>(get_from_memory::<u32>(0x638de0)));
        info!("DAT_638048 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638048), get_from_memory::<u32>(get_from_memory::<u32>(0x638048)));
        info!("DAT_638058 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638058), get_from_memory::<u32>(get_from_memory::<u32>(0x638058)));
        info!("DAT_638044 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x638044), get_from_memory::<u32>(get_from_memory::<u32>(0x638044)));
        info!("DAT_6380a8 -> {:#08x} -> {:#08x}", get_from_memory::<u32>(0x6380a8), get_from_memory::<u32>(get_from_memory::<u32>(0x6380a8)));
        unsafe { ZTApp_updateGame.call(_this_ptr, param_2) }
    }
}

#[hook_module("zoo.exe")]
mod zoo_logging {
    use tracing::info;
    use crate::debug_dll::get_string_from_memory;
    use crate::capture_ztlog::log_from_zt;

    #[hook(unsafe extern "cdecl" ZooLogging_LogHook, offset = 0x00001363)]
    fn zoo_log_func(source_file: u32, param_2: u32, param_3: u32, param_4: u8, param_5: u32, param_6: u32, log_message: u32) {
        let source_file_string = get_string_from_memory(source_file);
        let log_message_string = get_string_from_memory(log_message);
        log_from_zt(&source_file_string, param_2, param_3, &log_message_string);
    }
}

#[hook_module("zoo.exe")]
mod zoo_zt_entity_factory {
    use tracing::info;
    use crate::debug_dll::get_from_memory;
    
    #[hook(unsafe extern "thiscall" ZTEntityFactory_createEntity, offset = 0x00013a23)]
    fn zoo_zt_entity_factory_create_entity(_this_ptr: u32, param_1: i32, param_2: u32) -> u32 {
        info!("ZTEntityFactory({:#08x})::createEntity({:#08x}, {:#08x})", _this_ptr, param_1, param_2);
        let return_value = unsafe { ZTEntityFactory_createEntity.call(_this_ptr, param_1, param_2) };
        info!("return_value: {:#08x}", return_value);
        return_value
    }

    #[hook(unsafe extern "thiscall" ZTEntityFactory_createEntity2, offset = 0x00013acd)]
    fn zoo_zt_entity_factory_create_entity2(_this_ptr: u32, param_1: i32, param_2: u32) -> u32 {
        info!("ZTEntityFactory({:#08x})::createEntity2({:#08x}, {:#08x})", _this_ptr, param_1, param_2);
        let return_value  = unsafe { ZTEntityFactory_createEntity2.call(_this_ptr, param_1, param_2) };
        info!("return_value: {:#08x}", return_value);
        return_value
    }
}

#[hook_module("zoo.exe")]
mod zoo_dev_mode {
    use tracing::info;
    use crate::debug_dll::get_from_memory;

    // #[hook(unsafe extern "thiscall" ZTDevMode_print_text, offset = 0x000b189d)]
    // fn zoo_dev_mode_print_text(_this_ptr: u32, param_1: u32, param_2: u32, param_3: u32, param_4: u32) {
    //     info!("ZTDevMode({:#08x})::print_text({:#08x}, {:#08x}, {:#08x}, {:#08x})", _this_ptr, param_1, param_2, param_3, param_4);
    //     unsafe { ZTDevMode_print_text.call(_this_ptr, param_1, param_2, param_3, param_4) };
    // }

    #[hook(unsafe extern "thiscall" ZTDevMode_print_text2, offset = 0x000050f7)]
    fn zoo_dev_mode_print_text2(_this_ptr: u32, param_1: u32) -> u32 {
        if param_1 != 0 && _this_ptr != 0 && get_from_memory::<u32>(_this_ptr) != 0 && get_from_memory::<u32>(param_1) != 0 && get_from_memory::<u32>(_this_ptr) > 70190273 && get_from_memory::<u32>(_this_ptr) < 0x7FFFFFFF {
            info!("ZTDevMode({:#08x}->{:#08x}_{})::print_text2({:#08x}->{:#08x}->{:#08x})", _this_ptr, get_from_memory::<u32>(_this_ptr), get_from_memory::<u32>(_this_ptr), param_1, get_from_memory::<u32>(param_1), get_from_memory::<u32>(get_from_memory::<u32>(param_1)));
            info!("ZTDevMode({:#08x}->{:#08x}->{:#08x})::print_text2({:#08x}->{:#08x}->{:#08x})", _this_ptr, get_from_memory::<u32>(_this_ptr), get_from_memory::<u32>(get_from_memory::<u32>(_this_ptr)), param_1, get_from_memory::<u32>(param_1), get_from_memory::<u32>(get_from_memory::<u32>(param_1)));
        } else {
            info!("ZTDevMode({:#08x}->{:#08x})::print_text2({:#08x})", _this_ptr, get_from_memory::<u32>(_this_ptr), param_1);
        }
        let return_value = unsafe { ZTDevMode_print_text2.call(_this_ptr, param_1) };
        info!("return_value: {:#08x}", return_value);
        return_value
    }
}

#[hook_module("zoo.exe")]
mod zoo_string_loader {
    use tracing::info;
    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    #[hook(unsafe extern "stdcall" ZTStringLoad, offset = 0x00004e72)]
    fn zoo_string_loader(param_1: u32, param_2: u32, param_3: u32, param_4: u32) -> u32 {
        // info!("ZTStringLoad({:#08x}, {:#08x}, {:#08x}, {:#08x})", param_1, param_2, param_3, param_4);
        info!("{:#08x}", param_1);
        let return_value = unsafe { ZTStringLoad.call(param_1, param_2, param_3, param_4) };
        // info!("return_value: {:#08x}", return_value);
        if return_value != 0 {
            info!("{:#08x}: {}: {}", param_1, param_2, get_string_from_memory(param_3))
            // info!("return_value: {:#08x}->{}", return_value, get_string_from_memory(param_3));
        } //else {
            // info!("return_value: {:#08x}", return_value);
        // }
        return_value
    }

    #[hook(unsafe extern "cdecl" ZTStaticStringLoad, offset = 0x001349b0)]
    fn zoo_static_string_loader(param_1: u32) -> u32 {
        const global_dll_address: u32 = 0x0064c6e4;
        const global_buffer_base: u32 = 0x0064c6e8;
        const global_buffer_index: u32 = 0x0064c6c8;
        // info!("ZTStaticStringLoad({})", param_1);
        let return_value = unsafe { ZTStaticStringLoad.call(param_1) };
        if return_value != 0 {
            info!("Static {}: {}", param_1, get_string_from_memory(return_value))
            // info!("return_value: {:#08x}->{}", return_value, get_string_from_memory(return_value));
        } //else {
            // info!("return_value: {:#08x}", return_value);
        // }
        return_value
    }

    #[hook(unsafe extern "thiscall" ZTApp_LoadString, offset = 0x00004e0a)]
    fn zoo_app_load_string(_this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
        info!("{}", get_from_memory::<u32>(0x630908));
        info!("{:#08x} -> {:#08x}", _this_ptr, get_from_memory::<u32>(_this_ptr));
        // info!("ZTApp({:#08x})::LoadString({:#08x}, {:#08x})", _this_ptr, param_1, param_2);
        let return_value = unsafe { ZTApp_LoadString.call(_this_ptr, param_1, param_2) };
        // info!("return_value: {:#08x}", return_value);
        if return_value != 0 {
            info!("{}: {}", param_2, get_string_from_memory(param_2))
            // info!("return_value: {:#08x}->{}", return_value, get_string_from_memory(param_3));
        } //else {
            // info!("return_value: {:#08x}", return_value);
        // }
        return_value
    }

}

// #[hook_module("zoo.exe")]
// mod zoo_guest_manager {
//     #[hook(unsafe extern "thiscall" ZTGuestManager_test_1, offset = 0x000725ea)]
//     fn zoo_guest_manager_test_1(_this_ptr: u32, param_1: u32) -> u32 {
//         info!("ZTGuestManager({:#08x})::test_1({:#08x})", _this_ptr, param_1);
//         let return_value = unsafe { ZTGuestManager_test_1.call(_this_ptr) };
//         info!("return_value: {:#08x}", return_value);
//         return_value
//     }
// }

#[hook_module("zoo.exe")]
mod zoo_maintenance_worker {
    use tracing::info;
    use crate::debug_dll::{get_from_memory, get_string_from_memory};

    #[hook(unsafe extern "thiscall" ZT_maintenance_worker_test_1, offset = 0x000725ea)]
    fn zoo_maintenance_worker_test_1(_this_ptr: u32, param_1: u32) -> u32 {
        info!("ZT_maintenance_worker({:#08x})::test_1({:#08x}) -> switch {:#08x}", _this_ptr, param_1, get_from_memory::<u32>(_this_ptr + 0x174));
        let return_value = unsafe { ZT_maintenance_worker_test_1.call(_this_ptr, param_1) };
        info!("return_value: {:#08x}", return_value);
        return_value
    }
}

#[hook_module("zoo.exe")]
mod zoo_ai_mgr {
    use tracing::info;
    use crate::debug_dll::{get_from_memory, get_string_from_memory};
    use crate::ztaimgr::{read_cls_4551cb_from_memory, log_cls_4551cb};

    #[hook(unsafe extern "thiscall" ZT_ai_mgr_test_1, offset = 0x00037ccd)]
    fn zoo_ai_mgr_test_1(_this_ptr: u32, param_1: u32, param_2: u32, param_3: u32) -> u32 {
        info!("ZT_ai_mgr({:#08x})::test_1({}, {:#08x}, {:#08x})", _this_ptr, get_string_from_memory(get_from_memory::<u32>(param_1)), param_2, param_3);
        let return_value = unsafe { ZT_ai_mgr_test_1.call(_this_ptr, param_1, param_2, param_3) };
        info!("return_value: {:#08x}", return_value);
        return_value
    }

    #[hook(unsafe extern "thiscall" ZT_ai_mgr_test_2, offset = 0x00035f0a)]
    fn zoo_ai_mgr_test_2(_this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
        info!("ZT_ai_mgr({:#08x})::test_2({:#08x}, {:#08x})", _this_ptr, get_from_memory::<u32>(param_1), get_from_memory::<u32>(param_2));
        let cls_4551cb = read_cls_4551cb_from_memory(_this_ptr);
        log_cls_4551cb(&cls_4551cb);
        let return_value = unsafe { ZT_ai_mgr_test_2.call(_this_ptr, param_1, param_2) };
        info!("return_value: {:#08x}", return_value);
        return_value
    }
}

#[hook_module("zoo.exe")]
mod zoo_misc {
    use tracing::info;
    use crate::debug_dll::{get_from_memory, get_string_from_memory, save_to_memory, save_string_to_memory};
    use crate::load_ini::load_from_zoo_ini;

    // #[hook(unsafe extern "thiscall" ZT_startup_test_1, offset = 0x0001a8bc)]
    // fn zoo_startup_test_1(param_1: u32, param_2: u32, param_3: u32) -> u32 {
    //     info!("ZT_startup_test_1({:#08x} {:#08x} {:#08x})", param_1, param_2, param_3);
    //     let return_value = unsafe { ZT_startup_test_1.call(param_1, param_2, param_3) };
    //     info!("return_value: {:#08x}", return_value);
    //     return_value
    // }

    // #[hook(unsafe extern "thiscall" ZT_startup_test_2, offset = 0x0017e4b7)]
    // fn zoo_startup_test_2(param_1: u32) -> u32 {
    //     info!("ZT_startup_test_2({:#08x})", param_1);
    //     let return_value = unsafe { ZT_startup_test_2.call(param_1) };
    //     info!("return_value: {:#08x}", return_value);
    //     return_value
    // }

    // #[hook(unsafe extern "stdcall" ZT_get_private_profile_string, offset = 0x0001b4bc)]
    // fn zoo_get_private_profile_string(param_1_ptr: u32, section_ptr: u32, key_ptr: u32, default_ptr: u32) -> u32 {
    //     let param_1 = get_from_memory::<u32>(param_1_ptr);
    //     let section = get_string_from_memory(get_from_memory::<u32>(section_ptr));
    //     let key = get_string_from_memory(get_from_memory::<u32>(key_ptr));
    //     let default = get_string_from_memory(get_from_memory::<u32>(default_ptr));
    //     let result = load_from_zoo_ini::<String>(&section, &key, default);
    //     info!("ZT_get_private_profile_string({:#08x} {} {} {}) -> {}", param_1, section, key, default, result);
    //     let return_value = unsafe { ZT_get_private_profile_string.call(param_1_ptr, section_ptr, key_ptr, default_ptr) };
    //     info!("return_value: {:#08x}", get_from_memory::<u32>(return_value));
    //     return_value
    // }

    #[hook(unsafe extern "cdecl" ZT_get_private_profile_int, offset = 0x0001b55d)]
    fn zoo_get_private_profile_int(section_ptr: u32, key_ptr: u32, default: i32) -> i32 {
        let section = get_string_from_memory(get_from_memory::<u32>(section_ptr));
        let key = get_string_from_memory(get_from_memory::<u32>(key_ptr));
        let result = load_from_zoo_ini::<i32>(&section, &key, default);
        // info!("ZT_get_private_profile_int({}, {}, {}) => {}", section, key, default, result);
        result
    }

    #[hook(unsafe extern "cdecl" ZT_get_private_profile_f32, offset = 0x0012211d)]
    fn zoo_get_private_profile_f32(section_ptr: u32, key_ptr: u32, default: f32) -> f32 {
        let section = get_string_from_memory(get_from_memory::<u32>(section_ptr));
        let key = get_string_from_memory(get_from_memory::<u32>(key_ptr));
        let result = load_from_zoo_ini::<f32>(&section, &key, default);
        // info!("ZT_get_private_profile_f32({}, {}, {}) => {}", section, key, default, result);
        result
    }

    // #[hook(unsafe extern "cdecl" ZT_get_private_profile_string, offset = 0x0001b4bc)]
    // fn zoo_get_private_profile_string(string_buffer_ptr: u32, section_ptr: u32, key_ptr: u32, default: u32) -> u32 {
    //     let section = get_string_from_memory(get_from_memory::<u32>(section_ptr));
    //     let key = get_string_from_memory(get_from_memory::<u32>(key_ptr));
    //     let default = get_string_from_memory(get_from_memory::<u32>(default));
    //     let result = load_from_zoo_ini::<String>(&section, &key, default.clone());
    //     info!("ZT_get_private_profile_int({}, {}, {}) => {}", section, key, default, result);
    //     //Sneaky string buffer call
    //     // void __thiscall OOAnalyzer::AI_cls_0x40190a::meth_0x401d2d(AI_cls_0x40190a *this,uint *param_1)
    //     let ptr = 0x00401d2d as *const ();
    //     let code: extern "thiscall" fn(u32, u32) -> u32 = unsafe { std::mem::transmute(ptr) };
    //     // let return_value = (code)(0x63e500, len(&result) as u32);
    //     let return_value = (code)(string_buffer_ptr, result.len() as u32 + 1);
    //     info!("return_value: {:#08x}", return_value);
    //     info!("return_value: {:#08x}", get_from_memory::<u32>(string_buffer_ptr));
    //     save_string_to_memory(get_from_memory::<u32>(string_buffer_ptr), result.as_str());
    //     save_string_to_memory(0x63e500, result.as_str());
    //     string_buffer_ptr
    // }
    #[hook(unsafe extern "thiscall" ZT_BFUIMgr_displayHelp, offset = 0x0001b100)]
    fn zoo_bfuimgr_display_help(_this_ptr: u32, param_1: u32, param_2: u8) {
        info!("ZT_BFUIMgr({:#08x})::displayHelp({:#08x}, {:#08x})", _this_ptr, param_1, param_2);
        info!("{:#08x} -> {:#08x}: {:#08x}", _this_ptr, get_from_memory::<u32>(_this_ptr), get_from_memory::<u32>(get_from_memory::<u32>(_this_ptr) + 0x1c));
        // unsafe { ZT_BFUIMgr_displayHelp.call(_this_ptr, param_1, param_2) };
        unsafe { ZT_BFUIMgr_displayHelp.call(_this_ptr, 0x001bd6, param_2) };
    }

}

#[hook_module("zoo.exe")]
mod zoo_strings {
    use tracing::info;
    use crate::debug_dll::{get_from_memory, get_string_from_memory, save_to_memory, save_string_to_memory};

    // #[hook(unsafe extern "thiscall" ZTStringLoad, offset = 0x00001a94)]
    // fn zoo_string_loader(param_1: u32, param_2: u32, param_3: u32) {
    //     info!("ZTStringLoad({:#08x}, {:#08x}, {:#08x})", param_1, param_2, param_3);
    //     let param_1_deref = get_from_memory::<u32>(param_1);
    //     let string_2 = get_string_from_memory(param_2);
    //     info!("param_1: {} string_2: {}", param_1_deref, string_2);
    //     unsafe { ZTStringLoad.call(param_1, param_2, param_3) };
    //     let param_1_deref = get_from_memory::<u32>(param_1);
    //     info!("param_1: {:#08x} -> {}", param_1_deref, get_string_from_memory(param_1_deref));
    // }

    #[hook(unsafe extern "thiscall" ZTStringLoad2, offset = 0x00004c52)]
    fn zoo_string_2(param_1: u32, param_2: u32, param_3: u32, param_4: u32) -> u32 {
        info!("ZTStringLoad2({:#08x}, {:#08x}, {:#08x}, {:#08x})", param_1, param_2, param_3, param_4);
        let param_1_deref = get_from_memory::<i32>(param_1);
        let string_2 = get_string_from_memory(param_2);
        info!("param_1: {:#08x} string_2: {}", param_1_deref, string_2);
        let return_value = unsafe { ZTStringLoad2.call(param_1, param_2, param_3, param_4) };
        info!("return_value: {:#08x} -> {:#08x} -> {}", return_value, get_from_memory::<u32>(return_value), get_string_from_memory(get_from_memory::<u32>(return_value)));
        info!("param_1: {:#08x} -> {}", param_1_deref, get_string_from_memory(param_1_deref as u32));
        return_value
    }

    // #[hook(unsafe extern "thiscall" ZTStringLoad2, offset = 0x00004c52)]
    // fn zoo_string_2(param_1: u32, param_2: u32, param_3: u32) -> u32 {
    //     info!("ZTStringLoad2({:#08x}, {:#08x}, {:#08x})", param_1, param_2, param_3);
    //     let return_value = unsafe { ZTStringLoad2.call(param_1, param_2, param_3) };
    //     // info!("return_value: {:#08x}", return_value);
    //     return_value
    // }
}

#[no_mangle]
extern "system" fn DllMain(module: u8, reason: u32, _reserved: u8) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            dll_first_load();
            info!("DllMain: DLL_PROCESS_ATTACH: {}, {} {}", module, reason, _reserved);

            let addresses: Vec<u32> = vec![0x0061b7a2,0x0061b88b,0x0061b90b];

            patch_calls(addresses, load_string as u32);

            unsafe { 
                if cfg!(feature = "ini") {
                    info!("Feature ini enabled");
                    zoo_ini::init_detours().unwrap();
                    add_to_command_register("show_settings".to_owned(), command_show_settings);
                    add_to_command_register("get_setting".to_owned(), command_get_setting);
                    add_to_command_register("set_setting".to_owned(), command_set_setting);
                }
                if cfg!(feature = "bf_registry") {
                    info!("Feature bf_registry enabled");
                    zoo_bf_registry::init_detours().unwrap();
                    add_to_command_register("list_bf_registry".to_owned(), command_list_registry)
                }
                if cfg!(feature = "zt_world_mgr") {
                    use ztworldmgr::command_get_zt_world_mgr_entities;

                    info!("Feature zt_world_mgr enabled");
                    zoo_zt_world_mgr::init_detours().unwrap();
                    add_to_command_register("get_entities".to_owned(), command_get_zt_world_mgr_entities)
                }
                if cfg!(feature = "zoo_logging") {
                    info!("Feature zoo_logging enabled");
                    zoo_logging::init_detours().unwrap();
                }
                if cfg!(feature = "zt_app") {
                    info!("Feature zoo_zt_app enabled");
                    zoo_zt_app::init_detours().unwrap();
                }
                if cfg!(feature = "zt_entity_factory") {
                    info!("Feature zoo_zt_entity_factory enabled");
                    zoo_zt_entity_factory::init_detours().unwrap();
                }
                if cfg!(feature = "dev_mode") {
                    info!("Feature zoo_dev_mode enabled");
                    zoo_dev_mode::init_detours().unwrap();
                }

                if cfg!(feature = "zoo_string_loader") {
                    info!("Feature zoo_string_loader enabled");
                    zoo_string_loader::init_detours().unwrap();
                }

                if cfg!(feature = "zoo_maintenance_worker") {
                    info!("Feature zoo_maintenance_worker enabled");
                    zoo_maintenance_worker::init_detours().unwrap();
                }
                if cfg!(feature = "zt_ai_mgr") {
                    info!("Feature zt_ai_mgr enabled");
                    zoo_ai_mgr::init_detours().unwrap();
                }

                if cfg!(feature = "zoo_misc") {
                    info!("Feature zoo_misc enabled");
                    zoo_misc::init_detours().unwrap();
                }

                if cfg!(feature = "zoo_strings") {
                    info!("Feature zoo_strings enabled");
                    zoo_strings::init_detours().unwrap();
                }

                if cfg!(feature = "console") {
                    info!("Feature console enabled");
                    zoo_console::init_detours().unwrap();
                    std::thread::spawn(|| {
                        start_server();
                    });
                }
            }
        }
        DLL_PROCESS_DETACH => {
            info!("DllMain: DLL_PROCESS_DETACH: {}, {} {}", module, reason, _reserved);
        }
        DLL_THREAD_ATTACH => {
            info!("DllMain: DLL_THREAD_ATTACH: {}, {} {}", module, reason, _reserved);
        }
        DLL_THREAD_DETACH => {
            info!("DllMain: DLL_THREAD_DETACH: {}, {} {}", module, reason, _reserved);
        }
        _ => {
            info!("DllMain: Unknown: {}, {} {}", module, reason, _reserved);
        }
    }
    1
}


#[no_mangle]
extern "C" fn dll_ini_debug_log() {
    debug_dll::log_debug_ini_memory_values();
}

fn load_debug_settings_from_ini() {
    debug_dll::debug_logger("load_debug_settings_from_ini");
    debug_dll::log_exe_location_memory_value();
    debug_dll::log_debug_ini_memory_values();
    let mut base_path = debug_dll::get_base_path();
    base_path.push("zoo.ini");
    let debug_settings = load_ini::load_debug_settings(base_path.as_path());
    debug_dll::debug_logger("Saving debug ini settings");
    debug_dll::save_debug_settings(debug_settings);
    debug_dll::log_debug_ini_memory_values();
}

#[no_mangle]
pub fn patch_load_debug_ini_call() {
    debug_dll::debug_logger(&format!("load_debug_settings_from_ini {:p}", load_debug_settings_from_ini as *const ()));
    debug_dll::debug_logger(&format!("load_debug_settings_from_ini (u32) {}", load_debug_settings_from_ini as u32));
    debug_dll::get_code_from_memory(debug_dll::DEBUG_INI_LOAD_CALL_ADDRESS, 0x10);
    debug_dll::patch_call(debug_dll::DEBUG_INI_LOAD_CALL_ADDRESS, load_debug_settings_from_ini as u32);
}

#[no_mangle]
extern "C" fn patch_load_int_from_ini_call() {
    debug_dll::debug_logger(&format!("load_int_from_ini {:p}", load_int_from_ini as *const ()));
    debug_dll::patch_calls(debug_dll::LOAD_INT_FROM_INI_ADDRESS_ARRAY_SUBSET.to_vec(), load_int_from_ini as u32);
    debug_dll::patch_nops_series(debug_dll::LOAD_INT_FROM_INI_ADDRESS_ARRAY_SUBSET_NOP.to_vec());
}

#[no_mangle]
extern "C" fn patch_load_value_from_ini_call() {
    debug_dll::debug_logger(&format!("load_value_from_ini {:p}", load_value_from_ini as *const ()));
    debug_dll::patch_calls(debug_dll::LOAD_VALUE_FROM_INI_ADDRESS_ARRAY.to_vec(), load_value_from_ini as u32);
}


#[no_mangle]
extern "cdecl" fn load_int_from_ini(section_address: &u32, header_address: &u32, default: i32) -> u32 {
    debug_dll::debug_logger(&format!("load_int_from_ini {:p} {:p} default: {}", *section_address as *const (), *header_address as *const (), default));
    let section = debug_dll::get_string_from_memory(*section_address);
    let header = debug_dll::get_string_from_memory(*header_address);
    let mut zoo_ini = Ini::new();
    zoo_ini.load(get_ini_path()).unwrap();
    let result = load_ini::load_int_with_default(&zoo_ini, &section, &header, default) as u32;
    debug_dll::debug_logger(&format!("load_int_from_ini {} {} result: {}", section, header, result));
    return result;
}

#[no_mangle]
extern "cdecl" fn load_value_from_ini<'a>(result_address: &'a u32, section_address: &u32, header_address: &u32, default_address: &u32) -> &'a u32{
    debug_dll::debug_logger(&format!("load_value_from_ini {:p} {:p} default: {:p}", *section_address as *const (), *header_address as *const (), *default_address as *const ()));
    let section = debug_dll::get_string_from_memory(*section_address);
    let header = debug_dll::get_string_from_memory(*header_address);
    let default = debug_dll::get_string_from_memory(*default_address);
    let mut zoo_ini = Ini::new();
    zoo_ini.load(get_ini_path()).unwrap();
    let result = load_ini::load_string_with_default(&zoo_ini, &section, &header, &default);

    debug_dll::debug_logger(&format!("load_value_from_ini {} {} result: {}", section, header, result));
    debug_dll::debug_logger(&format!("encoding string at address: {:p}", *result_address as *const ()));
    debug_dll::save_string_to_memory(*(&result_address as &u32), &result);
    // ptr::write(result_address as *mut _, result);
    return result_address;
}

fn get_ini_path() -> String {
    let mut base_path = debug_dll::get_base_path();
    base_path.push("zoo.ini");
    base_path.to_str().unwrap().to_string()
}

#[no_mangle]
extern "thiscall" fn load_string(_this_ptr: u32, param_1: u32, param_2: u32) -> u32 {
    let ptr = 0x00404e0a as *const ();
    let code: extern "thiscall" fn(u32, u32, u32) -> u32 = unsafe { std::mem::transmute(ptr) };
    let return_value = (code)(_this_ptr, param_1, param_2);

    if return_value != 0 {
        info!("{}: {}", param_1, get_string_from_memory(param_2))
    }
    return_value
}

#[derive(Debug)]
#[repr(C)]
struct cls_0x40190a {
    mbr_1: u32,
    mbr_2: u32,
    mbr_3: u32,                 // cls_0x4012a6
    mbr_4: u32,
    // mbr_5: u32,
    // mbr_6: u32,                 // cls_0x4012a6
    // mbr_7: u32,
    // mbr_8: u32,
    // mbr_9: u32,                 // cls_0x4012a6
    // mbr_10: u32,
    // mbr_11: u32
}