use super::TestResult;
use crate::resource_manager::{
    dependency_resolver::{DependencyResolver, ResolutionWarning},
    validation::validate_load_order,
};
use std::collections::HashMap;

// ============================================================================
// Embedded Test Resources for Dependency Tests
// ============================================================================

#[cfg(feature = "implementation-tests")]
mod embedded_resources {
    // Test Mod A - No dependencies (base mod)
    pub const META_MOD_A: &str = r#"
name = "Test Mod A"
description = "Base test mod with no dependencies"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_a"
version = "1.0.0"
ztd_type = "openzt"
"#;

    // Test Mod B - Depends on A (after)
    pub const META_MOD_B: &str = r#"
name = "Test Mod B"
description = "Test mod that depends on mod A"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_b"
version = "1.0.0"
ztd_type = "openzt"

dependencies = [
    { mod_id = "test.dependency.mod_a", name = "Test Mod A", ordering = "after" }
]
"#;

    // Test Mod C - Depends on B (after), creating chain A -> B -> C
    pub const META_MOD_C: &str = r#"
name = "Test Mod C"
description = "Test mod that depends on mod B"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_c"
version = "1.0.0"
ztd_type = "openzt"

dependencies = [
    { mod_id = "test.dependency.mod_b", name = "Test Mod B", ordering = "after" }
]
"#;

    // Test Mod D - Circular dependency with E
    pub const META_MOD_D: &str = r#"
name = "Test Mod D"
description = "Test mod with circular dependency"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_d"
version = "1.0.0"
ztd_type = "openzt"

dependencies = [
    { mod_id = "test.dependency.mod_e", name = "Test Mod E", ordering = "after" }
]
"#;

    // Test Mod E - Circular dependency with D
    pub const META_MOD_E: &str = r#"
name = "Test Mod E"
description = "Test mod with circular dependency"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_e"
version = "1.0.0"
ztd_type = "openzt"

dependencies = [
    { mod_id = "test.dependency.mod_d", name = "Test Mod D", ordering = "after" }
]
"#;

    // Test Mod F - Optional dependency
    pub const META_MOD_F: &str = r#"
name = "Test Mod F"
description = "Test mod with optional dependency"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_f"
version = "1.0.0"
ztd_type = "openzt"

dependencies = [
    { mod_id = "test.dependency.nonexistent", name = "Nonexistent Mod", optional = true, ordering = "after" }
]
"#;

    // Test Mod G - Before dependency
    pub const META_MOD_G: &str = r#"
name = "Test Mod G"
description = "Test mod that must load before H"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_g"
version = "1.0.0"
ztd_type = "openzt"

dependencies = [
    { mod_id = "test.dependency.mod_h", name = "Test Mod H", ordering = "before" }
]
"#;

    // Test Mod H - No dependencies but G loads before it
    pub const META_MOD_H: &str = r#"
name = "Test Mod H"
description = "Test mod with no dependencies"
authors = ["OpenZT Test Suite"]
mod_id = "test.dependency.mod_h"
version = "1.0.0"
ztd_type = "openzt"
"#;
}

/// Create test mod metadata map for dependency tests
#[cfg(feature = "implementation-tests")]
fn create_test_mods() -> HashMap<String, &'static str> {
    use embedded_resources::*;
    let mut mods = HashMap::new();

    mods.insert("test.dependency.mod_a".to_string(), META_MOD_A);
    mods.insert("test.dependency.mod_b".to_string(), META_MOD_B);
    mods.insert("test.dependency.mod_c".to_string(), META_MOD_C);
    mods.insert("test.dependency.mod_d".to_string(), META_MOD_D);
    mods.insert("test.dependency.mod_e".to_string(), META_MOD_E);
    mods.insert("test.dependency.mod_f".to_string(), META_MOD_F);
    mods.insert("test.dependency.mod_g".to_string(), META_MOD_G);
    mods.insert("test.dependency.mod_h".to_string(), META_MOD_H);

    mods
}

/// Parse test mods into Meta objects
#[cfg(feature = "implementation-tests")]
fn parse_test_mods() -> HashMap<String, crate::mods::Meta> {
    let test_mods = create_test_mods();
    let mut parsed = HashMap::new();

    for (mod_id, toml_str) in test_mods {
        match toml::from_str(toml_str) {
            Ok(meta) => {
                parsed.insert(mod_id, meta);
            }
            Err(e) => {
                panic!("Failed to parse test mod {}: {}", mod_id, e);
            }
        }
    }

    parsed
}

