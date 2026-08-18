#![allow(dead_code)]

use std::{any::Any, fmt};

use proptest::test_runner::{FailurePersistence, PersistedSeed};
use tracing::{error, info};

#[cfg(target_os = "windows")]
use crate::detour_mod;

pub fn init() {
    #[cfg(target_os = "windows")]
    {
        #[cfg(feature = "tui")]
        let tui_config: Option<&crate::tui_console::TuiConfig> = None;
        #[cfg(not(feature = "tui"))]
        let tui_config = None;

        if let Err(e) = crate::logging::init_with_console(
            &crate::logging::LoggingConfig::default(),
            tui_config,
        ) {
            eprintln!("Failed to initialize logging: {}", e);
        }

        unsafe { detour_zoo_main::init_detours() }.is_err().then(|| {
            error!("Error initialising zoo_main detours");
        });
    }
}

#[derive(Debug, Default, PartialEq)]
struct NoopFailurePersistence;

impl FailurePersistence for NoopFailurePersistence {
    fn load_persisted_failures2(&self, _source_file: Option<&'static str>) -> Vec<PersistedSeed> {
        Vec::new()
    }

    fn save_persisted_failure2(&mut self, _source_file: Option<&'static str>, _seed: PersistedSeed, _shrunken_value: &dyn fmt::Debug) {}

    fn box_clone(&self) -> Box<dyn FailurePersistence> {
        Box::new(NoopFailurePersistence)
    }

    fn eq(&self, other: &dyn FailurePersistence) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(target_os = "windows")]
#[detour_mod]
mod detour_zoo_main {
    use std::{backtrace::Backtrace, cell::Cell, fs::OpenOptions, io::Write};

    thread_local! {
        static BACKTRACE: Cell<Option<Backtrace>> = const { Cell::new(None) };
    }

    #[cfg(target_os = "windows")]
    use openzt_detour::generated::bfapp::LOAD_LANG_DLLS;
    use openzt_detour::generated::bfentity::GET_FOOTPRINT as BFENTITY_GET_FOOTPRINT;
    use openzt_detour::generated::bftile::GET_LOCAL_ELEVATION;
    use openzt_detour::generated::ztanimal::GET_FOOTPRINT as ZTANIMAL_GET_FOOTPRINT;
    use openzt_detour::generated::ztunit::GET_FOOTPRINT as ZTUNIT_GET_FOOTPRINT;
    use openzt_detour::FunctionDef;
    use proptest::prelude::ProptestConfig;
    use tracing::{error, info};

    use crate::{
        bfentitytype::{BFEntityType, ZTAnimalType, ZTUnitType},
        ztmapview::BFTile,
        ztworldmgr::{BFEntity, IVec3, ZTAnimal, ZTUnit},
    };

