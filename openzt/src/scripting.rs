use std::sync::LazyLock;
use std::sync::Mutex;
use mlua::Lua;
use tracing::info;

static LUA_CONTEXT: LazyLock<Mutex<Lua>> = LazyLock::new(|| Mutex::new(Lua::new()));

// Metadata for help() function
static LUA_FUNCTION_METADATA: LazyLock<Mutex<Vec<LuaFunctionMeta>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

struct LuaFunctionMeta {
    name: String,
    description: String,
    signature: String,
}

/// Registers a Lua function with metadata for help()
pub fn add_lua_function(
    name: &str,
    description: &str,
    signature: &str,
    func_closure: fn(&Lua) -> mlua::Function
) -> Result<(), mlua::Error> {
    // Add metadata
    LUA_FUNCTION_METADATA.lock().unwrap().push(LuaFunctionMeta {
        name: name.to_string(),
        description: description.to_string(),
        signature: signature.to_string(),
    });

    // Register function in Lua global scope
    let lua = LUA_CONTEXT.lock().unwrap();
    let func = func_closure(&lua);
    let globals = lua.globals();
    globals.set(name, func)
}

/// Executes Lua code and returns the result as a string
pub fn execute_lua(code: &str) -> Result<String, String> {
    let lua = LUA_CONTEXT.lock().unwrap();
    match lua.load(code).eval::<mlua::Value>() {
        Ok(value) => Ok(lua_value_to_string(&value)),
        Err(e) => Err(format!("Lua error: {}", e))
    }
}

/// Converts a Lua value to a string representation
fn lua_value_to_string(value: &mlua::Value) -> String {
    match value {
        mlua::Value::Nil => "nil".to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_else(|_| "<invalid utf8>".to_string()),
        mlua::Value::Table(t) => {
            // Simple table representation
            let mut result = String::from("{");
            let mut first = true;
            for pair in t.pairs::<mlua::Value, mlua::Value>() {
                if let Ok((k, v)) = pair {
                    if !first {
                        result.push_str(", ");
                    }
                    first = false;
                    result.push_str(&format!("{} = {}", lua_value_to_string(&k), lua_value_to_string(&v)));
                }
            }
            result.push('}');
            result
        },
        mlua::Value::Function(_) => "<function>".to_string(),
        mlua::Value::Thread(_) => "<thread>".to_string(),
        mlua::Value::UserData(_) => "<userdata>".to_string(),
        mlua::Value::LightUserData(_) => "<lightuserdata>".to_string(),
        mlua::Value::Error(e) => format!("<error: {}>", e),
        _ => "<unknown>".to_string(),
    }
}

pub fn init() {
    info!("Initializing Lua scripting");

    // Register the continue() function
    add_lua_function(
        "continue",
        "Clicks the continue button",
        "continue()",
        |lua| {
            lua.create_function(|_, ()| {
                unsafe {
                    info!("Clicking continue");
                    openzt_detour::gen::ztui::CLICK_CONTINUE.original()();
                }
                Ok(())
            }).unwrap()
        }
    ).expect("Failed to add 'continue' function to Lua context");

    // Register the help() function
    add_lua_function(
        "help",
        "Lists available Lua functions or searches by keyword",
        "help([search_term])",
        |lua| {
            lua.create_function(|_, search: Option<String>| {
                let metadata = LUA_FUNCTION_METADATA.lock().unwrap();
                let filtered: Vec<&LuaFunctionMeta> = match &search {
                    Some(term) => metadata.iter()
                        .filter(|m| m.name.contains(term.as_str()) || m.description.to_lowercase().contains(&term.to_lowercase()))
                        .collect(),
                    None => metadata.iter().collect()
                };

                if filtered.is_empty() {
                    if let Some(term) = search {
                        return Ok(format!("No functions found matching '{}'", term));
                    } else {
                        return Ok("No functions registered".to_string());
                    }
                }

                let mut result = String::new();
                for meta in filtered {
                    result.push_str(&format!("{} - {}\n  Usage: {}\n\n",
                        meta.name, meta.description, meta.signature));
                }
                Ok(result)
            }).unwrap()
        }
    ).expect("Failed to add 'help' function to Lua context");
}
