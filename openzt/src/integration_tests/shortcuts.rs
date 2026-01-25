//! Integration tests for the keyboard shortcut registration system.

use super::TestResult;
use crate::shortcuts::{VkKey, register_shortcut, list_shortcuts};

pub fn run_all_tests() -> Vec<TestResult> {
    vec![
        test_simple_shortcut_registration(),
        test_shortcut_with_modifiers(),
        test_shortcut_conflict_detection(),
        test_shortcut_override_behavior(),
        test_multiple_shortcuts_different_keys(),
        test_list_shortcuts(),
        test_all_modifiers_combination(),
    ]
}

/// Test simple shortcut registration (no modifiers)
fn test_simple_shortcut_registration() -> TestResult {
    let test_name = "test_simple_shortcut_registration";

    // Register a simple shortcut
    let result = register_shortcut(
        "test_module",
        "Test F1 shortcut",
        VkKey::F1,
        false, false, false,
        false,
        || { /* Test callback */ }
    );

    if result.is_err() {
        return TestResult::fail(test_name, format!("Registration failed: {:?}", result));
    }

    // Verify shortcut is in list
    let list = list_shortcuts();
    if !list.contains("Test F1 shortcut") {
        return TestResult::fail(test_name, format!("Shortcut not found in list. List: {}", list));
    }

    if !list.contains("test_module") {
        return TestResult::fail(test_name, format!("Module name not found in list. List: {}", list));
    }

    TestResult::pass(test_name)
}

/// Test shortcut with Ctrl+Shift+Alt modifiers
fn test_shortcut_with_modifiers() -> TestResult {
    let test_name = "test_shortcut_with_modifiers";

    // Register shortcuts with different modifier combinations
    let ctrl_f2 = register_shortcut(
        "test_module",
        "Ctrl+F2 shortcut",
        VkKey::F2,
        true, false, false,
        false,
        || { /* Ctrl+F2 callback */ }
    );

    if ctrl_f2.is_err() {
        return TestResult::fail(test_name, format!("Ctrl+F2 registration failed: {:?}", ctrl_f2));
    }

    let ctrl_shift_f3 = register_shortcut(
        "test_module",
        "Ctrl+Shift+F3 shortcut",
        VkKey::F3,
        true, true, false,
        false,
        || { /* Ctrl+Shift+F3 callback */ }
    );

    if ctrl_shift_f3.is_err() {
        return TestResult::fail(test_name, format!("Ctrl+Shift+F3 registration failed: {:?}", ctrl_shift_f3));
    }

    let ctrl_shift_alt_f4 = register_shortcut(
        "test_module",
        "Ctrl+Shift+Alt+F4 shortcut",
        VkKey::F4,
        true, true, true,
        false,
        || { /* Ctrl+Shift+Alt+F4 callback */ }
    );

    if ctrl_shift_alt_f4.is_err() {
        return TestResult::fail(test_name, format!("Ctrl+Shift+Alt+F4 registration failed: {:?}", ctrl_shift_alt_f4));
    }

    // Verify all shortcuts are in list
    let list = list_shortcuts();
    if !list.contains("Ctrl+F2 shortcut") {
        return TestResult::fail(test_name, "Ctrl+F2 shortcut not found in list".to_string());
    }
    if !list.contains("Ctrl+Shift+F3 shortcut") {
        return TestResult::fail(test_name, "Ctrl+Shift+F3 shortcut not found in list".to_string());
    }
    if !list.contains("Ctrl+Shift+Alt+F4 shortcut") {
        return TestResult::fail(test_name, "Ctrl+Shift+Alt+F4 shortcut not found in list".to_string());
    }

    TestResult::pass(test_name)
}

/// Test that registering duplicate shortcuts fails when override=false
fn test_shortcut_conflict_detection() -> TestResult {
    let test_name = "test_shortcut_conflict_detection";

    // Register first shortcut
    let first = register_shortcut(
        "module_a",
        "First F5 shortcut",
        VkKey::F5,
        true, false, false,
        false,
        || { /* First callback */ }
    );

    if first.is_err() {
        return TestResult::fail(test_name, format!("First registration failed: {:?}", first));
    }

    // Try to register second shortcut with same key+modifiers (should fail)
    let second = register_shortcut(
        "module_b",
        "Second F5 shortcut",
        VkKey::F5,
        true, false, false,
        false,  // override = false
        || { /* Second callback */ }
    );

    if second.is_ok() {
        return TestResult::fail(test_name, "Second registration should have failed but succeeded".to_string());
    }

    // Verify error message contains expected content
    let error_msg = second.unwrap_err();
    if !error_msg.contains("already registered") && !error_msg.contains("module_a") {
        return TestResult::fail(test_name, format!("Error message unexpected: {}", error_msg));
    }

    // Verify original shortcut is still in list
    let list = list_shortcuts();
    if !list.contains("First F5 shortcut") {
        return TestResult::fail(test_name, "Original shortcut was replaced".to_string());
    }
    if list.contains("Second F5 shortcut") {
        return TestResult::fail(test_name, "Second shortcut should not be in list".to_string());
    }

    TestResult::pass(test_name)
}

