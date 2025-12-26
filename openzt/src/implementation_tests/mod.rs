#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io::Write as IoWrite;

use tracing::{error, info};
#[cfg(target_os = "windows")]
use windows::Win32::System::Console::{AllocConsole, FreeConsole};

#[cfg(target_os = "windows")]
use crate::detour_mod;

pub mod patch_rollback;

pub fn init() {
    #[cfg(target_os = "windows")]
    {
        match init_console() {
            Ok(_) => {
                let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();
                tracing_subscriber::fmt().with_ansi(enable_ansi).init();
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
        info!("Implementation tests starting...");

        // Read filepath from environment variable with default
        let test_log_path = std::env::var("OPENZT_TEST_LOG")
            .unwrap_or_else(|_| "C:\\Program Files (x86)\\Microsoft Games\\Zoo Tycoon\\openzt_implementation_tests.log".to_string());

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

        write_log("=== OpenZT Implementation Tests ===");
        write_log("");

        // Run patch rollback tests
        write_log("Running patch rollback tests...");
        let patch_results = super::patch_rollback::run_all_tests();

        let mut total_passed = 0;
        let mut total_failed = 0;

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
