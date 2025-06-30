use retour_utils::hook_module;

#[hook_module("test.dll")]
mod test_module {
    // Test basic generic detour
    #[hook(pub GenericAdd, symbol = "add", generic)]
    fn add_detour(a: i32, b: i32) -> i32 {
        // Add 10 instead of normal addition
        unsafe { call_original_GenericAdd(a, b) + 10 }
    }
    
    // Test mix of static and generic detours
    #[hook(StaticMultiply, symbol = "multiply")]
    fn multiply_detour(a: i32, b: i32) -> i32 {
        a * b * 2
    }
    
    // Test generic detour without unsafe
    #[hook(GenericSubtract, symbol = "subtract", generic)]
    fn subtract_detour(a: i32, b: i32) -> i32 {
        unsafe { call_original_GenericSubtract(a, b) - 5 }
    }
}

fn main() {
    // Test that generated functions exist
    unsafe {
        // Enable generic detour
        let _ = test_module::enable_GenericAdd();
        
        // Disable generic detour
        test_module::disable_GenericAdd();
        
        // Init only initializes static detours
        let _ = test_module::init_detours();
    }
}