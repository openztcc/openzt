#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io::Write as IoWrite;

use tracing::{error, info};
#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{AllocConsole, FreeConsole};

#[cfg(target_os = "windows")]
use crate::detour_mod;

pub mod dependency_resolution;
pub mod legacy_attributes;
pub mod loading_order;
pub mod patch_rollback;

/// Result of a single test
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

impl TestResult {
    pub fn pass(name: &str) -> Self {
        TestResult {
            name: name.to_string(),
            passed: true,
            error: None,
        }
    }

    pub fn fail(name: &str, error: String) -> Self {
        TestResult {
            name: name.to_string(),
            passed: false,
            error: Some(error),
        }
    }
}

pub fn init() {
    #[cfg(target_os = "windows")]
    {
        match init_console() {
            Ok(_) => {
                // let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();
                // tracing_subscriber::fmt().with_ansi(enable_ansi).init();
                let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();

                // Set up file appender - truncate file on startup
                let log_file = std::fs::File::create("openzt.log")
                    .expect("Failed to create openzt.log");
                let (non_blocking_file, _guard) = tracing_appender::non_blocking(log_file);

                // Set up layered logging to both console and file
                use tracing_subscriber::layer::SubscriberExt;
                use tracing_subscriber::util::SubscriberInitExt;

                tracing_subscriber::registry()
                    .with(tracing_subscriber::fmt::layer().with_ansi(enable_ansi).with_writer(std::io::stdout))
                    .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(non_blocking_file))
                    .init();

                // Store the guard to prevent it from being dropped
                std::mem::forget(_guard);
            }
            Err(e) => {
                info!("Failed to initialize console: {}", e);
            }
        }

        unsafe { detour_zoo_main::init_detours() }.is_err().then(|| {
            error!("Error initialising zoo_main detours");
        });
    }
}

#[cfg(target_os = "windows")]
fn init_console() -> windows::core::Result<()> {
    // Free the current console
    unsafe { FreeConsole()? };

    // Allocate a new console
    unsafe { AllocConsole()? };

    Ok(())
}

/// Setup test target files for loading order tests
fn setup_test_files() -> anyhow::Result<()> {
    use crate::resource_manager::{
        lazyresourcemap::add_ztfile,
        ztfile::{ZTFile, ZTFileType},
    };
    use std::ffi::CString;
    use std::path::Path;

    // Create animals/test.ai for habitat reference tests
    let test_ai_content = "[Habitat]\n";
    let test_ai_cstring = CString::new(test_ai_content)?;
    let test_ai_file = ZTFile::Text(
        test_ai_cstring,
        ZTFileType::Ai,
        test_ai_content.len() as u32,
    );
    add_ztfile(Path::new(""), "animals/test.ai".to_string(), test_ai_file)?;

    // Create animals/test_order.ai for patch order tests
    let test_order_content = "[Test]\n";
    let test_order_cstring = CString::new(test_order_content)?;
    let test_order_file = ZTFile::Text(
        test_order_cstring,
        ZTFileType::Ai,
        test_order_content.len() as u32,
    );
    add_ztfile(
        Path::new(""),
        "animals/test_order.ai".to_string(),
        test_order_file,
    )?;

    Ok(())
}

/// Load the embedded test mod into the game
#[cfg(feature = "integration-tests")]
fn load_test_mod() -> anyhow::Result<()> {
    use crate::resource_manager::openzt_mods::load_open_zt_mod_from_memory;
    use std::path::Path;

    info!("Loading embedded test mod: loading-order-test");

    let file_map = loading_order::create_test_mod_file_map();
    load_open_zt_mod_from_memory(file_map, "loading-order-test", Path::new(""))?;

    info!("Test mod loaded successfully");
    Ok(())
}

#[cfg(target_os = "windows")]
#[detour_mod]
mod detour_zoo_main {
    #[cfg(target_os = "windows")]
    use openzt_detour::gen::bfapp::LOAD_LANG_DLLS;
    use tracing::{error, info};