    // TODO: Fix this so it works with a crate/mod prefix
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(_this: u32) -> u32 {
        info!("Detour success");

        // Read filepath from environment variable with default
        let failure_log_path =
            std::env::var("OPENZT_TEST_LOG").unwrap_or_else(|_| "C:\\Program Files (x86)\\Microsoft Games\\Zoo Tycoon\\openzt_test.log".to_string());

        // Create or truncate the file
        let mut failure_log = match OpenOptions::new().create(true).write(true).truncate(true).open(&failure_log_path) {
            Ok(file) => Some(file),
            Err(e) => {
                error!("Failed to create failure log file '{}': {}", failure_log_path, e);
                None
            }
        };

        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "BFTILE_GET_LOCAL_ELEVATION";
        // Create a enum strategy so this is all tested in one test and test that each enum variant is tested
        let unknown_byte_values = vec![
            0x1, 0x4, 0x5, 0x10, 0x11, 0x14, 0x15, 0x19, 0x40, 0x41, 0x44, 0x45, 0x46, 0x50, 0x51, 0x54, 0x64, 0x91,
        ];
        let mut fail_flag = false;
        for unknown_byte_2 in unknown_byte_values {
            match runner.run(&(0..1000i32, 0..1000i32), |(x, y)| {
                let pos = IVec3::new(x, y, 0);
                let tile = BFTile::new(pos, unknown_byte_2);
                let reimplemented_result = tile.get_local_elevation(pos);

                let result = GET_LOCAL_ELEVATION.original()(&raw const tile as *const u32, &raw const pos as *const u32);
                assert_eq!(
                    result,
                    reimplemented_result,
                    "Failed for pos: {:?}, tile: {:?}, unknown_byte_2: {}, real: {}, reimplemented: {}",
                    pos,
                    tile,
                    unknown_byte_2,
                    result,
                    reimplemented_result
                );
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for unknown_byte_2: {}", unknown_byte_2);
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let proptest::test_runner::TestError::Fail(r, (x, y)) = e {
                        let pos = IVec3::new(x, y, 0);
                        let tile = BFTile::new(pos, unknown_byte_2);
                        let reimplemented_result = tile.get_local_elevation(pos);
                        let result = GET_LOCAL_ELEVATION.original()(&raw const tile as *const u32, &raw const pos as *const u32);
                        let failure_line = format!("unknown_byte_2: {}, x: {}, y: {}, real: {}, reimplemented: {}\n", unknown_byte_2, x, y, result, reimplemented_result);

                        if let Some(ref mut log_file) = failure_log {
                            if let Err(write_err) = log_file.write_all(failure_line.as_bytes()) {
                                error!("Failed to write to failure log: {}", write_err);
                            }
                        }

                        info!("Failed case ({}): x: {}, y: {}", r, x, y);
                        fail_flag = true;
                    }
                }
            }
        }

        if fail_flag {
            error!("Proptest failed for some cases, check the failure log at: {}", failure_log_path);
            std::process::exit(1);
        } else {
            let success_line = format!("Test Passed {}\n", test_name);

            if let Some(ref mut log_file) = failure_log {
                if let Err(write_err) = log_file.write_all(success_line.as_bytes()) {
                    error!("Failed to write to failure log: {}", write_err);
                }
            }
        }

        fail_flag |= run_bfentity_get_footprint_tests(&mut failure_log);
        fail_flag |= run_ztunit_get_footprint_tests(&mut failure_log);
        fail_flag |= run_ztanimal_get_footprint_tests(&mut failure_log);

        if fail_flag {
            error!("Proptest failed for some cases, check the failure log at: {}", failure_log_path);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    fn write_success_line(failure_log: &mut Option<std::fs::File>, test_name: &str) {
        let success_line = format!("Test Passed {}\n", test_name);
        if let Some(log_file) = failure_log {
            if let Err(write_err) = log_file.write_all(success_line.as_bytes()) {
                error!("Failed to write to failure log: {}", write_err);
            }
        }
    }

    /// Calls the real GET_FOOTPRINT function at `entity_ptr` and reads the `IVec3` it writes back.
    fn call_original_get_footprint(
        original: FunctionDef<unsafe extern "thiscall" fn(*const u32, *const u32, bool) -> *const u32>,
        entity_ptr: *const u32,
        use_map_footprint: bool,
    ) -> IVec3 {
        let mut result = IVec3::default();
        unsafe {
            (original.original())(entity_ptr, &raw mut result as *const u32, use_map_footprint);
        }
        result
    }

    fn run_bfentity_get_footprint_tests(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "BFENTITY_GET_FOOTPRINT";
        let mut fail_flag = false;

        match runner.run(
            &(-1000i32..1000i32, -1000i32..1000i32, -1000i32..1000i32, -8i32..8i32, proptest::bool::ANY),
            |(fx, fy, fz, rotation, use_map_footprint)| {
                let mut entity_type: BFEntityType = unsafe { std::mem::zeroed() };
                entity_type.footprintx = fx;
                entity_type.footprinty = fy;
                entity_type.footprintz = fz;

                let entity = BFEntity::new_for_test(&raw const entity_type as u32, rotation, 0);

                let reimplemented_result = entity.get_footprint(use_map_footprint);
                let real_result = call_original_get_footprint(BFENTITY_GET_FOOTPRINT, &raw const entity as *const u32, use_map_footprint);

                assert_eq!(
                    (real_result.x, real_result.y, real_result.z),
                    (reimplemented_result.x, reimplemented_result.y, reimplemented_result.z),
                    "BFEntity::get_footprint mismatch: fx={}, fy={}, fz={}, rotation={}, use_map_footprint={}, real={:?}, reimplemented={:?}",
                    fx, fy, fz, rotation, use_map_footprint, real_result, reimplemented_result
                );
                Ok(())
            },
        ) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let proptest::test_runner::TestError::Fail(r, (fx, fy, fz, rotation, use_map_footprint)) = e {
                    let failure_line = format!(
                        "{}: fx={}, fy={}, fz={}, rotation={}, use_map_footprint={}, reason={}\n",
                        test_name, fx, fy, fz, rotation, use_map_footprint, r
                    );
                    if let Some(log_file) = failure_log {
                        if let Err(write_err) = log_file.write_all(failure_line.as_bytes()) {
                            error!("Failed to write to failure log: {}", write_err);
                        }
                    }
                    fail_flag = true;
                }
            }
        }

        fail_flag
    }

