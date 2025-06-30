

mod baz {
    use retour_utils::hook_module;
    #[hook_module("foo.dll")]
    pub mod pub_scope {
        #[hook(DtLuaLoad, offset = 0x1234)]
        fn lua_load() {

        }
    }

    mod other_scope {
        pub unsafe fn bar() {
            super::pub_scope::init_detours().unwrap()
        }
    }
}

// needed for trybuild
fn main() {
    use baz::pub_scope::init_detours;
}