/// Example demonstrating GenericDetour usage with retour-utils
/// 
/// This example shows how to use the new `generic` flag in the #[hook] macro
/// to create GenericDetour instances instead of StaticDetour instances.
/// 
/// GenericDetour allows for runtime enabling/disabling of hooks, unlike
/// StaticDetour which is initialized once and lives for the program lifetime.

use retour_utils::hook_module;

#[hook_module("example.dll")]
mod hooks {
    // Static detour - initialized once during init_detours()
    #[hook(StaticAdd, symbol = "add")]
    fn static_add_detour(a: i32, b: i32) -> i32 {
        println!("Static hook: adding {} + {}", a, b);
        unsafe { StaticAdd.call(a, b) }
    }
    
    // Generic detour - can be enabled/disabled at runtime
    #[hook(GenericMultiply, symbol = "multiply", generic)]
    fn generic_multiply_detour(a: i32, b: i32) -> i32 {
        println!("Generic hook: multiplying {} * {}", a, b);
        unsafe { call_original_GenericMultiply(a, b) }
    }
    
    // Generic detour with visibility
    #[hook(pub GenericSubtract, symbol = "subtract", generic)]
    fn generic_subtract_detour(a: i32, b: i32) -> i32 {
        println!("Generic hook: subtracting {} - {}", a, b);
        unsafe { call_original_GenericSubtract(a, b) }
    }
}

fn main() {
    unsafe {
        // Initialize static detours only
        if let Err(e) = hooks::init_detours() {
            eprintln!("Failed to initialize static detours: {}", e);
            return;
        }
        println!("Static detours initialized successfully");
        
        // Enable generic detours manually
        if let Err(e) = hooks::enable_GenericMultiply() {
            eprintln!("Failed to enable GenericMultiply: {}", e);
        } else {
            println!("GenericMultiply enabled");
        }
        
        if let Err(e) = hooks::enable_GenericSubtract() {
            eprintln!("Failed to enable GenericSubtract: {}", e);
        } else {
            println!("GenericSubtract enabled");
        }
        
        // Later, you can disable generic detours
        hooks::disable_GenericMultiply();
        println!("GenericMultiply disabled");
        
        hooks::disable_GenericSubtract();
        println!("GenericSubtract disabled");
    }
}

/*
Key differences between StaticDetour and GenericDetour:

1. **StaticDetour** (existing behavior):
   - Initialized during `init_detours()`
   - Lives for the entire program lifetime
   - Cannot be disabled once enabled
   - Uses `detour.call()` to invoke original function

2. **GenericDetour** (new with `generic` flag):
   - Manual control via `enable_*()` and `disable_*()` functions
   - Can be enabled and disabled at runtime
   - Storage in `static mut Option<GenericDetour<...>>`
   - Uses `call_original_*()` to invoke original function
   - NOT initialized by `init_detours()`

Generated API for generic detours:
- `enable_DetourName()` - Creates and enables the detour
- `disable_DetourName()` - Disables and destroys the detour  
- `call_original_DetourName()` - Calls the original function
*/