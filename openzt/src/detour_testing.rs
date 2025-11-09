use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use tracing::{info, warn};

/// Global registry for tracking detour calls during testing
static DETOUR_REGISTRY: Lazy<Arc<Mutex<DetourRegistry>>> = Lazy::new(|| {
    Arc::new(Mutex::new(DetourRegistry::new()))
});

#[derive(Debug, Clone)]
pub struct DetourCallInfo {
    pub name: String,
    pub address: u32,
    pub call_count: u32,
    pub last_called: std::time::Instant,
    pub signature_verified: bool,
}

pub struct DetourRegistry {
    calls: HashMap<String, DetourCallInfo>,
    enabled_detours: HashSet<String>,
    test_mode: bool,
}

impl DetourRegistry {
    fn new() -> Self {
        Self {
            calls: HashMap::new(),
            enabled_detours: HashSet::new(),
            test_mode: cfg!(feature = "detour-testing"),
        }
    }

    pub fn register_call(&mut self, name: &str, address: u32) {
        let entry = self.calls.entry(name.to_string()).or_insert(DetourCallInfo {
            name: name.to_string(),
            address,
            call_count: 0,
            last_called: std::time::Instant::now(),
            signature_verified: false,
        });
        
        entry.call_count += 1;
        entry.last_called = std::time::Instant::now();
        
        if self.test_mode {
            info!("DETOUR_TEST: {} called (count: {})", name, entry.call_count);
        }
    }

    pub fn mark_signature_verified(&mut self, name: &str) {
        if let Some(info) = self.calls.get_mut(name) {
            info.signature_verified = true;
        }
    }

    pub fn enable_detour(&mut self, name: &str) {
        self.enabled_detours.insert(name.to_string());
        info!("Detour enabled: {}", name);
    }

    pub fn is_detour_enabled(&self, name: &str) -> bool {
        !self.test_mode || self.enabled_detours.contains(name)
    }

    pub fn get_coverage_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Detour Coverage Report ===\n");
        
        let total_detours = self.calls.len();
        let called_detours = self.calls.values().filter(|info| info.call_count > 0).count();
        let verified_detours = self.calls.values().filter(|info| info.signature_verified).count();
        
        report.push_str(&format!("Total detours: {}\n", total_detours));
        report.push_str(&format!("Called at least once: {}\n", called_detours));
        report.push_str(&format!("Signature verified: {}\n", verified_detours));
        report.push_str(&format!("Coverage: {:.1}%\n\n", (called_detours as f32 / total_detours as f32) * 100.0));
        
        // Most frequently called
        let mut by_count: Vec<_> = self.calls.values().collect();
        by_count.sort_by(|a, b| b.call_count.cmp(&a.call_count));
        
        report.push_str("Most frequently called:\n");
        for info in by_count.iter().take(10) {
            if info.call_count > 0 {
                report.push_str(&format!("  {} - {} calls\n", info.name, info.call_count));
            }
        }
        
        // Never called
        report.push_str("\nNever called:\n");
        for info in self.calls.values() {
            if info.call_count == 0 {
                report.push_str(&format!("  {} (0x{:08x})\n", info.name, info.address));
            }
        }
        
        report
    }
}

/// Register a detour call for testing purposes
pub fn register_detour_call(name: &str, address: u32) {
    if let Ok(mut registry) = DETOUR_REGISTRY.lock() {
        registry.register_call(name, address);
    }
}

/// Check if a detour should execute its logic or just pass through
pub fn should_execute_detour_logic(name: &str) -> bool {
    if let Ok(registry) = DETOUR_REGISTRY.lock() {
        registry.is_detour_enabled(name)
    } else {
        true // Default to enabled if registry is unavailable
    }
}

/// Mark a detour's signature as verified
pub fn mark_signature_verified(name: &str) {
    if let Ok(mut registry) = DETOUR_REGISTRY.lock() {
        registry.mark_signature_verified(name);
    }
}

/// Enable a specific detour after testing
pub fn enable_detour(name: &str) {
    if let Ok(mut registry) = DETOUR_REGISTRY.lock() {
        registry.enable_detour(name);
    }
}

/// Get coverage report
pub fn get_coverage_report() -> String {
    if let Ok(registry) = DETOUR_REGISTRY.lock() {
        registry.get_coverage_report()
    } else {
        "Failed to access detour registry".to_string()
    }
}

/// Macro to wrap detour implementations with testing support
#[macro_export]
macro_rules! test_safe_detour {
    ($detour_name:expr, $address:expr, $original_call:expr, $detour_logic:expr) => {
        {
            crate::detour_testing::register_detour_call($detour_name, $address);
            
            if crate::detour_testing::should_execute_detour_logic($detour_name) {
                $detour_logic
            } else {
                // Test mode: just pass through to original
                $original_call
            }
        }
    };
}

/// Console commands for testing
pub mod console_commands {
    use super::*;
    use crate::command_console::{add_to_command_register, CommandError};
    
    pub fn init() {
        add_to_command_register("detour_coverage".to_owned(), command_detour_coverage);
        add_to_command_register("enable_detour".to_owned(), command_enable_detour);
        add_to_command_register("test_ui_systems".to_owned(), command_test_ui_systems);
        add_to_command_register("test_entity_systems".to_owned(), command_test_entity_systems);
        add_to_command_register("test_resource_systems".to_owned(), command_test_resource_systems);
    }
    
    fn command_detour_coverage(_args: Vec<&str>) -> Result<String, CommandError> {
        Ok(get_coverage_report())
    }
    
    fn command_enable_detour(args: Vec<&str>) -> Result<String, CommandError> {
        if args.len() != 1 {
            return Err("Usage: enable_detour <detour_name>".into());
        }
        
        enable_detour(args[0]);
        Ok(format!("Enabled detour: {}", args[0]))
    }
    
    fn command_test_ui_systems(_args: Vec<&str>) -> Result<String, CommandError> {
        // TODO: Implement systematic UI testing
        // This would programmatically open different UI panels, tabs, etc.
        Ok("UI systems test not yet implemented".to_string())
    }
    
    fn command_test_entity_systems(_args: Vec<&str>) -> Result<String, CommandError> {
        // TODO: Implement entity placement testing
        // This would place different animals, buildings, etc.
        Ok("Entity systems test not yet implemented".to_string())
    }
    
    fn command_test_resource_systems(_args: Vec<&str>) -> Result<String, CommandError> {
        // TODO: Implement resource loading testing
        // This would force loading of different resource types
        Ok("Resource systems test not yet implemented".to_string())
    }
}