// ============================================================================
// Tests
// ============================================================================

pub fn run_all_tests() -> Vec<TestResult> {
    vec![
        test_simple_dependency_chain(),
        test_circular_dependency_handling(),
        test_optional_dependency_warning(),
        test_before_dependency(),
        test_new_mod_insertion(),
        test_disabled_mods_excluded(),
        test_validation_detects_violations(),
    ]
}

/// Test A -> B -> C dependency chain resolves correctly
fn test_simple_dependency_chain() -> TestResult {
    let test_name = "test_simple_dependency_chain";

    let all_mods = parse_test_mods();

    // Only use mods A, B, C for this test
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_a".to_string(), all_mods["test.dependency.mod_a"].clone());
    mods.insert("test.dependency.mod_b".to_string(), all_mods["test.dependency.mod_b"].clone());
    mods.insert("test.dependency.mod_c".to_string(), all_mods["test.dependency.mod_c"].clone());

    let resolver = DependencyResolver::new(mods);
    let result = resolver.resolve_order(&[], &[]);

    // Expected order: A, B, C
    let expected = vec![
        "test.dependency.mod_a".to_string(),
        "test.dependency.mod_b".to_string(),
        "test.dependency.mod_c".to_string(),
    ];

    if result.order != expected {
        return TestResult::fail(
            test_name,
            format!("Expected order {:?}, got {:?}", expected, result.order),
        );
    }

    if !result.warnings.is_empty() {
        return TestResult::fail(
            test_name,
            format!("Expected no warnings, got: {:?}", result.warnings),
        );
    }

    TestResult::pass(test_name)
}

/// Test circular dependency D <-> E is detected and handled
fn test_circular_dependency_handling() -> TestResult {
    let test_name = "test_circular_dependency_handling";

    let all_mods = parse_test_mods();

    // Only use mods D and E (circular dependency)
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_d".to_string(), all_mods["test.dependency.mod_d"].clone());
    mods.insert("test.dependency.mod_e".to_string(), all_mods["test.dependency.mod_e"].clone());

    let resolver = DependencyResolver::new(mods);
    let result = resolver.resolve_order(&[], &[]);

    // Should have both mods in the order
    if result.order.len() != 2 {
        return TestResult::fail(
            test_name,
            format!("Expected 2 mods in order, got {}", result.order.len()),
        );
    }

    // Should have circular dependency warning
    let has_cycle_warning = result.warnings.iter().any(|w| matches!(w, ResolutionWarning::CircularDependency { .. }));

    if !has_cycle_warning {
        return TestResult::fail(
            test_name,
            "Expected CircularDependency warning".to_string(),
        );
    }

    TestResult::pass(test_name)
}

/// Test optional dependency generates warning but doesn't fail
fn test_optional_dependency_warning() -> TestResult {
    let test_name = "test_optional_dependency_warning";

    let all_mods = parse_test_mods();

    // Only use mod F (has optional dependency on nonexistent mod)
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_f".to_string(), all_mods["test.dependency.mod_f"].clone());

    let resolver = DependencyResolver::new(mods);
    let result = resolver.resolve_order(&[], &[]);

    // Should have mod F in the order
    if result.order != vec!["test.dependency.mod_f"] {
        return TestResult::fail(
            test_name,
            format!("Expected only mod_f in order, got {:?}", result.order),
        );
    }

    // Should have missing optional dependency warning
    let has_optional_warning = result.warnings.iter().any(|w| {
        matches!(w, ResolutionWarning::MissingOptionalDependency { .. })
    });

    if !has_optional_warning {
        return TestResult::fail(
            test_name,
            "Expected MissingOptionalDependency warning".to_string(),
        );
    }

    TestResult::pass(test_name)
}

/// Test "before" dependency works correctly (G before H)
fn test_before_dependency() -> TestResult {
    let test_name = "test_before_dependency";

    let all_mods = parse_test_mods();

    // Use mods G and H
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_g".to_string(), all_mods["test.dependency.mod_g"].clone());
    mods.insert("test.dependency.mod_h".to_string(), all_mods["test.dependency.mod_h"].clone());

    let resolver = DependencyResolver::new(mods);
    let result = resolver.resolve_order(&[], &[]);

    // Expected order: G, H (G must be before H)
    let expected = vec![
        "test.dependency.mod_g".to_string(),
        "test.dependency.mod_h".to_string(),
    ];

    if result.order != expected {
        return TestResult::fail(
            test_name,
            format!("Expected order {:?}, got {:?}", expected, result.order),
        );
    }

    TestResult::pass(test_name)
}

