use retour_utils::hook_module;

#[hook_module("lua52.dll")]
mod lua {
    #[allow(non_camel_case_types)]
    type lua_State = ();
    #[allow(non_camel_case_types)]
    type lua_Alloc = ();

    #[hook(unsafe extern "C" Lua_newstate, symbol = "Lua_newstate")]
    pub fn newstate(f: *mut lua_Alloc, ud: *mut std::ffi::c_void) -> *mut lua_State {
        unsafe { Lua_newstate.call(f, ud) }
    }
}

fn main() {
    unsafe { lua::init_detours().unwrap() };
}
