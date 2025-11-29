use std::sync::LazyLock;
use std::sync::Mutex;
use mlua::Lua;
use tracing::info;

use crate::command_console::{CommandError, add_to_command_register};

static LUA_CONTEXT: LazyLock<Mutex<Lua>> = LazyLock::new(|| Mutex::new(Lua::new()));

fn command_run_lua(args: Vec<&str>) -> Result<String, CommandError> {
    if args.is_empty() {
        return Err(Into::into("No Lua code provided".to_string()));
    }
    let lua_code = args.join(" ");
    match LUA_CONTEXT.lock().unwrap().load(&lua_code).eval::<mlua::Value>() {
        Ok(result) => Ok(format!("Lua executed successfully: {:?}", result)),
        Err(e) => Err(Into::into(format!("Lua execution error: {}", e))),
    }
}

pub fn add_function_to_lua(name: &str, func_closure: fn(&Lua) -> mlua::Function) -> Result<(), mlua::Error> {
    let lua = LUA_CONTEXT.lock().unwrap();
    let func = func_closure(&lua);
    let globals = lua.globals();
    globals.set(name, func)
}



pub fn init() {
    // use openzt_detour::gen::
    add_to_command_register("lua".to_string(), command_run_lua);
    add_function_to_lua("continue", |lua| {
        lua.create_function(|_, ()| {
            unsafe {
                info!("Clicking continue");
                openzt_detour::gen::ztui::CLICK_CONTINUE.original()();
            }
            Ok(())
        }).unwrap()
    }).expect("Failed to add 'continue' function to Lua context");
}
