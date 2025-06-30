use retour_utils::hook_module;


#[hook_module("foo.bar")]
mod hooks {
    #[hook(NormalHook, symbol = "Foo")]
    fn normal_hook() -> i32 {
        NormalHook.call()
    }

    #[hook(unsafe UnsafeHook, symbol = "Foo")]
    fn unsafe_hook() -> i32 {
        unsafe {
            UnsafeHook.call()
        }
    }

    #[hook(extern "cdecl" CDeclAbiHook, symbol = "Foo")]
    fn cdecl_abi_hook() -> i32 {
        CDeclAbiHook.call()
    }

    #[hook(extern "stdcall" StdCallAbiHook, symbol = "Foo")]
    fn stdcall_abi_hook() -> i32 {
        StdCallAbiHook.call()
    }

    #[hook(extern "fastcall" FastCallAbiHook, symbol = "Foo")]
    fn fastcall_abi_hook() -> i32 {
        FastCallAbiHook.call()
    }

    #[hook(extern "C" CAbiHook, symbol = "Foo")]
    fn c_abi_hook() -> i32 {
        CAbiHook.call()
    }

    #[hook(extern "system" SystemAbiHook, symbol = "Foo")]
    fn system_abi_hook() -> i32 {
        SystemAbiHook.call()
    }


    // #[hook(extern "thiscall" ThisCallHook, symbol = "Foo")]
    // fn thiscall_abi_hook() -> i32 {
    //     ThisCallHook.call()
    // }

}
#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
#[hook_module("foo.bar")]
mod win64 {
    #[hook(extern "win64" Win64AbiHook, symbol = "Foo")]
    fn win64_abi_hook() -> i32 {
        Win64AbiHook.call()
    }
}

fn main() {}