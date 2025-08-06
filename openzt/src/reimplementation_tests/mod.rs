#![allow(dead_code)]

use std::any::Any;
use std::fmt;

#[cfg(target_os = "windows")]
use crate::detour_mod;
use proptest::test_runner::{FailurePersistence, PersistedSeed};
use tracing::{error, info};

#[cfg(target_os = "windows")]
use windows::Win32::System::{Console::{AllocConsole, FreeConsole}};

pub fn init() {
    #[cfg(target_os = "windows")]
    {
        match init_console() {
            Ok(_) => {
                let enable_ansi = enable_ansi_support::enable_ansi_support().is_ok();
                tracing_subscriber::fmt().with_ansi(enable_ansi).init();
            },
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

#[derive(Debug, Default, PartialEq)]
struct NoopFailurePersistence;

impl FailurePersistence for NoopFailurePersistence {
    fn load_persisted_failures2(
        &self,
        _source_file: Option<&'static str>,
    ) -> Vec<PersistedSeed> {
        Vec::new()
    }

    fn save_persisted_failure2(
        &mut self,
        _source_file: Option<&'static str>,
        _seed: PersistedSeed,
        _shrunken_value: &dyn fmt::Debug,
    ) {
    }

    fn box_clone(&self) -> Box<dyn FailurePersistence> {
        Box::new(NoopFailurePersistence)
    }

    fn eq(&self, other: &dyn FailurePersistence) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |x| x == self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(target_os = "windows")]
#[detour_mod]
mod detour_zoo_main {
    use std::panic;

    use tracing::{error, info};
    #[cfg(target_os = "windows")]
    use openzt_detour::{LOAD_LANG_DLLS, BFTILE_GET_LOCAL_ELEVATION};
    use proptest::proptest;
    use proptest::prelude::ProptestConfig;

    use crate::ztworldmgr::IVec3;
    use crate::ztmapview::BFTile;

    // TODO: Fix this so it works with a crate/mod prefix
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(this: u32) -> u32 {
        info!("Detour success");

        panic::catch_unwind(|| {
             let runner = ProptestConfig { 
                // source_file: Some(src_file_absolute_path), 
                // failure_persistence: Some(Box::new(FileFailurePersistence::Direct(persistence_absolute_path))),
                failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
                .. ProptestConfig::default() 
            };
            let unknown_byte_values = vec![0x1,0x4,0x5,0x10,0x11,0x14,0x15,0x19,0x40,0x41,0x44,0x45,0x46,0x50,0x51,0x54,0x64,0x91];
            for unknown_byte_2 in unknown_byte_values {
                // let runner = ProptestConfig::with_failure_persistence(FileFailurePersistence::Off)::with_source_file(file!());
                proptest!(runner, |(x in 0i32..1000i32, y in 0i32..1000i32)| {
                    let pos = IVec3::new(x, y, 0);
                    let tile = BFTile::new(pos, unknown_byte_2);
                    let reimplemented_result = tile.get_local_elevation(pos);

                    // let fb_ref_good_new = &raw const ms.field_b; 
                    let result = BFTILE_GET_LOCAL_ELEVATION.original()(&raw const tile as u32, &raw const pos as u32);
                    assert_eq!(result+1, reimplemented_result, "Failed for pos: {:?}, tile: {:?}, unknown_byte_2: {}", pos, tile, unknown_byte_2);
                });
            }
        }).unwrap_or_else(|e| {
            error!("Proptest failed: {:?}", e);
        });
        

        unsafe { LOAD_LANG_DLLS_DETOUR.call(this) }
    }
}
