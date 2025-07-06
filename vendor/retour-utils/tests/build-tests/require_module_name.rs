use retour_utils::hook_module;

#[hook_module]
mod lua {
    #[hook(DtLuaLoad, offset = 0x1234)]
    fn lua_load() {

    }
}
// needed for trybuild
fn main() {}