    /// `ZTUnit::getFootprint`'s `use_map_footprint=true` branch virtual-dispatches through
    /// `entity_type`'s vtable (not `this`'s own), so the fixture's `ZTUnitType` needs a real
    /// vtable pointer - see `ZTUnitType::new_for_test` / `ztunit-ztanimal-footprint-crash-investigation.md`.
    fn run_ztunit_get_footprint_tests(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTUNIT_GET_FOOTPRINT";
        let mut fail_flag = false;

        match runner.run(
            &(
                -1000i32..1000i32,
                -1000i32..1000i32,
                -1000i32..1000i32,
                -1000i32..1000i32,
                -8i32..8i32,
                proptest::bool::ANY,
            ),
            |(fx, fy, fz, map_footprint, rotation, use_map_footprint)| {
                let entity_type = ZTUnitType::new_for_test(IVec3::new(fx, fy, fz), map_footprint);
                let entity = ZTUnit::new_for_test(&raw const entity_type as u32, rotation, 0);

                let reimplemented_result = entity.get_footprint(use_map_footprint);
                let real_result = call_original_get_footprint(ZTUNIT_GET_FOOTPRINT, &raw const entity as *const u32, use_map_footprint);

                assert_eq!(
                    (real_result.x, real_result.y, real_result.z),
                    (reimplemented_result.x, reimplemented_result.y, reimplemented_result.z),
                    "ZTUnit::get_footprint mismatch: fx={}, fy={}, fz={}, map_footprint={}, rotation={}, use_map_footprint={}, real={:?}, reimplemented={:?}",
                    fx, fy, fz, map_footprint, rotation, use_map_footprint, real_result, reimplemented_result
                );
                Ok(())
            },
        ) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let proptest::test_runner::TestError::Fail(r, (fx, fy, fz, map_footprint, rotation, use_map_footprint)) = e {
                    let failure_line = format!(
                        "{}: fx={}, fy={}, fz={}, map_footprint={}, rotation={}, use_map_footprint={}, reason={}\n",
                        test_name, fx, fy, fz, map_footprint, rotation, use_map_footprint, r
                    );
                    if let Some(log_file) = failure_log {
                        if let Err(write_err) = log_file.write_all(failure_line.as_bytes()) {
                            error!("Failed to write to failure log: {}", write_err);
                        }
                    }
                    fail_flag = true;
                }
            }
        }

        fail_flag
    }

    /// `ZTAnimal::getFootprint`'s `is_egg`/`is_boxed` branches virtual-dispatch through
    /// `entity_type`'s vtable unconditionally (regardless of `use_map_footprint`) - same fixture
    /// requirement as `ZTUnit` above.
    fn run_ztanimal_get_footprint_tests(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTANIMAL_GET_FOOTPRINT";
        let mut fail_flag = false;

        match runner.run(
            &(
                (-1000i32..1000i32, -1000i32..1000i32, -1000i32..1000i32),
                -1000i32..1000i32,
                (-1000i32..1000i32, -1000i32..1000i32, -1000i32..1000i32),
                (-1000i32..1000i32, -1000i32..1000i32, -1000i32..1000i32),
                -8i32..8i32,
                proptest::bool::ANY,
                proptest::bool::ANY,
                proptest::bool::ANY,
            ),
            |((fx, fy, fz), map_footprint, box_footprint, egg_footprint, rotation, use_map_footprint, is_egg, is_boxed)| {
                let entity_type = ZTAnimalType::new_for_test(
                    IVec3::new(fx, fy, fz),
                    map_footprint,
                    IVec3::new(box_footprint.0, box_footprint.1, box_footprint.2),
                    IVec3::new(egg_footprint.0, egg_footprint.1, egg_footprint.2),
                );
                let entity = ZTAnimal::new_for_test(&raw const entity_type as u32, rotation, 0, is_egg, is_boxed);

                let reimplemented_result = entity.get_footprint(use_map_footprint);
                let real_result = call_original_get_footprint(ZTANIMAL_GET_FOOTPRINT, &raw const entity as *const u32, use_map_footprint);

                assert_eq!(
                    (real_result.x, real_result.y, real_result.z),
                    (reimplemented_result.x, reimplemented_result.y, reimplemented_result.z),
                    "ZTAnimal::get_footprint mismatch: is_egg={}, is_boxed={}, rotation={}, use_map_footprint={}, real={:?}, reimplemented={:?}",
                    is_egg, is_boxed, rotation, use_map_footprint, real_result, reimplemented_result
                );
                Ok(())
            },
        ) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let proptest::test_runner::TestError::Fail(r, ((fx, fy, fz), map_footprint, box_footprint, egg_footprint, rotation, use_map_footprint, is_egg, is_boxed)) = e {
                    let failure_line = format!(
                        "{}: fx={}, fy={}, fz={}, map_footprint={}, box_footprint={:?}, egg_footprint={:?}, rotation={}, use_map_footprint={}, is_egg={}, is_boxed={}, reason={}\n",
                        test_name, fx, fy, fz, map_footprint, box_footprint, egg_footprint, rotation, use_map_footprint, is_egg, is_boxed, r
                    );
                    if let Some(log_file) = failure_log {
                        if let Err(write_err) = log_file.write_all(failure_line.as_bytes()) {
                            error!("Failed to write to failure log: {}", write_err);
                        }
                    }
                    fail_flag = true;
                }
            }
        }

        fail_flag
    }
}