/// Test inserting a new mod into existing order
fn test_new_mod_insertion() -> TestResult {
    let test_name = "test_new_mod_insertion";

    let all_mods = parse_test_mods();

    // Start with A and C in existing order
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_a".to_string(), all_mods["test.dependency.mod_a"].clone());
    mods.insert("test.dependency.mod_b".to_string(), all_mods["test.dependency.mod_b"].clone());
    mods.insert("test.dependency.mod_c".to_string(), all_mods["test.dependency.mod_c"].clone());

    let resolver = DependencyResolver::new(mods);

    // Existing order has A and C, missing B
    let existing = vec![
        "test.dependency.mod_a".to_string(),
        "test.dependency.mod_c".to_string(),
    ];

    let result = resolver.resolve_order(&existing, &[]);

    // B should be inserted between A and C
    let expected = vec![
        "test.dependency.mod_a".to_string(),
        "test.dependency.mod_b".to_string(),
        "test.dependency.mod_c".to_string(),
    ];

    if result.order != expected {
        return TestResult::fail(
            test_name,
            format!("Expected B to be inserted between A and C, got {:?}", result.order),
        );
    }

    TestResult::pass(test_name)
}

/// Test disabled mods stay in order but new disabled mods are not added
fn test_disabled_mods_excluded() -> TestResult {
    let test_name = "test_disabled_mods_excluded";

    let all_mods = parse_test_mods();

    // Use mods A, B, C
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_a".to_string(), all_mods["test.dependency.mod_a"].clone());
    mods.insert("test.dependency.mod_b".to_string(), all_mods["test.dependency.mod_b"].clone());
    mods.insert("test.dependency.mod_c".to_string(), all_mods["test.dependency.mod_c"].clone());

    let resolver = DependencyResolver::new(mods);

    // B is in existing order but disabled - should stay in order
    // C is new and disabled - should NOT be added
    let existing = vec![
        "test.dependency.mod_a".to_string(),
        "test.dependency.mod_b".to_string(),
    ];
    let disabled = vec![
        "test.dependency.mod_b".to_string(),
        "test.dependency.mod_c".to_string(),
    ];
    let result = resolver.resolve_order(&existing, &disabled);

    // Should have A and B (B is disabled but stays in order)
    // C is new and disabled, so should not be added
    if !result.order.contains(&"test.dependency.mod_b".to_string()) {
        return TestResult::fail(
            test_name,
            "Disabled mod_b should stay in order (already exists)".to_string(),
        );
    }

    if result.order.contains(&"test.dependency.mod_c".to_string()) {
        return TestResult::fail(
            test_name,
            "New disabled mod_c should not be added to order".to_string(),
        );
    }

    if !result.order.contains(&"test.dependency.mod_a".to_string()) {
        return TestResult::fail(
            test_name,
            "Enabled mod_a should be in order".to_string(),
        );
    }

    // Expected order: A, B (C not added because it's new and disabled)
    let expected = vec![
        "test.dependency.mod_a".to_string(),
        "test.dependency.mod_b".to_string(),
    ];
    if result.order != expected {
        return TestResult::fail(
            test_name,
            format!("Expected order {:?}, got {:?}", expected, result.order),
        );
    }

    TestResult::pass(test_name)
}

/// Test validation detects ordering violations
fn test_validation_detects_violations() -> TestResult {
    let test_name = "test_validation_detects_violations";

    let all_mods = parse_test_mods();

    // Use mods A, B (B depends on A)
    let mut mods = HashMap::new();
    mods.insert("test.dependency.mod_a".to_string(), all_mods["test.dependency.mod_a"].clone());
    mods.insert("test.dependency.mod_b".to_string(), all_mods["test.dependency.mod_b"].clone());

    // Create WRONG order: B before A (violates dependency)
    let wrong_order = vec![
        "test.dependency.mod_b".to_string(),
        "test.dependency.mod_a".to_string(),
    ];

    let validation = validate_load_order(&wrong_order, &mods);

    // Should have warnings about ordering violation
    if validation.warnings.is_empty() {
        return TestResult::fail(
            test_name,
            "Expected warnings for ordering violation".to_string(),
        );
    }

    TestResult::pass(test_name)
}
