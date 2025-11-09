// Example of how to integrate test macros into detour wrappers
// This file shows the pattern for adding test support to generated detours

use retour::static_detour;

// Example 1: Simple detour with return value
static_detour! {
    static BFAPP_GET_INSTANCE_DETOUR: unsafe extern "thiscall" fn(u32) -> u32;
}

#[cfg(feature = "detour-testing")]
unsafe extern "thiscall" fn bfapp_get_instance_test(this: u32) -> u32 {
    // Use test_detour! macro for functions with return values
    crate::test_detour!(
        "BFAPP_GET_INSTANCE",
        BFAPP_GET_INSTANCE_DETOUR.call(this)
    )
}

// Example 2: Detour with void return
static_detour! {
    static BFAPP_INIT_WINDOW_CLASS_DETOUR: unsafe extern "thiscall" fn(u32);
}

#[cfg(feature = "detour-testing")]
unsafe extern "thiscall" fn bfapp_init_window_class_test(this: u32) {
    // Use test_detour_void! macro for void functions
    crate::test_detour_void!(
        "BFAPP_INIT_WINDOW_CLASS",
        BFAPP_INIT_WINDOW_CLASS_DETOUR.call(this)
    );
}

// Example 3: Multiple parameter detour
static_detour! {
    static BFMAP_TILE_TO_WORLD_DETOUR: unsafe extern "thiscall" fn(u32, u32, u32, u32) -> u32;
}

#[cfg(feature = "detour-testing")]
unsafe extern "thiscall" fn bfmap_tile_to_world_test(this: u32, x: u32, y: u32, z: u32) -> u32 {
    crate::test_detour!(
        "BFMAP_TILE_TO_WORLD",
        BFMAP_TILE_TO_WORLD_DETOUR.call(this, x, y, z)
    )
}

// Example initialization function to set up test detours
#[cfg(feature = "detour-testing")]
pub fn initialize_test_detours() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // Only initialize the detour that's enabled for this test run
        if crate::test::is_detour_enabled("BFAPP_GET_INSTANCE") {
            let detour = crate::gen::bfapp::GET_INSTANCE
                .detour(bfapp_get_instance_test)?;
            detour.enable()?;
            std::mem::forget(detour); // Keep detour active
        }
        
        if crate::test::is_detour_enabled("BFAPP_INIT_WINDOW_CLASS") {
            let detour = crate::gen::bfapp::INIT_WINDOW_CLASS
                .detour(bfapp_init_window_class_test)?;
            detour.enable()?;
            std::mem::forget(detour);
        }
        
        if crate::test::is_detour_enabled("BFMAP_TILE_TO_WORLD") {
            let detour = crate::gen::bfmap::TILE_TO_WORLD
                .detour(bfmap_tile_to_world_test)?;
            detour.enable()?;
            std::mem::forget(detour);
        }
        
        // Add more detours as needed...
    }
    
    Ok(())
}

// Alternative approach: Batch registration with a macro
#[cfg(feature = "detour-testing")]
macro_rules! register_test_detour {
    ($name:expr, $def:expr, $wrapper:expr) => {
        if crate::test::is_detour_enabled($name) {
            unsafe {
                let detour = $def.detour($wrapper)?;
                detour.enable()?;
                std::mem::forget(detour);
            }
        }
    };
}

#[cfg(feature = "detour-testing")]
pub fn initialize_all_test_detours() -> Result<(), Box<dyn std::error::Error>> {
    use crate::gen;
    
    // Register all test detours using the macro
    register_test_detour!("BFAPP_GET_INSTANCE", gen::bfapp::GET_INSTANCE, bfapp_get_instance_test);
    register_test_detour!("BFAPP_INIT_WINDOW_CLASS", gen::bfapp::INIT_WINDOW_CLASS, bfapp_init_window_class_test);
    register_test_detour!("BFMAP_TILE_TO_WORLD", gen::bfmap::TILE_TO_WORLD, bfmap_tile_to_world_test);
    
    // Continue for all detours...
    
    Ok(())
}