/// Test that override=true allows replacing existing shortcuts
fn test_shortcut_override_behavior() -> TestResult {
    let test_name = "test_shortcut_override_behavior";

    // Register first shortcut
    let first = register_shortcut(
        "module_a",
        "Original F6 shortcut",
        VkKey::F6,
        false, true, false,
        false,
        || { /* Original callback */ }
    );

    if first.is_err() {
        return TestResult::fail(test_name, format!("First registration failed: {:?}", first));
    }

    // Register second shortcut with same key+modifiers but override=true
    let second = register_shortcut(
        "module_b",
        "Replacement F6 shortcut",
        VkKey::F6,
        false, true, false,
        true,  // override = true
        || { /* Replacement callback */ }
    );

    if second.is_err() {
        return TestResult::fail(test_name, format!("Override registration failed: {:?}", second));
    }

    // Verify replacement shortcut is in list
    let list = list_shortcuts();
    if !list.contains("Replacement F6 shortcut") {
        return TestResult::fail(test_name, "Replacement shortcut not found in list".to_string());
    }
    if list.contains("Original F6 shortcut") {
        return TestResult::fail(test_name, "Original shortcut should be replaced".to_string());
    }
    if !list.contains("module_b") {
        return TestResult::fail(test_name, "Module_b not found in list".to_string());
    }

    TestResult::pass(test_name)
}

/// Test that multiple shortcuts with different keys can coexist
fn test_multiple_shortcuts_different_keys() -> TestResult {
    let test_name = "test_multiple_shortcuts_different_keys";

    // Register multiple shortcuts with different keys
    let keys = [
        (VkKey::F7, "F7 action", false, false, false),
        (VkKey::F8, "F8 action", true, false, false),
        (VkKey::F9, "F9 action", false, true, false),
        (VkKey::F10, "F10 action", true, true, false),
        (VkKey::F11, "F11 action", false, false, true),
        (VkKey::Num1, "Num1 action", true, true, true),
        (VkKey::A, "A action", false, false, false),
    ];

    for (key, desc, ctrl, shift, alt) in &keys {
        let result = register_shortcut(
            "multi_test",
            desc,
            *key,
            *ctrl, *shift, *alt,
            false,
            || { /* Callback */ }
        );

        if result.is_err() {
            return TestResult::fail(test_name, format!("Failed to register {}: {:?}", desc, result));
        }
    }

    // Verify all shortcuts are in list
    let list = list_shortcuts();
    for (_, desc, _, _, _) in &keys {
        if !list.contains(desc) {
            return TestResult::fail(test_name, format!("Shortcut '{}' not found in list", desc));
        }
    }

    TestResult::pass(test_name)
}

/// Test the list_shortcuts function format and content
fn test_list_shortcuts() -> TestResult {
    let test_name = "test_list_shortcuts";

    // Clear registry by checking initial state
    let initial_list = list_shortcuts();

    // Register a few test shortcuts
    let _ = register_shortcut(
        "list_test_module",
        "Test list shortcut A",
        VkKey::A,
        false, false, false,
        false,
        || { /* Callback A */ }
    );

    let _ = register_shortcut(
        "list_test_module",
        "Test list shortcut B",
        VkKey::B,
        true, false, false,
        false,
        || { /* Callback B */ }
    );

    let list = list_shortcuts();

    // Check for expected content
    if !list.contains("Registered shortcuts:") && !list.contains("No shortcuts registered") {
        return TestResult::fail(test_name, format!("List format unexpected: {}", list));
    }

    if !list.contains("Test list shortcut A") {
        return TestResult::fail(test_name, format!("Shortcut A not found. List: {}", list));
    }

    if !list.contains("Test list shortcut B") {
        return TestResult::fail(test_name, format!("Shortcut B not found. List: {}", list));
    }

    if !list.contains("list_test_module") {
        return TestResult::fail(test_name, format!("Module name not found. List: {}", list));
    }

    TestResult::pass(test_name)
}

/// Test all possible modifier combinations
fn test_all_modifiers_combination() -> TestResult {
    let test_name = "test_all_modifiers_combination";

    // Test all 8 possible modifier combinations
    let combinations = [
        (false, false, false, "None"),
        (true, false, false, "Ctrl"),
        (false, true, false, "Shift"),
        (false, false, true, "Alt"),
        (true, true, false, "Ctrl+Shift"),
        (true, false, true, "Ctrl+Alt"),
        (false, true, true, "Shift+Alt"),
        (true, true, true, "Ctrl+Shift+Alt"),
    ];

    for (i, (ctrl, shift, alt, mod_name)) in combinations.iter().enumerate() {
        let key = match i {
            0 => VkKey::Num1,
            1 => VkKey::Num2,
            2 => VkKey::Num3,
            3 => VkKey::Num4,
            4 => VkKey::Num5,
            5 => VkKey::Num6,
            6 => VkKey::Num7,
            _ => VkKey::Num8,
        };

        let result = register_shortcut(
            "modifier_test",
            &format!("Test with {} modifiers", mod_name),
            key,
            *ctrl, *shift, *alt,
            false,
            || { /* Callback */ }
        );

        if result.is_err() {
            return TestResult::fail(test_name, format!("Failed to register with {}: {:?}", mod_name, result));
        }
    }

    // Verify all are in list
    let list = list_shortcuts();
    if !list.contains("Test with None modifiers") {
        return TestResult::fail(test_name, "No-modifier shortcut not found".to_string());
    }
    if !list.contains("Test with Ctrl+Shift+Alt modifiers") {
        return TestResult::fail(test_name, "All-modifier shortcut not found".to_string());
    }

    TestResult::pass(test_name)
}