    use std::fs::OpenOptions;
    use std::io::Write as IoWrite;

    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(_this: u32) -> u32 {
        info!("Integration tests starting...");

        // Clear load order tracker
        #[cfg(feature = "integration-tests")]
        crate::resource_manager::openzt_mods::loading::clear_load_tracker();

        // Setup test target files for loading order tests
        if let Err(e) = super::setup_test_files() {
            error!("Failed to setup test files: {}", e);
            std::process::exit(1);
        }

        // Load embedded test mod
        if let Err(e) = super::load_test_mod() {
            error!("Failed to load test mod: {}", e);
            std::process::exit(1);
        }

        // Load legacy entities for integration tests (needed for legacy_attributes tests)
        #[cfg(feature = "integration-tests")]
        match crate::resource_manager::load_legacy_entities_for_tests() {
            Ok(count) => {
                info!("Loaded {} legacy .cfg files for testing", count);
            }
            Err(e) => {
                info!("Note: Legacy entity loading not available ({}), adding test attributes", e);
                // Add some test legacy attributes for testing
                use crate::resource_manager::openzt_mods::legacy_attributes::{add_legacy_entity, LegacyEntityAttributes, LegacyEntityType, SubtypeAttributes};

                // Add test elephant (animal with subtypes)
                let mut elephant_attrs = LegacyEntityAttributes::new("elephant".to_string());
                elephant_attrs.subtype_attributes.insert("m".to_string(), SubtypeAttributes { subtype: "m".to_string(), name_id: Some(1001) });
                elephant_attrs.subtype_attributes.insert("f".to_string(), SubtypeAttributes { subtype: "f".to_string(), name_id: Some(1002) });
                let _ = add_legacy_entity(LegacyEntityType::Animal, "elephant".to_string(), elephant_attrs);

                // Add test zookeeper (staff with subtypes)
                let mut zookeeper_attrs = LegacyEntityAttributes::new("zookeeper".to_string());
                zookeeper_attrs.subtype_attributes.insert("m".to_string(), SubtypeAttributes { subtype: "m".to_string(), name_id: Some(1501) });
                zookeeper_attrs.subtype_attributes.insert("f".to_string(), SubtypeAttributes { subtype: "f".to_string(), name_id: Some(1502) });
                let _ = add_legacy_entity(LegacyEntityType::Staff, "zookeeper".to_string(), zookeeper_attrs);

                // Add test atltank fence (insert 'f' first to ensure it's returned as default)
                let mut fence_attrs = LegacyEntityAttributes::new("atltank".to_string());
                fence_attrs.subtype_attributes.insert("f".to_string(), SubtypeAttributes { subtype: "f".to_string(), name_id: Some(2001) });
                fence_attrs.subtype_attributes.insert("g".to_string(), SubtypeAttributes { subtype: "g".to_string(), name_id: Some(2002) });
                let _ = add_legacy_entity(LegacyEntityType::Fence, "atltank".to_string(), fence_attrs);

                // Add test atltank wall
                let mut wall_attrs = LegacyEntityAttributes::new("atltank".to_string());
                wall_attrs.subtype_attributes.insert("f".to_string(), SubtypeAttributes { subtype: "f".to_string(), name_id: Some(2501) });
                wall_attrs.subtype_attributes.insert("g".to_string(), SubtypeAttributes { subtype: "g".to_string(), name_id: Some(2502) });
                let _ = add_legacy_entity(LegacyEntityType::Wall, "atltank".to_string(), wall_attrs);

                // Add test restroom (building without subtypes)
                let mut restroom_attrs = LegacyEntityAttributes::new("restroom".to_string());
                restroom_attrs.subtype_attributes.insert("".to_string(), SubtypeAttributes { subtype: "".to_string(), name_id: Some(3001) });
                let _ = add_legacy_entity(LegacyEntityType::Building, "restroom".to_string(), restroom_attrs);

                // Add test rock item
                let mut rock_attrs = LegacyEntityAttributes::new("rock".to_string());
                rock_attrs.subtype_attributes.insert("".to_string(), SubtypeAttributes { subtype: "".to_string(), name_id: Some(4001) });
                let _ = add_legacy_entity(LegacyEntityType::Item, "rock".to_string(), rock_attrs);

                // Add test guest
                let mut guest_attrs = LegacyEntityAttributes::new("guest".to_string());
                guest_attrs.subtype_attributes.insert("man".to_string(), SubtypeAttributes { subtype: "man".to_string(), name_id: Some(5001) });
                guest_attrs.subtype_attributes.insert("woman".to_string(), SubtypeAttributes { subtype: "woman".to_string(), name_id: Some(5002) });
                let _ = add_legacy_entity(LegacyEntityType::Guest, "guest".to_string(), guest_attrs);

                info!("Test legacy attributes added");
            }
        }

        // Read filepath from environment variable with default
        let test_log_path = std::env::var("OPENZT_TEST_LOG")
            .unwrap_or_else(|_| "C:\\Program Files (x86)\\Microsoft Games\\Zoo Tycoon\\openzt_integration_tests.log".to_string());

        // Create or truncate the file
        let mut test_log = match OpenOptions::new().create(true).write(true).truncate(true).open(&test_log_path) {
            Ok(file) => Some(file),
            Err(e) => {
                error!("Failed to create test log file '{}': {}", test_log_path, e);
                None
            }
        };

        let mut write_log = |msg: &str| {
            info!("{}", msg);
            if let Some(ref mut log_file) = test_log {
                let _ = writeln!(log_file, "{}", msg);
            }
        };

        write_log("=== OpenZT Integration Tests ===");
        write_log("");

        // Run dependency resolution tests
        write_log("Running dependency resolution tests...");
        let dependency_results = super::dependency_resolution::run_all_tests();

        let mut total_passed = 0;
        let mut total_failed = 0;

        for result in &dependency_results {
            if result.passed {
                write_log(&format!("  ✓ {}", result.name));
                total_passed += 1;
            } else {
                write_log(&format!("  ✗ {} - {}", result.name, result.error.as_ref().unwrap_or(&"Unknown error".to_string())));
                total_failed += 1;
            }
        }

        write_log("");

        // Run patch rollback tests
        write_log("Running patch rollback tests...");
        let patch_results = super::patch_rollback::run_all_tests();

        for result in &patch_results {
            if result.passed {
                write_log(&format!("  ✓ {}", result.name));
                total_passed += 1;
            } else {
                write_log(&format!("  ✗ {} - {}", result.name, result.error.as_ref().unwrap_or(&"Unknown error".to_string())));
                total_failed += 1;
            }
        }

        write_log("");

        // Run loading order tests
        write_log("Running loading order tests...");
        let loading_results = super::loading_order::run_all_tests();

        for result in &loading_results {
            if result.passed {
                write_log(&format!("  ✓ {}", result.name));
                total_passed += 1;
            } else {
                write_log(&format!("  ✗ {} - {}", result.name, result.error.as_ref().unwrap_or(&"Unknown error".to_string())));
                total_failed += 1;
            }
        }

        write_log("");

        // Run legacy attributes tests
        write_log("Running legacy attributes tests...");
        let legacy_attributes_results = super::legacy_attributes::run_all_tests();

        for result in &legacy_attributes_results {
            if result.passed {
                write_log(&format!("  ✓ {}", result.name));
                total_passed += 1;
            } else {
                write_log(&format!("  ✗ {} - {}", result.name, result.error.as_ref().unwrap_or(&"Unknown error".to_string())));
                total_failed += 1;
            }
        }

        write_log("");
        write_log(&format!("Results: {} passed, {} failed", total_passed, total_failed));

        if total_failed > 0 {
            write_log("");
            write_log(&format!("FAILED - Check log at: {}", test_log_path));
            std::process::exit(1);
        } else {
            write_log("");
            write_log("ALL TESTS PASSED");
            std::process::exit(0);
        }
    }
}
