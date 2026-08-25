#![allow(dead_code)]

use std::{any::Any, fmt};

use proptest::test_runner::{FailurePersistence, PersistedSeed};
use tracing::{error, info};

#[cfg(target_os = "windows")]
use crate::detour_mod;

/// Redirects the `fwrite`/`fread`-shaped primitives `ZTResearchMgr::save`/`load` (and every other
/// vanilla `*::save`/`*::load`) go through, to in-memory buffers - lets `detour_zoo_main`'s live
/// research save/load comparison call the real `.original()` functions without a real save file.
#[cfg(target_os = "windows")]
mod io_redirect;

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

        io_redirect::init();

        // Installs `resource_manager::init()`'s hooks so `LAZY_RESOURCE_MAP` is populated before
        // `detour_zoo_main`'s battery runs, letting `ZTMARKETINGMGR_LOAD_CONFIGURATIONS`'s
        // `load_configurations()` call resolve real game resources via `get_file`.
        crate::resource_manager::init();

        // Installs the research/marketing `SAVE`/`LOAD` detours so the corresponding comparison tests'
        // `mgr.save()`/`mgr.load()` calls exercise the actual promoted live path.
        crate::ztresearch::research_save_reimplementation::init();
        crate::ztmarketing::marketing_save_reimplementation::init();

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
    use std::{
        backtrace::Backtrace,
        cell::Cell,
        ffi::{CStr, CString},
        fs::OpenOptions,
        io::Write,
        mem::size_of,
        sync::{
            atomic::{AtomicBool, Ordering},
            Once, OnceLock,
        },
    };

    thread_local! {
        static BACKTRACE: Cell<Option<Backtrace>> = const { Cell::new(None) };
    }

    #[cfg(target_os = "windows")]
    use openzt_detour::generated::bfapp::LOAD_LANG_DLLS;
    use openzt_detour::generated::bfentity::GET_FOOTPRINT as BFENTITY_GET_FOOTPRINT;
    use openzt_detour::generated::bftile::GET_LOCAL_ELEVATION;
    use openzt_detour::generated::standalone;
    use openzt_detour::generated::ztanimal::GET_FOOTPRINT as ZTANIMAL_GET_FOOTPRINT;
    use openzt_detour::generated::ztapp::UPDATE_SIM;
    use openzt_detour::generated::ztmarketing;
    use openzt_detour::generated::ztmarketingmgr::{CLEAR_CONFIGURATIONS as ZTMARKETINGMGR_CLEAR_CONFIGURATIONS, LOAD_CONFIGURATIONS, UPDATE as ZTMARKETINGMGR_UPDATE};
    use openzt_detour::generated::ztresearchbranch;
    use openzt_detour::generated::ztresearchbranch::GET_FUNDING_TEXT as ZTRESEARCHBRANCH_GET_FUNDING_TEXT;
    use openzt_detour::generated::ztresearchbranch::UPDATE as ZTRESEARCHBRANCH_UPDATE;
    use openzt_detour::generated::ztresearchmgr;
    use openzt_detour::generated::ztresearchmgr::FORCE_RESEARCH as ZTRESEARCHMGR_FORCE_RESEARCH;
    use openzt_detour::generated::ztresearchmgr::UPDATE as ZTRESEARCHMGR_UPDATE;
    use openzt_detour::generated::ztresearchprogram;
    use openzt_detour::generated::ztthought as gen_ztthought;
    use openzt_detour::generated::ztthoughtmgr as gen_ztthoughtmgr;
    use openzt_detour::generated::ztmegatilemgr as gen_ztmegatilemgr;
    use openzt_detour::generated::zthabitatmgr;
    use openzt_detour::generated::ztui_gameopts::LOAD_FILE as ZTUI_GAMEOPTS_LOAD_FILE;
    use openzt_detour::generated::ztunit::GET_FOOTPRINT as ZTUNIT_GET_FOOTPRINT;
    use openzt_detour::FunctionDef;
    use proptest::prelude::*;
    use tracing::{error, info};

    use crate::{
        bfentitytype::{BFEntityType, ZTAnimalType, ZTUnitType},
        globals::globals,
        util::{get_from_memory, save_to_memory, ZTBufferString, ZTString},
        zthabitatmgr::ZTHabitat,
        ztmapview::BFTile,
        ztmarketing::{live_support as marketing_live_support, marketing_save_reimplementation, predict_mgr_update, ZTMarketing, ZTMarketingMgr},
        ztresearch::research_save_reimplementation::{self, live_support, SaveRecord},
        ztresearch::{predict_branch_progress, predict_update, ZTResearchBranch, ZTResearchEffectKind, ZTResearchMgr},
        ztthoughtmgr::{live_support as thought_live_support, ZTThought, ZTThoughtMgr},
        ztmegatilemgr::live_support as megatile_live_support,
        ztworldmgr::{BFEntity, IVec3, ZTAnimal, ZTUnit},
    };

    use super::io_redirect;

    /// One generated program: `saved_progress_bits`, when `Some`, becomes a `Program` record in the
    /// stream fed to `load`; the initial `current_progress`/`target_cost` only matter for the `save`
    /// test (`load` always resets `current_progress` to `0` first regardless of these). `effect_kind_raw`
    /// spans `-1..=8` (unset through one past the last valid kind) but is only consumed by
    /// `ZTRESEARCHMGR_LOAD`'s own tree-building - `generated_branches` below (shared with the `save`/
    /// `force_research` tests) deliberately ignores it and always pins `-1`, since only `load`'s tail
    /// (`on_completion`, for any program whose loaded `current_progress` ends up `>= target_cost`)
    /// dispatches on this field.
    #[derive(Debug, Clone)]
    struct ProgramCase {
        id: i32,
        target_cost: f32,
        initial_progress: f32,
        saved_progress_bits: Option<u32>,
        effect_kind_raw: i32,
    }

    #[derive(Debug, Clone)]
    struct CategoryCase {
        id: i32,
        initial_enabled: u8,
        saved_enabled: Option<u8>,
        programs: Vec<ProgramCase>,
    }

    /// `saved_funding_level`, when generated, deliberately spans negative/in-range/out-of-range
    /// relative to `funding_level_count`, to exercise `ZTResearchMgr::load`'s clamp rule.
    #[derive(Debug, Clone)]
    struct BranchCase {
        id: i32,
        initial_funding_level: i32,
        funding_level_count: usize,
        saved_funding_level: Option<i32>,
        categories: Vec<CategoryCase>,
    }

    fn program_case_strategy() -> impl Strategy<Value = ProgramCase> {
        (any::<i32>(), any::<f32>(), any::<f32>(), prop::option::of(any::<u32>()), -1i32..=8i32).prop_map(
            |(id, target_cost, initial_progress, saved_progress_bits, effect_kind_raw)| ProgramCase {
                id,
                target_cost,
                initial_progress,
                saved_progress_bits,
                effect_kind_raw,
            },
        )
    }

    fn category_case_strategy() -> impl Strategy<Value = CategoryCase> {
        (any::<i32>(), any::<u8>(), prop::option::of(any::<u8>()), prop::collection::vec(program_case_strategy(), 0..3)).prop_map(
            |(id, initial_enabled, saved_enabled, programs)| CategoryCase { id, initial_enabled, saved_enabled, programs },
        )
    }

    fn branch_case_strategy() -> impl Strategy<Value = BranchCase> {
        (
            any::<i32>(),
            any::<i32>(),
            0usize..4,
            prop::option::of(prop_oneof![Just(-1i32), 0i32..8i32]),
            prop::collection::vec(category_case_strategy(), 0..3),
        )
            .prop_map(|(id, initial_funding_level, funding_level_count, saved_funding_level, categories)| BranchCase {
                id,
                initial_funding_level,
                funding_level_count,
                saved_funding_level,
                categories,
            })
    }

    /// Converts generated cases into the synthetic tree `live_support::with_synthetic_branches` splices
    /// into `ZTResearchMgr::branch_array`, using each case's *initial* field values. Used by the `save`/
    /// `force_research` tests, neither of which dispatch on `effect_kind_raw` - always pinned to `-1`
    /// here regardless of what `ProgramCase::effect_kind_raw` was generated as, keeping this helper's
    /// output unchanged from before that field existed. See `generated_branches_for_load` for the one
    /// call site that does vary it.
    fn generated_branches(cases: &[BranchCase]) -> Vec<live_support::GeneratedBranch> {
        cases
            .iter()
            .map(|branch| live_support::GeneratedBranch {
                id: branch.id,
                current_funding_level: branch.initial_funding_level,
                funding_level_count: branch.funding_level_count,
                categories: branch
                    .categories
                    .iter()
                    .map(|category| live_support::GeneratedCategory {
                        id: category.id,
                        enabled: category.initial_enabled,
                        programs: category
                            .programs
                            .iter()
                            .map(|program| live_support::GeneratedProgram {
                                id: program.id,
                                target_cost: program.target_cost,
                                current_progress: program.initial_progress,
                                effect_kind_raw: -1,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// Same as `generated_branches`, but threads each program's generated `effect_kind_raw` through
    /// instead of pinning `-1` - used only by `ZTRESEARCHMGR_LOAD`, whose tail (`on_completion`, for any
    /// program whose loaded `current_progress` ends up `>= target_cost`) is the one place in this file's
    /// live battery that actually dispatches on this field.
    fn generated_branches_for_load(cases: &[BranchCase]) -> Vec<live_support::GeneratedBranch> {
        cases
            .iter()
            .map(|branch| live_support::GeneratedBranch {
                id: branch.id,
                current_funding_level: branch.initial_funding_level,
                funding_level_count: branch.funding_level_count,
                categories: branch
                    .categories
                    .iter()
                    .map(|category| live_support::GeneratedCategory {
                        id: category.id,
                        enabled: category.initial_enabled,
                        programs: category
                            .programs
                            .iter()
                            .map(|program| live_support::GeneratedProgram {
                                id: program.id,
                                target_cost: program.target_cost,
                                current_progress: program.initial_progress,
                                effect_kind_raw: program.effect_kind_raw,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// One program-per-category case for the `ZTRESEARCHMGR_FORCE_RESEARCH` comparison. Unlike
    /// `BranchCase`/`CategoryCase` above (which generate `0..3` programs per category to exercise
    /// `save`/`load`), `ZTResearchMgr::forceResearch` walks *every* program in *every* category (see
    /// `ZTResearchMgr::force_research`'s doc comment) and then calls `pick_random_program` once per
    /// branch - fixing exactly one program per category keeps that RNG-driven call a safe, deterministic
    /// no-crash operation without needing to also compare which program it ends up selecting.
    #[derive(Debug, Clone)]
    struct ForceResearchProgramCase {
        id: i32,
        target_cost: f32,
        initial_progress: f32,
    }

    #[derive(Debug, Clone)]
    struct ForceResearchCategoryCase {
        id: i32,
        program: ForceResearchProgramCase,
    }

    #[derive(Debug, Clone)]
    struct ForceResearchBranchCase {
        id: i32,
        categories: Vec<ForceResearchCategoryCase>,
    }

    fn force_research_program_case_strategy() -> impl Strategy<Value = ForceResearchProgramCase> {
        (any::<i32>(), any::<f32>(), any::<f32>())
            .prop_map(|(id, target_cost, initial_progress)| ForceResearchProgramCase { id, target_cost, initial_progress })
    }

    fn force_research_category_case_strategy() -> impl Strategy<Value = ForceResearchCategoryCase> {
        (any::<i32>(), force_research_program_case_strategy()).prop_map(|(id, program)| ForceResearchCategoryCase { id, program })
    }

    fn force_research_branch_case_strategy() -> impl Strategy<Value = ForceResearchBranchCase> {
        (any::<i32>(), prop::collection::vec(force_research_category_case_strategy(), 0..3))
            .prop_map(|(id, categories)| ForceResearchBranchCase { id, categories })
    }

    /// Every generated category is `enabled` with exactly one program, and every branch gets an empty
    /// funding table - `force_research`/`forceResearch` never read funding levels, only
    /// `pick_random_program` does (for `current_funding_rate`, unrelated to program selection).
    fn force_research_generated_branches(cases: &[ForceResearchBranchCase]) -> Vec<live_support::GeneratedBranch> {
        cases
            .iter()
            .map(|branch| live_support::GeneratedBranch {
                id: branch.id,
                current_funding_level: 0,
                funding_level_count: 0,
                categories: branch
                    .categories
                    .iter()
                    .map(|category| live_support::GeneratedCategory {
                        id: category.id,
                        enabled: 1,
                        programs: vec![live_support::GeneratedProgram {
                            id: category.program.id,
                            target_cost: category.program.target_cost,
                            current_progress: category.program.initial_progress,
                            effect_kind_raw: -1,
                        }],
                    })
                    .collect(),
            })
            .collect()
    }

    /// One generated program for `ZTRESEARCHMGR_LOOKUPS`, id-only - `get_branch`/`get_category`/
    /// `get_program` only ever compare against `id`.
    #[derive(Debug, Clone)]
    struct LookupProgramCase {
        id: i32,
    }

    #[derive(Debug, Clone)]
    struct LookupCategoryCase {
        id: i32,
        programs: Vec<LookupProgramCase>,
    }

    #[derive(Debug, Clone)]
    struct LookupBranchCase {
        id: i32,
        categories: Vec<LookupCategoryCase>,
    }

    /// Small, overlapping id range (`0..8`), unlike the wide `any::<i32>()` ids `BranchCase`/etc. use
    /// elsewhere in this file (which essentially never collide by chance) - so generated
    /// `ZTRESEARCHMGR_LOOKUPS` trees actually produce duplicate/colliding ids across branches/
    /// categories/programs at a reasonable rate, exercising `get_branch`/`get_category`/`get_program`'s
    /// "first match in traversal order" semantics.
    fn lookup_program_case_strategy() -> impl Strategy<Value = LookupProgramCase> {
        (0i32..8i32).prop_map(|id| LookupProgramCase { id })
    }

    fn lookup_category_case_strategy() -> impl Strategy<Value = LookupCategoryCase> {
        (0i32..8i32, prop::collection::vec(lookup_program_case_strategy(), 0..3)).prop_map(|(id, programs)| LookupCategoryCase { id, programs })
    }

    fn lookup_branch_case_strategy() -> impl Strategy<Value = LookupBranchCase> {
        (0i32..8i32, prop::collection::vec(lookup_category_case_strategy(), 0..3)).prop_map(|(id, categories)| LookupBranchCase { id, categories })
    }

    /// Converts generated lookup cases into the synthetic tree for `ZTRESEARCHMGR_LOOKUPS` - read-only,
    /// so unlike `generated_branches`/`force_research_generated_branches` nothing here needs to be
    /// realistic beyond the id fields under test; funding/enabled/cost/progress/effect fields are all
    /// fixed to inert values.
    fn lookup_generated_branches(cases: &[LookupBranchCase]) -> Vec<live_support::GeneratedBranch> {
        cases
            .iter()
            .map(|branch| live_support::GeneratedBranch {
                id: branch.id,
                current_funding_level: 0,
                funding_level_count: 0,
                categories: branch
                    .categories
                    .iter()
                    .map(|category| live_support::GeneratedCategory {
                        id: category.id,
                        enabled: 1,
                        programs: category
                            .programs
                            .iter()
                            .map(|program| live_support::GeneratedProgram {
                                id: program.id,
                                target_cost: 0.0,
                                current_progress: 0.0,
                                effect_kind_raw: -1,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// One generated program for `ZTRESEARCHMGR_SET_EFFECT_DISCOUNT`: `effect_kind_raw` spans `-1..=8`
    /// (unset through one past the last valid kind) to cover both matching and non-matching programs
    /// within the same tree; `target_cost` is bounded well away from `f32`'s extremes since the
    /// discount math (`(100 - discount_pct) as f32 * target_cost * 0.01`) is compared bit-for-bit and
    /// non-finite/extreme inputs aren't a meaningful case to compare (same reasoning as
    /// `funding_level_case_strategy`'s `cost` bound).
    #[derive(Debug, Clone)]
    struct EffectDiscountProgramCase {
        id: i32,
        target_cost: f32,
        effect_kind_raw: i32,
    }

    #[derive(Debug, Clone)]
    struct EffectDiscountCategoryCase {
        id: i32,
        programs: Vec<EffectDiscountProgramCase>,
    }

    #[derive(Debug, Clone)]
    struct EffectDiscountBranchCase {
        id: i32,
        categories: Vec<EffectDiscountCategoryCase>,
    }

    fn effect_discount_program_case_strategy() -> impl Strategy<Value = EffectDiscountProgramCase> {
        (any::<i32>(), -1_000_000f32..1_000_000f32, -1i32..=8i32)
            .prop_map(|(id, target_cost, effect_kind_raw)| EffectDiscountProgramCase { id, target_cost, effect_kind_raw })
    }

    fn effect_discount_category_case_strategy() -> impl Strategy<Value = EffectDiscountCategoryCase> {
        (any::<i32>(), prop::collection::vec(effect_discount_program_case_strategy(), 0..3))
            .prop_map(|(id, programs)| EffectDiscountCategoryCase { id, programs })
    }

    fn effect_discount_branch_case_strategy() -> impl Strategy<Value = EffectDiscountBranchCase> {
        (any::<i32>(), prop::collection::vec(effect_discount_category_case_strategy(), 0..3))
            .prop_map(|(id, categories)| EffectDiscountBranchCase { id, categories })
    }

    /// Converts generated cases into a synthetic tree for `ZTRESEARCHMGR_SET_EFFECT_DISCOUNT` - called
    /// twice per test case (see the test itself) to build two independently-constructed but
    /// structurally identical trees, since `set_effect_discount` mutates `target_cost` in place.
    fn effect_discount_generated_branches(cases: &[EffectDiscountBranchCase]) -> Vec<live_support::GeneratedBranch> {
        cases
            .iter()
            .map(|branch| live_support::GeneratedBranch {
                id: branch.id,
                current_funding_level: 0,
                funding_level_count: 0,
                categories: branch
                    .categories
                    .iter()
                    .map(|category| live_support::GeneratedCategory {
                        id: category.id,
                        enabled: 1,
                        programs: category
                            .programs
                            .iter()
                            .map(|program| live_support::GeneratedProgram {
                                id: program.id,
                                target_cost: program.target_cost,
                                current_progress: 0.0,
                                effect_kind_raw: program.effect_kind_raw,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// `(records, branch_ids, category_ids, program_ids, funding_level_counts)` - see
    /// `loaded_records_and_ids`.
    type LoadedRecordsAndIds = (Vec<SaveRecord>, Vec<i32>, Vec<i32>, Vec<i32>, std::collections::HashMap<i32, usize>);

    /// The `(kind, id, value)` records a save file would contain for `cases` (using each case's
    /// *saved_* fields, skipping ids with no override), plus every id/funding-level-count in the tree -
    /// exactly what `research_save_reimplementation::predict_load` needs to predict `load`'s outcome.
    fn loaded_records_and_ids(cases: &[BranchCase]) -> LoadedRecordsAndIds {
        let mut records = Vec::new();
        let mut branch_ids = Vec::new();
        let mut category_ids = Vec::new();
        let mut program_ids = Vec::new();
        let mut funding_level_counts = std::collections::HashMap::new();

        for branch in cases {
            branch_ids.push(branch.id);
            funding_level_counts.insert(branch.id, branch.funding_level_count);
            if let Some(current_funding_level) = branch.saved_funding_level {
                records.push(SaveRecord::Branch { id: branch.id, current_funding_level });
            }
            for category in &branch.categories {
                category_ids.push(category.id);
                if let Some(enabled) = category.saved_enabled {
                    records.push(SaveRecord::Category { id: category.id, enabled });
                }
                for program in &category.programs {
                    program_ids.push(program.id);
                    if let Some(current_progress_bits) = program.saved_progress_bits {
                        records.push(SaveRecord::Program { id: program.id, current_progress_bits });
                    }
                }
            }
        }

        (records, branch_ids, category_ids, program_ids, funding_level_counts)
    }

    // TODO: Fix this so it works with a crate/mod prefix
    #[detour(LOAD_LANG_DLLS)]
    unsafe extern "thiscall" fn detour_target(this: *const u32) -> u32 {
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
        fail_flag |= run_research_branch_funding_test(&mut failure_log);
        fail_flag |= run_research_branch_pct_days_remaining_test(&mut failure_log);
        fail_flag |= run_marketing_increase_funding_test(&mut failure_log);
        fail_flag |= run_marketing_decrease_funding_test(&mut failure_log);
        fail_flag |= run_marketing_set_funding_level_test(&mut failure_log);
        fail_flag |= run_marketingmgr_update_test(&mut failure_log);
        fail_flag |= run_marketingmgr_save_test(&mut failure_log);
        fail_flag |= run_marketingmgr_load_test(&mut failure_log);
        fail_flag |= run_marketingmgr_clear_configurations_test(&mut failure_log);

        fail_flag |= run_thoughtmgr_add_thought_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_remove_thoughts_by_thinker_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_remove_thoughts_by_object_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_remove_thoughts_by_habitat_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_get_thoughts_by_thinker_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_get_thoughts_by_object_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_get_thoughts_by_habitat_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_save_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_load_test(&mut failure_log);

        // ZTRESEARCHMGR_SAVE: compares the real ZTResearchMgr::save's captured output against
        // research_save_reimplementation::serialize(&snapshot_mgr(mgr)) for generated synthetic trees.
        {
            let test_name = "ZTRESEARCHMGR_SAVE";
            match runner.run(&prop::collection::vec(branch_case_strategy(), 0..4), |cases| {
                let branches = generated_branches(&cases);

                let dummy_file: u32 = 0;
                let (expected_records, captured_bytes) = live_support::with_standalone_mgr(&branches, |mgr| {
                    let expected_records = research_save_reimplementation::snapshot_mgr(mgr);
                    io_redirect::begin_capture();
                    let _ = mgr.save(&dummy_file as *const u32);
                    let captured_bytes = io_redirect::end_capture();
                    (expected_records, captured_bytes)
                });

                let expected_bytes = research_save_reimplementation::serialize(&expected_records);
                prop_assert_eq!(captured_bytes, expected_bytes, "ZTResearchMgr::save byte mismatch for cases: {:?}", cases);
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // ZTRESEARCHMGR_LOAD: compares the real ZTResearchMgr::load's effect on funding
        // levels/enabled flags/current progress against research_save_reimplementation::predict_load,
        // for generated synthetic trees, generated save-stream records, and versions spanning both
        // sides of the 0x28 threshold that gates whether the stream is read at all.
        {
            let test_name = "ZTRESEARCHMGR_LOAD";
            match runner.run(&(prop::collection::vec(branch_case_strategy(), 0..4), 0u32..0x40), |(cases, version)| {
                let branches = generated_branches_for_load(&cases);
                let (records, branch_ids, category_ids, program_ids, funding_level_counts) = loaded_records_and_ids(&cases);
                let predicted = research_save_reimplementation::predict_load(&branch_ids, &category_ids, &program_ids, &funding_level_counts, &records, version);
                let bytes = research_save_reimplementation::serialize(&records);

                // `load` dereferences the file pointer directly to check a CRT-`FILE`-shaped EOF flag
                // at offset 0xc, unlike `save` - a zeroed 16-byte buffer keeps that flag clear so the
                // redirected `deallocate` calls actually run.
                let file_buffer = [0u32; 4];
                let actual_records = live_support::with_standalone_mgr(&branches, |mgr| {
                    live_support::with_global_ztresearchmgr_ptr(mgr, |mgr| {
                        io_redirect::begin_replay(bytes);
                        let _ = mgr.load(file_buffer.as_ptr(), version);
                        io_redirect::end_replay();
                        research_save_reimplementation::snapshot_mgr(mgr)
                    })
                });

                let mut actual_funding = std::collections::HashMap::new();
                let mut actual_enabled = std::collections::HashMap::new();
                let mut actual_progress = std::collections::HashMap::new();
                for record in actual_records {
                    match record {
                        SaveRecord::Branch { id, current_funding_level } => {
                            actual_funding.insert(id, current_funding_level);
                        }
                        SaveRecord::Category { id, enabled } => {
                            actual_enabled.insert(id, enabled);
                        }
                        SaveRecord::Program { id, current_progress_bits } => {
                            actual_progress.insert(id, current_progress_bits);
                        }
                    }
                }

                prop_assert_eq!(actual_funding, predicted.funding_levels, "funding level mismatch for version {} cases: {:?}", version, cases);
                prop_assert_eq!(actual_enabled, predicted.enabled, "enabled mismatch for version {} cases: {:?}", version, cases);
                prop_assert_eq!(actual_progress, predicted.progress_bits, "progress mismatch for version {} cases: {:?}", version, cases);
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // ZTRESEARCHMGR_LOAD_CORRUPT_STREAM: exercises `load`'s corrupt-stream abort path - a
        // malformed `kind` tag (anything other than `0`/`1`/`2`/the `-1` terminator) partway through
        // the stream, which `detours::load`'s reading loop treats the same as a genuine I/O read
        // failure: `load` returns `false` without running the `on_completion`/`pick_random_program`
        // tail, but every record read *before* the corruption point has already been applied (per
        // `ZTResearchMgr_load.c`/`.asm`'s "any read failure aborts the whole load" semantics - not a
        // rollback of already-applied records). `SaveRecord`/`serialize` can only ever produce
        // well-formed `kind` tags, so this path has no coverage anywhere else in this file. `version`
        // is fixed `>= 0x28` (the threshold gating whether the stream is read at all - below it,
        // there's nothing to corrupt); `raw_truncate_at` is reduced modulo `records.len() + 1` to land
        // in `0..=records.len()`, choosing how many well-formed records precede the injected
        // corruption. The expected state is `predict_load` fed only the *prefix* of records (before the
        // corruption point) - the unconditional reset always applies, and every record before the
        // corruption point was already parsed and applied before the abort, so this needs no new
        // prediction logic.
        {
            let test_name = "ZTRESEARCHMGR_LOAD_CORRUPT_STREAM";
            match runner.run(
                &(prop::collection::vec(branch_case_strategy(), 0..4), 0x28u32..0x40u32, any::<usize>()),
                |(cases, version, raw_truncate_at)| {
                    let branches = generated_branches_for_load(&cases);
                    let (records, branch_ids, category_ids, program_ids, funding_level_counts) = loaded_records_and_ids(&cases);
                    let truncate_at = raw_truncate_at % (records.len() + 1);

                    // header + well-formed records[..truncate_at] + terminator, then drop that
                    // terminator and replace it with a malformed `kind` tag (`3`, not `0`/`1`/`2`/`-1`)
                    // plus a dummy id, so the reading loop actually reaches and executes its
                    // `kind > 2` check rather than just running out of bytes mid-read.
                    let mut bytes = research_save_reimplementation::serialize(&records[..truncate_at]);
                    bytes.truncate(bytes.len() - 4);
                    bytes.extend_from_slice(&3i32.to_le_bytes());
                    bytes.extend_from_slice(&0i32.to_le_bytes());

                    let predicted = research_save_reimplementation::predict_load(
                        &branch_ids,
                        &category_ids,
                        &program_ids,
                        &funding_level_counts,
                        &records[..truncate_at],
                        version,
                    );

                    let file_buffer = [0u32; 4];
                    let (load_result, actual_records) = live_support::with_standalone_mgr(&branches, |mgr| {
                        live_support::with_global_ztresearchmgr_ptr(mgr, |mgr| {
                            io_redirect::begin_replay(bytes);
                            let load_result = mgr.load(file_buffer.as_ptr(), version);
                            io_redirect::end_replay();
                            (load_result, research_save_reimplementation::snapshot_mgr(mgr))
                        })
                    });

                    prop_assert!(!load_result, "load() should return false on a corrupt stream, cases: {:?}, truncate_at: {}", cases, truncate_at);

                    let mut actual_funding = std::collections::HashMap::new();
                    let mut actual_enabled = std::collections::HashMap::new();
                    let mut actual_progress = std::collections::HashMap::new();
                    for record in actual_records {
                        match record {
                            SaveRecord::Branch { id, current_funding_level } => {
                                actual_funding.insert(id, current_funding_level);
                            }
                            SaveRecord::Category { id, enabled } => {
                                actual_enabled.insert(id, enabled);
                            }
                            SaveRecord::Program { id, current_progress_bits } => {
                                actual_progress.insert(id, current_progress_bits);
                            }
                        }
                    }

                    prop_assert_eq!(
                        actual_funding,
                        predicted.funding_levels,
                        "funding level mismatch for version {} truncate_at {} cases: {:?}",
                        version,
                        truncate_at,
                        cases
                    );
                    prop_assert_eq!(
                        actual_enabled,
                        predicted.enabled,
                        "enabled mismatch for version {} truncate_at {} cases: {:?}",
                        version,
                        truncate_at,
                        cases
                    );
                    prop_assert_eq!(
                        actual_progress,
                        predicted.progress_bits,
                        "progress mismatch for version {} truncate_at {} cases: {:?}",
                        version,
                        truncate_at,
                        cases
                    );
                    Ok(())
                },
            ) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // ZTRESEARCHMGR_UPDATE: compares the real ZTResearchMgr::update's effect on `elapsed_ticks`
        // against the reimplemented `ZTResearchMgr::update`, for a synthetic manager with zero
        // branches - so `ZTResearchBranch::update` (still a call into the original implementation)
        // never actually runs, keeping this comparison independent of branch-level state.
        {
            let test_name = "ZTRESEARCHMGR_UPDATE";
            match runner.run(&(any::<u32>(), any::<u32>()), |(elapsed_ticks_before, delta_ticks)| {
                let real_elapsed_ticks = live_support::with_standalone_mgr(&[], |mgr| {
                    mgr.set_elapsed_ticks(elapsed_ticks_before);
                    unsafe { ZTRESEARCHMGR_UPDATE.original()((mgr as *mut ZTResearchMgr) as *const u32, delta_ticks) };
                    mgr.elapsed_ticks()
                });
                let reimplemented_elapsed_ticks = live_support::with_standalone_mgr(&[], |mgr| {
                    mgr.set_elapsed_ticks(elapsed_ticks_before);
                    mgr.update(delta_ticks);
                    mgr.elapsed_ticks()
                });

                prop_assert_eq!(
                    real_elapsed_ticks,
                    reimplemented_elapsed_ticks,
                    "elapsed_ticks mismatch for elapsed_ticks_before={}, delta_ticks={}",
                    elapsed_ticks_before,
                    delta_ticks
                );
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // ZTRESEARCHMGR_FORCE_RESEARCH: compares the real ZTResearchMgr::forceResearch's effect on
        // every program's `current_progress` against the reimplemented `ZTResearchMgr::force_research`,
        // for generated synthetic trees with exactly one program per category (see
        // `force_research_generated_branches`'s doc comment) and both `continue_program` values. Every
        // generated program's `effect_kind_raw` is fixed to `-1` (unset) by `live_support::GeneratedProgram`,
        // making `on_completion` a guaranteed no-op that needs no `GLOBAL_ZTWorldMgr`, so - like SAVE/LOAD/
        // UPDATE above - this can run at this early injection point rather than waiting for `updateSim`.
        {
            let test_name = "ZTRESEARCHMGR_FORCE_RESEARCH";
            match runner.run(&(prop::collection::vec(force_research_branch_case_strategy(), 0..3), proptest::bool::ANY), |(cases, continue_program)| {
                let real_progress_bits = {
                    let branches = force_research_generated_branches(&cases);
                    live_support::with_standalone_mgr(&branches, |mgr| {
                        live_support::with_global_ztresearchmgr_ptr(mgr, |mgr| {
                            unsafe { ZTRESEARCHMGR_FORCE_RESEARCH.original()((mgr as *mut ZTResearchMgr) as *const u32, continue_program) };
                            mgr.branches().flat_map(|b| b.categories()).flat_map(|c| c.programs()).map(|p| p.current_progress().to_bits()).collect::<Vec<_>>()
                        })
                    })
                };

                let reimplemented_progress_bits = {
                    let branches = force_research_generated_branches(&cases);
                    live_support::with_standalone_mgr(&branches, |mgr| {
                        live_support::with_global_ztresearchmgr_ptr(mgr, |mgr| {
                            mgr.force_research(continue_program);
                            mgr.branches().flat_map(|b| b.categories()).flat_map(|c| c.programs()).map(|p| p.current_progress().to_bits()).collect::<Vec<_>>()
                        })
                    })
                };

                prop_assert_eq!(
                    real_progress_bits,
                    reimplemented_progress_bits,
                    "current_progress mismatch for continue_program={} cases: {:?}",
                    continue_program,
                    cases
                );
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // ZTRESEARCHMGR_LOOKUPS: compares the real ZTResearchMgr::getBranch/getCategory/getProgram
        // against the reimplemented get_branch/get_category/get_program, on one shared generated tree
        // (read-only, so - unlike SET_EFFECT_DISCOUNT below - both sides query the very same tree
        // instance, no need for two independently-built ones), with small overlapping ids (see
        // `lookup_branch_case_strategy`'s doc comment) to exercise duplicate/colliding-id traversal
        // order, plus a lookup id that may or may not be present in the tree.
        {
            let test_name = "ZTRESEARCHMGR_LOOKUPS";
            match runner.run(&(prop::collection::vec(lookup_branch_case_strategy(), 0..4), any::<i32>()), |(cases, lookup_id)| {
                let branches = lookup_generated_branches(&cases);
                let (real_branch, real_category, real_program, reimpl_branch, reimpl_category, reimpl_program) =
                    live_support::with_standalone_mgr(&branches, |mgr| {
                        let mgr_ptr = (mgr as *mut ZTResearchMgr) as *const u32;
                        let real_branch = unsafe { ztresearchmgr::GET_BRANCH.original()(mgr_ptr, lookup_id) } as u32;
                        let real_category = unsafe { ztresearchmgr::GET_CATEGORY.original()(mgr_ptr, lookup_id) } as u32;
                        let real_program = unsafe { ztresearchmgr::GET_PROGRAM.original()(mgr_ptr, lookup_id) } as u32;

                        let reimpl_branch = mgr.get_branch(lookup_id).map_or(0u32, |b| b as *const _ as u32);
                        let reimpl_category = mgr.get_category(lookup_id).map_or(0u32, |c| c as *const _ as u32);
                        let reimpl_program = mgr.get_program(lookup_id).map_or(0u32, |p| p as *const _ as u32);

                        (real_branch, real_category, real_program, reimpl_branch, reimpl_category, reimpl_program)
                    });

                prop_assert_eq!(real_branch, reimpl_branch, "get_branch mismatch for lookup_id={} cases: {:?}", lookup_id, cases);
                prop_assert_eq!(real_category, reimpl_category, "get_category mismatch for lookup_id={} cases: {:?}", lookup_id, cases);
                prop_assert_eq!(real_program, reimpl_program, "get_program mismatch for lookup_id={} cases: {:?}", lookup_id, cases);
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // ZTRESEARCHMGR_SET_EFFECT_DISCOUNT: compares the real ZTResearchMgr::setEffectDiscount's
        // effect on every program's `target_cost` against the reimplemented `set_effect_discount`, for
        // two independently-constructed but structurally identical trees
        // (`effect_discount_generated_branches` is called once per side) - mutating, unlike LOOKUPS
        // above, so each side needs its own tree rather than sharing one. `kind` spans every valid
        // `ZTResearchEffectKind`; each generated program's own `effect_kind_raw` spans `-1..=8` (see
        // `effect_discount_program_case_strategy`'s doc comment), covering both matching and
        // non-matching programs within the same tree; `discount_pct` spans below `0`, within `0..=100`,
        // and above `100` - the reimplementation applies the raw arithmetic unconditionally, with no
        // range clamp on either side.
        {
            let test_name = "ZTRESEARCHMGR_SET_EFFECT_DISCOUNT";
            match runner.run(
                &(prop::collection::vec(effect_discount_branch_case_strategy(), 0..3), 0i32..=7i32, -50i32..150i32),
                |(cases, kind_raw, discount_pct)| {
                    let kind = ZTResearchEffectKind::try_from(kind_raw).expect("kind_raw generated in 0..=7, always a valid ZTResearchEffectKind");

                    let real_target_cost_bits = {
                        let branches = effect_discount_generated_branches(&cases);
                        live_support::with_standalone_mgr(&branches, |mgr| {
                            unsafe { ztresearchmgr::SET_EFFECT_DISCOUNT.original()((mgr as *mut ZTResearchMgr) as *const u32, kind_raw, discount_pct) };
                            mgr.branches()
                                .flat_map(|b| b.categories())
                                .flat_map(|c| c.programs())
                                .map(|p| p.target_cost().to_bits())
                                .collect::<Vec<_>>()
                        })
                    };

                    let reimpl_target_cost_bits = {
                        let branches = effect_discount_generated_branches(&cases);
                        live_support::with_standalone_mgr(&branches, |mgr| {
                            mgr.set_effect_discount(kind, discount_pct);
                            mgr.branches()
                                .flat_map(|b| b.categories())
                                .flat_map(|c| c.programs())
                                .map(|p| p.target_cost().to_bits())
                                .collect::<Vec<_>>()
                        })
                    };

                    prop_assert_eq!(
                        real_target_cost_bits,
                        reimpl_target_cost_bits,
                        "target_cost mismatch for kind_raw={}, discount_pct={} cases: {:?}",
                        kind_raw,
                        discount_pct,
                        cases
                    );
                    Ok(())
                },
            ) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Passed {}\n", test_name).as_bytes());
                    }
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        // `ZTRESEARCHPROGRAM_ON_COMPLETION_RESET` needs `GLOBAL_ZTWorldMgr` initialized (every
        // underlying effect call it exercises walks its entity-type list), which - confirmed live via
        // the Lua console's `get_zt_world_mgr()` - isn't true yet at this injection point (this early,
        // `LOAD_LANG_DLLS` hasn't even loaded language DLLs yet) but *is* true by the time
        // `ZTApp::updateSim` starts ticking (entity types load during app init, before the main loop
        // starts). So: stash this battery's `fail_flag` and hand off to the real `LOAD_LANG_DLLS`
        // instead of exiting here, so the game actually continues init through to the main loop;
        // `detour_update_sim` below runs the research test on the first tick and does the final exit
        // for the whole combined battery.
        EARLY_TESTS_FAILED.store(fail_flag, Ordering::SeqCst);
        unsafe { LOAD_LANG_DLLS_DETOUR.call(this) }
    }

    static EARLY_TESTS_FAILED: AtomicBool = AtomicBool::new(false);
    static RAN_UPDATE_SIM_TESTS: Once = Once::new();

    /// The resource-relative path vanilla's own boot-time `ZTMarketingMgr::loadConfigurations` call
    /// passes (e.g. `"mktg.cfg"`) - captured below so `run_marketingmgr_load_configurations_test` can
    /// reuse the real path. Only the first call's path is kept.
    static CAPTURED_MARKETING_PATH: OnceLock<String> = OnceLock::new();

    /// Transparent path-capture detour on `ZTMarketingMgr::loadConfigurations` - always calls through
    /// to the original and never alters its return value or behavior, just records the path into
    /// `CAPTURED_MARKETING_PATH`.
    #[detour(LOAD_CONFIGURATIONS)]
    unsafe extern "thiscall" fn detour_capture_marketing_load_configurations_path(this: *const u32, path: *const i8) -> u32 {
        let path_str = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();
        let _ = CAPTURED_MARKETING_PATH.set(path_str);
        unsafe { LOAD_CONFIGURATIONS_DETOUR.call(this, path) }
    }

    #[detour(UPDATE_SIM)]
    unsafe extern "thiscall" fn detour_update_sim(this_ptr: *const u32, param_2: u32) {
        RAN_UPDATE_SIM_TESTS.call_once(run_on_completion_reset_test_and_exit);
        unsafe { UPDATE_SIM_DETOUR.call(this_ptr, param_2) }
    }

    /// Runs on `ZTApp::updateSim`'s first tick (see `detour_update_sim`), once `GLOBAL_ZTWorldMgr` is
    /// actually initialized: compares the real `ZTResearchProgram::onCompletion`/`reset` against
    /// `ztresearch::dispatch_on_completion`/`dispatch_reset` (via the public `on_completion()`/
    /// `reset()` wrappers) for every `effect_kind_raw` from `-1` (unset) through `8` (one past the
    /// last valid kind), on freestanding programs built with `live_support::build_standalone_program`
    /// (safe no-op target/entity ids - see its own doc comment). Only the low byte of the real return
    /// value is compared - the rest is undefined garbage left over in EAX from whatever vanilla called
    /// last (see `ZTResearchProgram::on_completion`'s doc comment). Appends to the same log file
    /// `detour_target` started, then performs the final exit for the whole combined battery.
    fn run_on_completion_reset_test_and_exit() {
        let failure_log_path =
            std::env::var("OPENZT_TEST_LOG").unwrap_or_else(|_| "C:\\Program Files (x86)\\Microsoft Games\\Zoo Tycoon\\openzt_test.log".to_string());
        let mut failure_log = match OpenOptions::new().create(true).append(true).open(&failure_log_path) {
            Ok(file) => Some(file),
            Err(e) => {
                error!("Failed to open failure log file '{}': {}", failure_log_path, e);
                None
            }
        };

        let mut fail_flag = EARLY_TESTS_FAILED.load(Ordering::SeqCst);

        let test_name = "ZTRESEARCHPROGRAM_ON_COMPLETION_RESET";
        if globals().ztworldmgr_ptr().is_null() {
            info!("Skipping {}: GLOBAL_ZTWorldMgr not initialized at this injection point", test_name);
            write_success_line(&mut failure_log, &format!("{} (skipped: ZTWorldMgr not initialized)", test_name));
        } else {
            let runner_config = ProptestConfig {
                failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
                ..ProptestConfig::default()
            };
            let mut runner = proptest::test_runner::TestRunner::new(runner_config);
            match runner.run(&(-1i32..=8i32), |effect_kind_raw| {
                let (real_on_completion, reimpl_on_completion, real_reset, reimpl_reset) = live_support::with_standalone_mgr(&[], |mgr| {
                    live_support::with_global_ztresearchmgr_ptr(mgr, |_mgr| {
                        let real_ptr = live_support::build_standalone_program(effect_kind_raw);
                        let reimpl_ptr = live_support::build_standalone_program(effect_kind_raw);

                        let real_on_completion = unsafe { ztresearchprogram::ON_COMPLETION.original()(real_ptr as *const u32) } as u32;
                        let reimpl_on_completion = unsafe { (*reimpl_ptr).on_completion() };

                        let real_reset = unsafe { ztresearchprogram::RESET.original()(real_ptr as *const u32) } as u32;
                        let reimpl_reset = unsafe { (*reimpl_ptr).reset() };

                        live_support::destroy_standalone_program(real_ptr);
                        live_support::destroy_standalone_program(reimpl_ptr);

                        (real_on_completion, reimpl_on_completion, real_reset, reimpl_reset)
                    })
                });

                prop_assert_eq!(real_on_completion, reimpl_on_completion, "on_completion mismatch for effect_kind_raw={}", effect_kind_raw);
                prop_assert_eq!(real_reset, reimpl_reset, "reset mismatch for effect_kind_raw={}", effect_kind_raw);
                Ok(())
            }) {
                Ok(_) => {
                    info!("Proptest passed for {}", test_name);
                    write_success_line(&mut failure_log, test_name);
                }
                Err(e) => {
                    error!("Proptest failed: {:?}", e);
                    if let Some(ref mut log_file) = failure_log {
                        let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                    }
                    fail_flag = true;
                }
            }
        }

        fail_flag |= run_funding_text_test(&mut failure_log);
        fail_flag |= run_branch_update_test(&mut failure_log);
        fail_flag |= run_branch_update_reimpl_boundary_test(&mut failure_log);
        fail_flag |= run_research_mgr_update_branches_test(&mut failure_log);
        fail_flag |= run_marketing_update_test(&mut failure_log);
        fail_flag |= run_marketing_update_boundary_test(&mut failure_log);
        fail_flag |= run_marketing_update_reimpl_boundary_test(&mut failure_log);
        fail_flag |= run_marketing_funding_text_test(&mut failure_log);
        fail_flag |= run_marketingmgr_load_configurations_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_load_modern_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_add_thought_animal_override_test(&mut failure_log);
        fail_flag |= run_thoughtmgr_populate_thoughts_test(&mut failure_log);
        fail_flag |= run_thought_get_string_test(&mut failure_log);

        // Loads a real save file directly, so GLOBAL_ZTWorldMgr/GLOBAL_ZTHabitatMgr go from
        // empty/synthetic to real, populated state. Everything below this line runs against that real
        // zoo instead of a standalone/synthetic struct.
        if run_load_live_zoo(&mut failure_log) {
            fail_flag |= run_habitat_get_habitat_ptr_live_test(&mut failure_log);
            // Risk-sequenced per ztmegatilemgr.rs's module doc comment: update() first (trivial scalar
            // logic), then recalculate_characteristics() (in-place map mutation, no vector resize), then
            // the category-map node-layout live check, then init() last (the only vector-resize path).
            // All four ran unconditionally in the default battery for the first time once two real bugs
            // were found and fixed - see `ztmegatilemgr-live-crash-investigation.md` for the full
            // history: (1) `outer_vector_erase`/`outer_vector_insert_n` were passing the wrong `this`
            // pointer (the `ZTMegatileMgr*` itself instead of `&mgr->row_start`, the embedded vector
            // header's real address - this was the original crash, previously misdiagnosed as a vanilla
            // exception-filter/uninitialized-global issue), and (2) `init()`'s per-column grow path
            // needed a real, correctly-shaped empty-tree sentinel (`parent: null`, `left`/`right`
            // self-referential) for the fill value's `category_map.head`, not a null or dummy pointer -
            // see `empty_category_map_sentinel`'s own doc comment in `ztmegatilemgr.rs`.
            fail_flag |= run_megatilemgr_update_test(&mut failure_log);
            fail_flag |= run_megatilemgr_recalculate_characteristics_test(&mut failure_log);
            fail_flag |= run_megatile_category_map_layout_test(&mut failure_log);
            fail_flag |= run_megatilemgr_init_test(&mut failure_log);
        }

        if fail_flag {
            error!("Proptest failed for some cases, check the failure log at: {}", failure_log_path);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    /// Default path for `run_load_live_zoo`'s save file - a real save (not embedded/synthetic) placed
    /// in the actual Zoo Tycoon "Saved Games" directory, overridable via `OPENZT_TEST_ZOO` for anyone
    /// whose install lives elsewhere.
    const DEFAULT_TEST_ZOO_PATH: &str = r"C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\Saved Games\reimplementation-test-zoo.zoo";

    /// Loads `OPENZT_TEST_ZOO` (or `DEFAULT_TEST_ZOO_PATH`) into the running game, bringing up a real,
    /// fully-populated `GLOBAL_ZTWorldMgr`/`GLOBAL_ZTHabitatMgr`/etc. - unlike every test above this
    /// point in the battery, which only ever build standalone/synthetic structs.
    ///
    /// Calls `FOPEN`/`ZTUI_GAMEOPTS_LOAD_FILE`/`FCLOSE` directly - the same primitives
    /// `ZTUI::gameopts::loadGame`/`ZTUI::clickContinue` use, minus the file-picker dialog and UI click
    /// handlers, neither of which touches `GLOBAL_ZTWorldMgr`/`GLOBAL_ZTHabitatMgr`.
    ///
    /// Returns `true` only on a real load success (`LOAD_FILE`'s low byte non-zero).
    fn run_load_live_zoo(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "LOAD_LIVE_ZOO";
        let path = std::env::var("OPENZT_TEST_ZOO").unwrap_or_else(|_| DEFAULT_TEST_ZOO_PATH.to_string());

        let path_cstring = match CString::new(path.clone()) {
            Ok(c) => c,
            Err(e) => {
                error!("{}: path {:?} contains a NUL byte: {}", test_name, path, e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: path {:?} contains a NUL byte: {}\n", test_name, path, e).as_bytes());
                }
                return false;
            }
        };
        let mode_cstring = c"rb";

        let file_ptr = unsafe { standalone::FOPEN.original()(path_cstring.as_ptr(), mode_cstring.as_ptr()) };
        if file_ptr.is_null() {
            error!("{}: fopen failed for {:?} (file missing? see OPENZT_TEST_ZOO)", test_name, path);
            if let Some(log_file) = failure_log {
                let _ = log_file.write_all(format!("Test Failed {}: fopen failed for {:?}\n", test_name, path).as_bytes());
            }
            return false;
        }

        let load_result = unsafe { ZTUI_GAMEOPTS_LOAD_FILE.original()(file_ptr as *const u8) };
        unsafe { standalone::FCLOSE.original()(file_ptr) };

        let success = (load_result & 0xff) != 0;
        if success {
            info!("{}: loaded {:?}", test_name, path);
            write_success_line(failure_log, test_name);
        } else {
            error!("{}: LOAD_FILE reported failure (raw result {:#010x}) for {:?}", test_name, load_result, path);
            if let Some(log_file) = failure_log {
                let _ = log_file.write_all(format!("Test Failed {}: LOAD_FILE reported failure (raw result {:#010x}) for {:?}\n", test_name, load_result, path).as_bytes());
            }
        }
        success
    }

    /// Compares the real `ZTHabitatMgr::getHabitat` against the reimplemented `get_habitat_ptr`, for
    /// small in-range tile coordinates, now that `run_load_live_zoo` has populated
    /// `other_array_start`/`other_array_end` with a real zoo's bounds (`get_habitat_ptr` does no
    /// bounds-checking of its own, so this needs a real, loaded zoo to be safe).
    fn run_habitat_get_habitat_ptr_live_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTHABITATMGR_GET_HABITAT_PTR_LIVE";
        let mgr_ptr = globals().zthabitatmgr_ptr() as *const u32;
        let habitat_mgr = globals().zthabitatmgr();

        let mut fail_flag = false;
        for x in 0..5i32 {
            for y in 0..5i32 {
                let real = unsafe { zthabitatmgr::GET_HABITAT.original()(mgr_ptr, x, y) };
                let reimpl = habitat_mgr.get_habitat_ptr(x, y);
                if real != reimpl {
                    error!("{}: mismatch at ({}, {}): real={:#010x}, reimpl={:#010x}", test_name, x, y, real, reimpl);
                    if let Some(log_file) = failure_log {
                        let _ = log_file.write_all(
                            format!("Test Failed {}: mismatch at ({}, {}): real={:#010x}, reimpl={:#010x}\n", test_name, x, y, real, reimpl).as_bytes(),
                        );
                    }
                    fail_flag = true;
                }
            }
        }

        if !fail_flag {
            write_success_line(failure_log, test_name);
        }
        fail_flag
    }

    /// ZTMEGATILEMGR_UPDATE: compares the real `ZTMegatileMgr::update`'s effect on
    /// `dirty`/`tick_accumulator` against the reimplemented `update`, for `delta_ticks` below the
    /// `0x1d4b` threshold only. Snapshots the live singleton's scalars, calls the real
    /// `UPDATE.original()`, records the result as expected, restores the snapshot (via
    /// `megatile_live_support::restore_scalars`), calls the reimplemented `update()`, and compares.
    /// Runs after `run_load_live_zoo` so the live singleton is a real, populated grid - see
    /// `ztmegatilemgr.rs`'s own module doc comment for why this is implemented/tested first.
    ///
    /// Deliberately never crosses the threshold here: doing so calls through to
    /// `recalculate_characteristics` as a side effect (real or reimplemented, either way), which is
    /// exercised directly and more thoroughly by `run_megatilemgr_recalculate_characteristics_test`
    /// below - no need for this test to also trigger it as a side effect. The threshold/dirty-flag
    /// transition logic itself is still fully covered by `ztmegatilemgr::tests::update_state_*` (pure,
    /// no live memory touched).
    fn run_megatilemgr_update_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMEGATILEMGR_UPDATE";
        let mgr_ptr = globals().ztmegatilemgr_ptr();
        if mgr_ptr.is_null() {
            info!("Skipping {}: GLOBAL_ZTMegatileMgr not initialized", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTMegatileMgr not initialized)", test_name));
            return false;
        }
        let real_this = mgr_ptr as *const u32;

        let runner_config = ProptestConfig { failure_persistence: Some(Box::new(super::NoopFailurePersistence)), ..ProptestConfig::default() };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let mut fail_flag = false;
        match runner.run(&(0u32..0x1000u32), |delta_ticks| {
            let mgr = unsafe { &mut *mgr_ptr };
            // Force a known-safe starting accumulator (not just restore the pre-call value) - the live
            // singleton's own accumulator may already be close to the threshold from real game ticks
            // that ran before this test, and `delta_ticks + before_accumulator` crossing `0x1d4b` would
            // hit the same live recalc crash this test exists to avoid (see the doc comment above).
            megatile_live_support::restore_scalars(mgr, false, 0);
            let before_dirty = false;
            let before_accumulator = 0u32;

            unsafe { gen_ztmegatilemgr::UPDATE.original()(real_this, delta_ticks as i32) };
            let expected_dirty = mgr.is_dirty();
            let expected_accumulator = mgr.tick_accumulator();

            megatile_live_support::restore_scalars(mgr, before_dirty, before_accumulator);
            mgr.update(delta_ticks);
            let reimpl_dirty = mgr.is_dirty();
            let reimpl_accumulator = mgr.tick_accumulator();

            // A recalculation triggered by either side (real or reimplemented) already leaves the grid
            // in a self-consistent state per its own logic - `ZTMEGATILEMGR_RECALCULATE_CHARACTERISTICS`
            // is the dedicated test for whether that recalculation itself matches, so this test only
            // compares the scalars `update` itself owns.
            prop_assert_eq!(reimpl_dirty, expected_dirty, "dirty mismatch for delta_ticks={}", delta_ticks);
            prop_assert_eq!(reimpl_accumulator, expected_accumulator, "tick_accumulator mismatch for delta_ticks={}", delta_ticks);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }
        fail_flag
    }

    /// Compares two megatile-grid snapshots allowing a small tolerance on the float fields
    /// (`esthetic_bonus`) - real vanilla x87 arithmetic and Rust's SSE2 `f32` arithmetic can differ in
    /// the last bit or two despite following the same formula, which isn't a meaningful mismatch for
    /// this test. `guest_count` (an integer accumulation) is still compared exactly.
    fn grids_approximately_equal(expected: &megatile_live_support::GridSnapshot, actual: &megatile_live_support::GridSnapshot) -> bool {
        if expected.columns.len() != actual.columns.len() {
            return false;
        }
        expected.columns.iter().zip(actual.columns.iter()).all(|(e_col, a_col)| {
            e_col.len() == a_col.len()
                && e_col.iter().zip(a_col.iter()).all(|(&(e_guests, e_bonus), &(a_guests, a_bonus))| e_guests == a_guests && (e_bonus - a_bonus).abs() < 0.01)
        })
    }

    /// ZTMEGATILEMGR_RECALCULATE_CHARACTERISTICS: compares the real
    /// `ZTMegatileMgr::recalculateCharacteristics`'s effect on the full megatile grid against the
    /// reimplemented `recalculate_characteristics`. Recalculation is a pure function of live world state
    /// (every field is zeroed then recomputed from scratch each call - see that method's own doc
    /// comment), so this simply runs the real call, snapshots as expected, runs the reimplemented call
    /// on top, and snapshots again - no restore needed.
    ///
    /// This test used to crash reliably (misdiagnosed at the time as a vanilla exception-filter /
    /// uninitialized-global issue inside an `isKindOf`-style vtable function). The real cause turned out
    /// to be entirely on this side: `outer_vector_erase`/`outer_vector_insert_n` in `ztmegatilemgr.rs`
    /// were passing the wrong `this` pointer to vanilla's own vector-resize helpers (the
    /// `ZTMegatileMgr*` itself instead of `&mgr->row_start`, where the embedded `vector<MegatileRow>`
    /// header actually lives), corrupting the manager's own `vtable`/`flag`/`dirty`/`tick_accumulator`
    /// fields instead of the vector header - see `ztmegatilemgr-live-crash-investigation.md` for the
    /// full history and `ztmegatilemgr-review-findings.md` for the original review that flagged it.
    /// Fixed, this test (and `ZTMEGATILE_CATEGORY_MAP_LAYOUT`/`ZTMEGATILEMGR_INIT` below) now passes
    /// live. Two smaller real bugs were also found and fixed along the way: `bfcategory::GET_VALUE`
    /// needed `this = entity_type_ptr + 0x154` (not `entity_type_ptr` itself) and its category id
    /// argument by value (not by pointer) - both confirmed directly from
    /// `ZTMegatileMgr_recalculateCharacteristics.asm`, see `recalculate_characteristics`'s own call site.
    fn run_megatilemgr_recalculate_characteristics_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMEGATILEMGR_RECALCULATE_CHARACTERISTICS";
        let mgr_ptr = globals().ztmegatilemgr_ptr();
        if mgr_ptr.is_null() {
            write_success_line(failure_log, &format!("{} (skipped: ZTMegatileMgr not initialized)", test_name));
            return false;
        }
        let mgr = unsafe { &mut *mgr_ptr };
        let real_this = mgr_ptr as *const u32;

        unsafe { gen_ztmegatilemgr::RECALCULATE_CHARACTERISTICS.original()(real_this) };
        let expected = megatile_live_support::snapshot_grid(mgr);

        mgr.recalculate_characteristics();
        let actual = megatile_live_support::snapshot_grid(mgr);

        if grids_approximately_equal(&expected, &actual) {
            write_success_line(failure_log, test_name);
            false
        } else {
            error!("{}: grid mismatch after recalculate_characteristics", test_name);
            if let Some(log_file) = failure_log {
                let _ = log_file.write_all(format!("Test Failed {}: grid mismatch\n", test_name).as_bytes());
            }
            true
        }
    }

    /// ZTMEGATILE_CATEGORY_MAP_LAYOUT: validates the reconstructed `MapHeader`/`TreeNode` layout itself,
    /// independent of the tests above. After a real `RECALCULATE_CHARACTERISTICS.original()` call on the
    /// live singleton, walks every populated `ZTMegatile`'s `category_map` via `category_value()` for
    /// every key in the observed range `0x251f..0x2523` (9503-9506) and asserts every returned value is
    /// finite. `category_value`'s own step cap turns a wrong left/right offset guess into a graceful
    /// `None` rather than a live hang - see its doc comment - so this test's real signal is "did any
    /// value come back non-finite", which would mean the walk landed somewhere nonsensical.
    fn run_megatile_category_map_layout_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMEGATILE_CATEGORY_MAP_LAYOUT";
        let mgr_ptr = globals().ztmegatilemgr_ptr();
        if mgr_ptr.is_null() {
            write_success_line(failure_log, &format!("{} (skipped: ZTMegatileMgr not initialized)", test_name));
            return false;
        }
        let mgr = unsafe { &*mgr_ptr };
        let real_this = mgr_ptr as *const u32;
        unsafe { gen_ztmegatilemgr::RECALCULATE_CHARACTERISTICS.original()(real_this) };

        let keys: Vec<i32> = (0x251fi32..0x2523).collect();
        let mut fail_flag = false;
        for column in 0..mgr.megatile_columns() {
            for row in 0..mgr.megatile_rows_in_column(column) {
                let Some(mt) = mgr.megatile(column, row) else {
                    continue;
                };
                for &key in &keys {
                    if let Some(v) = mt.category_value(key) {
                        if !v.is_finite() {
                            error!("{}: non-finite value {} for key {:#x} at column={}, row={}", test_name, v, key, column, row);
                            fail_flag = true;
                        }
                    }
                }
            }
        }

        if !fail_flag {
            write_success_line(failure_log, test_name);
        } else if let Some(log_file) = failure_log {
            let _ = log_file.write_all(format!("Test Failed {}\n", test_name).as_bytes());
        }
        fail_flag
    }

    /// ZTMEGATILEMGR_INIT: run last, and only after the other three are passing - `init()` resizes the
    /// outer/inner vectors (the actual allocation-adjacent call), the single highest-risk piece in
    /// `ztmegatilemgr.rs`. For a few small tile-count targets (both shrinking and growing relative to the
    /// live map's own size), calls the real `INIT.original()` to capture the resulting grid dimensions,
    /// resets to a different size, then calls the reimplemented `init()` for the same target and compares
    /// dimensions. Restores the live singleton to the real map's own dimensions afterward regardless of
    /// outcome, via the real vanilla `init`/`recalculateCharacteristics`, so nothing later depends on
    /// whatever the last reimplemented call left behind.
    fn run_megatilemgr_init_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMEGATILEMGR_INIT";
        let mgr_ptr = globals().ztmegatilemgr_ptr();
        if mgr_ptr.is_null() {
            write_success_line(failure_log, &format!("{} (skipped: ZTMegatileMgr not initialized)", test_name));
            return false;
        }
        let mgr = unsafe { &mut *mgr_ptr };
        let real_this = mgr_ptr as *const u32;

        let world = globals().ztworldmgr();
        let original_x = world.map_x_size as i32;
        let original_y = world.map_y_size as i32;

        let mut fail_flag = false;
        for &(x, y) in &[(3i32, 3i32), (8i32, 8i32), (original_x, original_y)] {
            unsafe { gen_ztmegatilemgr::INIT.original()(real_this, x as u32, y as u32) };
            let expected_columns = mgr.megatile_columns();
            let expected_rows: Vec<usize> = (0..expected_columns).map(|c| mgr.megatile_rows_in_column(c)).collect();

            unsafe { gen_ztmegatilemgr::INIT.original()(real_this, 1, 1) };
            mgr.init(x, y);
            let actual_columns = mgr.megatile_columns();
            let actual_rows: Vec<usize> = (0..actual_columns).map(|c| mgr.megatile_rows_in_column(c)).collect();

            if actual_columns != expected_columns || actual_rows != expected_rows {
                error!(
                    "{}: dimension mismatch for ({}, {}): expected {} columns {:?}, got {} columns {:?}",
                    test_name, x, y, expected_columns, expected_rows, actual_columns, actual_rows
                );
                fail_flag = true;
            }
        }

        // Restore real state regardless of outcome.
        unsafe { gen_ztmegatilemgr::INIT.original()(real_this, original_x as u32, original_y as u32) };
        unsafe { gen_ztmegatilemgr::RECALCULATE_CHARACTERISTICS.original()(real_this) };

        if !fail_flag {
            write_success_line(failure_log, test_name);
        } else if let Some(log_file) = failure_log {
            let _ = log_file.write_all(format!("Test Failed {}\n", test_name).as_bytes());
        }
        fail_flag
    }

    /// One funding-level entry: `name_id` is always one of the real, known-valid `research.ini`
    /// funding-level name string ids (`23100`/`23101`/`23102`/`23103` - `"%s none"`/`"%s min"`/
    /// `"%s normal"`/`"%s max"`, per `ZTResearchFundingLevel::name`'s own doc comment) so the `%s`
    /// substitution both sides perform has a real, resolvable template to work with - this test runs
    /// at the `updateSim` first-tick injection point specifically so those ids are actually resolvable
    /// (language DLLs load during app init, same reasoning as `ZTRESEARCHPROGRAM_ON_COMPLETION_RESET`
    /// above). `cost` is bounded well away from `f32`'s extremes - `funding_text` casts
    /// `cost * (1.0/30.0)` to `i32` after rounding, and vanilla's own float-to-int conversion is
    /// undefined for non-finite/out-of-i32-range inputs, which isn't a meaningful case to compare.
    fn funding_level_case_strategy() -> impl Strategy<Value = (i32, f32)> {
        (prop_oneof![Just(23100i32), Just(23101i32), Just(23102i32), Just(23103i32)], -1_000_000f32..1_000_000f32)
    }

    /// ZTRESEARCHBRANCH_FUNDING_TEXT: compares the real `ZTResearchBranch::getFundingText`'s output
    /// against the reimplemented `ZTResearchBranch::funding_text`, for a standalone branch (not
    /// spliced into any `ZTResearchMgr` - see `live_support::build_standalone_funding_branch`'s doc
    /// comment) with a generated funding table and `current_funding_level` spanning negative/in-range/
    /// out-of-range relative to the table's length, to exercise the "no active level" empty-string path
    /// alongside the normal formatted-text path.
    fn run_funding_text_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTRESEARCHBRANCH_FUNDING_TEXT";
        let mut fail_flag = false;

        match runner.run(&(-2i32..4i32, prop::collection::vec(funding_level_case_strategy(), 0..3)), |(current_funding_level, levels)| {
            let branch_ptr = live_support::build_standalone_funding_branch(current_funding_level, &levels);

            let mut buffer = [0u32; 3];
            unsafe {
                ZTRESEARCHBRANCH_GET_FUNDING_TEXT.original()(branch_ptr as *const u32, buffer.as_mut_ptr() as *const u32);
            }
            let real_text = get_from_memory::<ZTBufferString>(buffer.as_ptr() as u32).copy_to_string();
            let branch: &ZTResearchBranch = unsafe { &*branch_ptr };
            let reimpl_text = branch.funding_text();

            live_support::destroy_standalone_funding_branch(branch_ptr);

            prop_assert_eq!(
                real_text,
                reimpl_text,
                "funding_text mismatch for current_funding_level={}, levels={:?}",
                current_funding_level,
                levels
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTRESEARCHBRANCH_UPDATE: compares the real `ZTResearchBranch::update`'s progress effect on the
    /// currently-selected program against the reimplemented `update`, for a single synthetic
    /// branch/category/program/funding-level built via `live_support::build_update_test_branch`.
    /// Restricted to the non-completing case - `TARGET_COST` is fixed far above any possible
    /// `progress_delta` for the generated `days`/`funding_rate` ranges (max ~23, per
    /// `predict_branch_progress`'s scale constant), so `on_completion`/`pick_random_program`/UI
    /// (covered separately by the `ZTRESEARCHPROGRAM_ON_COMPLETION_RESET`/`ZTRESEARCHMGR_FORCE_RESEARCH`
    /// tests) never actually run on either side.
    ///
    /// `available_cash` is generated as `cash_delta * cash_multiplier` for `cash_multiplier` in
    /// `0.0..2.0` (`cash_delta` computed from the same generated `days`/`funding_cost` via
    /// `predict_branch_progress(.., f32::MAX).0`), so roughly half of generated cases land unaffordable
    /// and half affordable - exercising both `ZTGameMgr::subtractCash`/`subtract_cash` on the real and
    /// reimplemented sides. This used to be restricted to the *insufficient-cash* case only, because of
    /// a real bug: `openzt-detour/src/generated.rs`'s `SUBTRACT_CASH` `FunctionDef` declared one `f32`
    /// stack arg, but the real `ZTGameMgr::subtractCash` takes `(f32, bool)` per its `.asm`'s `RET 8` -
    /// a 4-byte stack imbalance on every `.original()` call. That's now fixed (see
    /// `ztmarketing-update-setmoneytext-crash-investigation.md`'s "Resolution" section), so the
    /// affordable branch is safe to exercise here too. The exact `available_cash == cash_delta` boundary
    /// is separately covered deterministically by `run_branch_update_reimpl_boundary_test` below.
    fn run_branch_update_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTRESEARCHBRANCH_UPDATE";
        let mut fail_flag = false;

        if live_support::ztgamemgr_ptr_is_null() {
            info!("Skipping {}: GLOBAL_ZTGameMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTGameMgr not initialized)", test_name));
            return fail_flag;
        }

        const TARGET_COST: f32 = 1_000_000.0;

        match runner.run(
            &(1u32..1000u32, -1000f32..1000f32, 1f32..1000f32, 0f32..1000f32, 0.0f32..2.0f32),
            |(days, funding_rate, funding_cost, initial_progress, cash_multiplier)| {
                let (cash_delta, _) = predict_branch_progress(days, funding_cost, funding_rate, f32::MAX);
                let available_cash = (cash_delta * cash_multiplier).max(0.0);

                let real_progress = live_support::with_update_test_branch(TARGET_COST, initial_progress, funding_rate, funding_cost, |mgr| {
                    let branch = mgr.branch_mut(0);
                    live_support::with_ztgamemgr_cash(available_cash, || unsafe {
                        ZTRESEARCHBRANCH_UPDATE.original()((branch as *mut ZTResearchBranch) as *const u32, days);
                    });
                    branch.current_program().map(|p| p.current_progress())
                });

                let reimpl_progress = live_support::with_update_test_branch(TARGET_COST, initial_progress, funding_rate, funding_cost, |mgr| {
                    let branch = mgr.branch_mut(0);
                    live_support::with_ztgamemgr_cash(available_cash, || branch.update(days));
                    branch.current_program().map(|p| p.current_progress())
                });

                prop_assert_eq!(
                    real_progress,
                    reimpl_progress,
                    "current_progress mismatch for days={}, funding_rate={}, funding_cost={}, initial_progress={}, available_cash={}",
                    days,
                    funding_rate,
                    funding_cost,
                    initial_progress,
                    available_cash
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
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// Deterministic single-case regression for the *reimplemented* side of `ZTRESEARCHBRANCH_UPDATE`'s
    /// affordable branch - the actual previously-buggy path (`ZTResearchBranch::update` ->
    /// `ZTGameMgr::spend_research`/`subtract_cash`, routed through our own `SPEND_RESEARCH`/`SUBTRACT_CASH`
    /// `FunctionDef`s). Mirrors `run_marketing_update_reimpl_boundary_test`'s shape/rationale - see
    /// `ztmarketing-update-setmoneytext-crash-investigation.md`'s "Resolution" section and "Suggested next
    /// steps" item 7.
    ///
    /// Calls only the reimplemented `ZTResearchBranch::update`, skipping `.original()` entirely. `TARGET_COST`
    /// stays fixed far above any possible progress delta (same rationale as `run_branch_update_test` above), so
    /// completion/`on_completion`/UI never runs here - this is narrowly about the `subtract_cash` boundary call
    /// surviving. `available_cash` is pinned to exactly `cash_delta`, forcing the affordable branch.
    fn run_branch_update_reimpl_boundary_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTRESEARCHBRANCH_UPDATE_REIMPL_BOUNDARY_REPRO";

        if live_support::ztgamemgr_ptr_is_null() {
            info!("Skipping {}: GLOBAL_ZTGameMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTGameMgr not initialized)", test_name));
            return false;
        }

        const TARGET_COST: f32 = 1_000_000.0;
        let days: u32 = 5;
        let funding_rate: f32 = 30.0;
        let funding_cost: f32 = 100.0;
        let initial_progress: f32 = 0.0;
        let (cash_delta, _) = predict_branch_progress(days, funding_cost, funding_rate, f32::MAX);

        live_support::with_update_test_branch(TARGET_COST, initial_progress, funding_rate, funding_cost, |mgr| {
            let branch = mgr.branch_mut(0);
            info!("{}: about to call reimplemented ZTResearchBranch::update with available_cash == cash_delta ({})", test_name, cash_delta);
            live_support::with_ztgamemgr_cash(cash_delta, || branch.update(days));
            info!("{}: reimplemented call returned without crashing", test_name);
        });

        write_success_line(failure_log, test_name);
        false
    }

    /// ZTRESEARCHMGR_UPDATE (branch-count extension): compares the real `ZTResearchMgr::update`'s
    /// effect on `elapsed_ticks` and every branch's currently-selected program's `current_progress`
    /// against the reimplemented `update`, for 1-3 synthetic branches built via
    /// `live_support::with_update_test_branches`. The zero-branch `ZTRESEARCHMGR_UPDATE` test above
    /// only exercises `elapsed_ticks`' accumulator/day-count bookkeeping in isolation - this is the
    /// first test that actually exercises `ZTResearchMgr::update` iterating multiple branches and
    /// threading the correct `days` count to each (via `ZTResearchBranch::update`, native since Phase F).
    ///
    /// `target_cost` stays fixed far above any possible progress delta, same rationale as
    /// `run_branch_update_test` above, so `on_completion`/`pick_random_program`/UI never run on either
    /// side. Unlike that test, cash affordability here is drawn from one *shared* pool across every
    /// branch in a call: `ZTResearchMgr::update` (ztresearch.rs) iterates branches sequentially, and
    /// each `ZTResearchBranch::update` reads/spends the real, shared `GLOBAL_ZTGameMgr` cash fresh - so
    /// cash genuinely depletes across branches within one call, not per-branch. `available_cash` is
    /// `total_cash_delta * cash_multiplier` (`cash_multiplier` in `0.0..2.0`), where `total_cash_delta`
    /// sums every branch's own `cash_delta` (via `predict_branch_progress(.., f32::MAX).0`) for the
    /// shared `days` count the whole call receives. A multiplier below `1.0` naturally produces
    /// "prefix affordable, suffix not" cases as an earlier branch exhausts the shared pool - exercising
    /// real sequential depletion, not just per-branch affordability in isolation.
    ///
    /// This used to be restricted to the *insufficient-cash* case only (`AVAILABLE_CASH = 0.0`,
    /// `funding_rate` fixed inert at `0.0`), because of a real bug: `openzt-detour/src/generated.rs`'s
    /// `SUBTRACT_CASH` `FunctionDef` declared one `f32` stack arg, but the real
    /// `ZTGameMgr::subtractCash` takes `(f32, bool)` per its `.asm`'s `RET 8` - a 4-byte stack imbalance
    /// on every `.original()` call. That's now fixed (see
    /// `ztmarketing-update-setmoneytext-crash-investigation.md`'s "Resolution" section), so
    /// `funding_rate` is now generated too - a fixed `0.0` would leave `current_progress` trivially
    /// unchanged regardless of whether the affordable-branch math is right, silently defeating the
    /// comparison now that cash is sometimes affordable.
    fn run_research_mgr_update_branches_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTRESEARCHMGR_UPDATE_BRANCHES";
        let mut fail_flag = false;

        if live_support::ztgamemgr_ptr_is_null() {
            info!("Skipping {}: GLOBAL_ZTGameMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTGameMgr not initialized)", test_name));
            return fail_flag;
        }

        const TARGET_COST: f32 = 1_000_000.0;

        let branch_spec_strategy = (-1000f32..1000f32, 1f32..1000f32, 0f32..1000f32).prop_map(|(funding_rate, funding_cost, initial_progress)| {
            live_support::UpdateTestBranchSpec { target_cost: TARGET_COST, initial_progress, funding_rate, funding_cost }
        });

        match runner.run(
            &(any::<u32>(), any::<u32>(), prop::collection::vec(branch_spec_strategy, 1..4), 0.0f32..2.0f32),
            |(elapsed_ticks_before, delta_ticks, branch_specs, cash_multiplier)| {
                let (_, days) = predict_update(elapsed_ticks_before, delta_ticks);
                let total_cash_delta: f32 =
                    branch_specs.iter().map(|s| predict_branch_progress(days, s.funding_cost, s.funding_rate, f32::MAX).0).sum();
                let available_cash = (total_cash_delta * cash_multiplier).max(0.0);

                let (real_elapsed_ticks, real_progress_bits) = live_support::with_update_test_branches(&branch_specs, |mgr| {
                    mgr.set_elapsed_ticks(elapsed_ticks_before);
                    live_support::with_ztgamemgr_cash(available_cash, || unsafe {
                        ZTRESEARCHMGR_UPDATE.original()((mgr as *mut ZTResearchMgr) as *const u32, delta_ticks);
                    });
                    let progress_bits =
                        mgr.branches().flat_map(|b| b.current_program()).map(|p| p.current_progress().to_bits()).collect::<Vec<_>>();
                    (mgr.elapsed_ticks(), progress_bits)
                });

                let (reimpl_elapsed_ticks, reimpl_progress_bits) = live_support::with_update_test_branches(&branch_specs, |mgr| {
                    mgr.set_elapsed_ticks(elapsed_ticks_before);
                    live_support::with_ztgamemgr_cash(available_cash, || mgr.update(delta_ticks));
                    let progress_bits =
                        mgr.branches().flat_map(|b| b.current_program()).map(|p| p.current_progress().to_bits()).collect::<Vec<_>>();
                    (mgr.elapsed_ticks(), progress_bits)
                });

                prop_assert_eq!(
                    real_elapsed_ticks,
                    reimpl_elapsed_ticks,
                    "elapsed_ticks mismatch for elapsed_ticks_before={}, delta_ticks={}, branch_count={}",
                    elapsed_ticks_before,
                    delta_ticks,
                    branch_specs.len()
                );
                prop_assert_eq!(
                    real_progress_bits,
                    reimpl_progress_bits,
                    "current_progress mismatch for elapsed_ticks_before={}, delta_ticks={}, branch_count={}",
                    elapsed_ticks_before,
                    delta_ticks,
                    branch_specs.len()
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
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
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

    /// ZTRESEARCHBRANCH_FUNDING: compares the real `ZTResearchBranch::increaseFunding`/
    /// `decreaseFunding` against the reimplemented `increase_funding`/`decrease_funding`, for two
    /// independently-constructed but structurally identical standalone branches (built via
    /// `live_support::build_standalone_funding_branch`, same as `ZTRESEARCHBRANCH_FUNDING_TEXT` -
    /// mutating `current_funding_level`, so each side needs its own branch rather than sharing one).
    /// Only the funding table's *length* matters here (`increase_funding`/`decrease_funding` never read
    /// an entry's own content, only `funding_level_count()`), so every generated entry is a dummy
    /// `(0, 0.0)`. `current_funding_level` spans `-2..4` to cover negative/in-range/top-of-range/
    /// one-past-the-end starting values against funding tables spanning empty (`0`) through a few
    /// entries, for both `increase_funding` and `decrease_funding`.
    fn run_research_branch_funding_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTRESEARCHBRANCH_FUNDING";
        let mut fail_flag = false;

        match runner.run(&(-2i32..4i32, 0usize..5, proptest::bool::ANY), |(current_funding_level, level_count, increase)| {
            let levels = vec![(0i32, 0.0f32); level_count];

            let real_branch_ptr = live_support::build_standalone_funding_branch(current_funding_level, &levels);
            if increase {
                unsafe { ztresearchbranch::INCREASE_FUNDING.original()(real_branch_ptr as *const u32) };
            } else {
                unsafe { ztresearchbranch::DECREASE_FUNDING.original()(real_branch_ptr as *const u32) };
            }
            let real_level = unsafe { &*real_branch_ptr }.current_funding_level();
            live_support::destroy_standalone_funding_branch(real_branch_ptr);

            let reimpl_branch_ptr = live_support::build_standalone_funding_branch(current_funding_level, &levels);
            let reimpl_branch = unsafe { &mut *reimpl_branch_ptr };
            if increase {
                reimpl_branch.increase_funding();
            } else {
                reimpl_branch.decrease_funding();
            }
            let reimpl_level = reimpl_branch.current_funding_level();
            live_support::destroy_standalone_funding_branch(reimpl_branch_ptr);

            prop_assert_eq!(
                real_level,
                reimpl_level,
                "current_funding_level mismatch for current_funding_level={}, level_count={}, increase={}",
                current_funding_level,
                level_count,
                increase
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETING_INCREASE_FUNDING: compares the real `ZTMarketing::increaseFunding` against the
    /// reimplemented `increase_funding`, on two independent standalone `ZTMarketing`s (not spliced into
    /// any `ZTMarketingMgr`, since `increaseFunding` only reads/writes `this`). `current_funding_level`
    /// spans `0..6` against funding tables of `0..5` entries, to cover in-range/top-of-range/
    /// one-past-the-end starting values. Compares both the resulting index and vanilla's masked
    /// low-byte return value.
    fn run_marketing_increase_funding_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETING_INCREASE_FUNDING";
        let mut fail_flag = false;

        match runner.run(&(0u32..6, 0usize..5), |(current_funding_level, level_count)| {
            let real_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
            let real_ret = unsafe { ztmarketing::INCREASE_FUNDING.original()(real_ptr as *const u32) };
            let real_index = unsafe { &*real_ptr }.current_funding_level();
            marketing_live_support::destroy_standalone_marketing(real_ptr);

            let reimpl_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
            let reimpl_marketing = unsafe { &mut *reimpl_ptr };
            let reimpl_ret = reimpl_marketing.increase_funding();
            let reimpl_index = reimpl_marketing.current_funding_level();
            marketing_live_support::destroy_standalone_marketing(reimpl_ptr);

            prop_assert_eq!(
                real_index,
                reimpl_index,
                "current_funding_level mismatch for start={}, level_count={}",
                current_funding_level,
                level_count
            );
            prop_assert_eq!(
                (real_ret & 0xff) != 0,
                reimpl_ret,
                "return-flag mismatch for start={}, level_count={}",
                current_funding_level,
                level_count
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETING_DECREASE_FUNDING: same shape as `run_marketing_increase_funding_test`, comparing
    /// `ZTMarketing::decreaseFunding` against `decrease_funding`.
    fn run_marketing_decrease_funding_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETING_DECREASE_FUNDING";
        let mut fail_flag = false;

        match runner.run(&(0u32..6, 0usize..5), |(current_funding_level, level_count)| {
            let real_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
            let real_ret = unsafe { ztmarketing::DECREASE_FUNDING.original()(real_ptr as *const u32) };
            let real_index = unsafe { &*real_ptr }.current_funding_level();
            marketing_live_support::destroy_standalone_marketing(real_ptr);

            let reimpl_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
            let reimpl_marketing = unsafe { &mut *reimpl_ptr };
            let reimpl_ret = reimpl_marketing.decrease_funding();
            let reimpl_index = reimpl_marketing.current_funding_level();
            marketing_live_support::destroy_standalone_marketing(reimpl_ptr);

            prop_assert_eq!(
                real_index,
                reimpl_index,
                "current_funding_level mismatch for start={}, level_count={}",
                current_funding_level,
                level_count
            );
            prop_assert_eq!(
                (real_ret & 0xff) != 0,
                reimpl_ret,
                "return-flag mismatch for start={}, level_count={}",
                current_funding_level,
                level_count
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETING_SET_FUNDING_LEVEL: compares the real `ZTMarketing::setFundingLevel` against the
    /// reimplemented `set_funding_level`. `level` spans `0..6` against funding tables of `0..5`
    /// entries, to cover `setFundingLevel`'s "reset to `0`" out-of-range behavior.
    fn run_marketing_set_funding_level_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETING_SET_FUNDING_LEVEL";
        let mut fail_flag = false;

        match runner.run(&(0u32..2, 0usize..5, 0u32..6), |(current_funding_level, level_count, level)| {
            let real_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
            unsafe { ztmarketing::SET_FUNDING_LEVEL.original()(real_ptr as *const u32, level) };
            let real_index = unsafe { &*real_ptr }.current_funding_level();
            marketing_live_support::destroy_standalone_marketing(real_ptr);

            let reimpl_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
            let reimpl_marketing = unsafe { &mut *reimpl_ptr };
            reimpl_marketing.set_funding_level(level);
            let reimpl_index = reimpl_marketing.current_funding_level();
            marketing_live_support::destroy_standalone_marketing(reimpl_ptr);

            prop_assert_eq!(
                real_index,
                reimpl_index,
                "current_funding_level mismatch for start={}, level_count={}, level={}",
                current_funding_level,
                level_count,
                level
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETINGMGR_UPDATE: compares the real `ZTMarketingMgr::update`'s effect on `tick_accumulator`
    /// against the reimplemented `update`, for a synthetic manager with no owned `ZTMarketing`
    /// (`marketing_ptr = null`) - so `ZTMarketing::update` (which needs a live `GLOBAL_ZTGameMgr`, see
    /// `run_marketing_update_test` below) never runs on either side.
    fn run_marketingmgr_update_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETINGMGR_UPDATE";
        let mut fail_flag = false;

        match runner.run(&(any::<u32>(), any::<u32>()), |(tick_accumulator_before, delta_ticks)| {
            let real_ptr = marketing_live_support::build_standalone_marketing_mgr(tick_accumulator_before, std::ptr::null_mut());
            unsafe { ZTMARKETINGMGR_UPDATE.original()((real_ptr as *mut ZTMarketingMgr) as *const u32, delta_ticks) };
            let real_tick_accumulator = unsafe { &*real_ptr }.tick_accumulator();
            marketing_live_support::destroy_standalone_marketing_mgr(real_ptr);

            let reimpl_ptr = marketing_live_support::build_standalone_marketing_mgr(tick_accumulator_before, std::ptr::null_mut());
            unsafe { &mut *reimpl_ptr }.update(delta_ticks);
            let reimpl_tick_accumulator = unsafe { &*reimpl_ptr }.tick_accumulator();
            marketing_live_support::destroy_standalone_marketing_mgr(reimpl_ptr);

            prop_assert_eq!(
                real_tick_accumulator,
                reimpl_tick_accumulator,
                "tick_accumulator mismatch for tick_accumulator_before={}, delta_ticks={}",
                tick_accumulator_before,
                delta_ticks
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETING_UPDATE: compares the real `ZTMarketingMgr::update`'s effect on a *wired-in*
    /// `ZTMarketing` against the reimplemented `update`, with `tick_accumulator` fixed so a threshold
    /// crossing always happens (`delta_ticks` generated `3000..10000`, always `> 359` days' worth per
    /// `predict_mgr_update`) - so `ZTMarketing::update` genuinely runs on both sides, not just the
    /// accumulator bookkeeping already covered by `ZTMARKETINGMGR_UPDATE` above. Run from
    /// `run_on_completion_reset_test_and_exit`'s `updateSim` injection point, since `GLOBAL_ZTGameMgr`
    /// isn't constructed yet at the earlier `LOAD_LANG_DLLS` battery.
    ///
    /// The funding table has `1..5` entries (`ZTMarketing::update`'s unchecked
    /// `funding_level(current_funding_level)` read needs a real, non-empty table to be safe), with
    /// `current_funding_level` spanning the whole table rather than fixed at index `0`.
    ///
    /// `available_cash` is generated as `cash_delta * cash_multiplier` for `cash_multiplier` in
    /// `0.0..2.0` (`cash_delta` computed the same way as `ZTMarketing::update`'s own
    /// `DAYS_TO_FUNDING_SCALE` formula below), so roughly half of generated cases land unaffordable and
    /// half affordable - taking the real affordable `<=` branch calls
    /// `ZooStatus::spendMarketing`/`ZTGameMgr::subtractCash` on the real `GLOBAL_ZTGameMgr` singleton on
    /// both sides. This used to be restricted to the *insufficient-cash* case only, because of a real
    /// bug: `openzt-detour/src/generated.rs`'s `SUBTRACT_CASH` `FunctionDef` declared one `f32` stack
    /// arg, but the real `ZTGameMgr::subtractCash` takes `(f32, bool)` per its `.asm`'s `RET 8` - a
    /// 4-byte stack imbalance on every `.original()` call. That's now fixed (see
    /// `ztmarketing-update-setmoneytext-crash-investigation.md`'s "Resolution" section), so the
    /// affordable branch is safe to exercise here too. The exact `available_cash == cash_delta`
    /// boundary is separately covered deterministically by `run_marketing_update_boundary_test` (real
    /// side only) and `run_marketing_update_reimpl_boundary_test` (reimplemented side only) below.
    fn run_marketing_update_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETING_UPDATE";
        let mut fail_flag = false;

        if marketing_live_support::ztgamemgr_ptr_is_null() {
            info!("Skipping {}: GLOBAL_ZTGameMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTGameMgr not initialized)", test_name));
            return fail_flag;
        }

        // Mirrors `ZTMarketing::update`'s own `DAYS_TO_FUNDING_SCALE` constant so the generated
        // `available_cash` can land just below the real `cash_delta` (`days * cost * scale`, where
        // `days` comes from `predict_mgr_update`'s tick-to-day conversion).
        const DAYS_TO_FUNDING_SCALE: f32 = 1.0 / 43200.0;

        match runner.run(
            &(3000u32..10000, prop::collection::vec(1f32..1000f32, 1..5), any::<usize>(), 0.0f32..2.0f32),
            |(delta_ticks, costs, raw_index, cash_multiplier)| {
                let current_funding_level = (raw_index % costs.len()) as u32;
                let levels: Vec<(i32, f32)> = costs.iter().map(|&cost| (0i32, cost)).collect();
                let selected_cost = costs[current_funding_level as usize];
                let (_, days) = predict_mgr_update(0, delta_ticks);
                let cash_delta = days as f32 * selected_cost * DAYS_TO_FUNDING_SCALE;
                let available_cash = (cash_delta * cash_multiplier).max(0.0);

                let real_marketing_ptr = marketing_live_support::build_standalone_marketing_with_levels(current_funding_level, &levels);
                let real_mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, real_marketing_ptr);
                marketing_live_support::with_ztgamemgr_cash(available_cash, || unsafe {
                    ZTMARKETINGMGR_UPDATE.original()((real_mgr_ptr as *mut ZTMarketingMgr) as *const u32, delta_ticks);
                });
                let real_tick_accumulator = unsafe { &*real_mgr_ptr }.tick_accumulator();
                let real_index = unsafe { &*real_marketing_ptr }.current_funding_level();
                marketing_live_support::destroy_standalone_marketing_mgr(real_mgr_ptr);
                marketing_live_support::destroy_standalone_marketing(real_marketing_ptr);

                let reimpl_marketing_ptr = marketing_live_support::build_standalone_marketing_with_levels(current_funding_level, &levels);
                let reimpl_mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, reimpl_marketing_ptr);
                marketing_live_support::with_ztgamemgr_cash(available_cash, || unsafe { &mut *reimpl_mgr_ptr }.update(delta_ticks));
                let reimpl_tick_accumulator = unsafe { &*reimpl_mgr_ptr }.tick_accumulator();
                let reimpl_index = unsafe { &*reimpl_marketing_ptr }.current_funding_level();
                marketing_live_support::destroy_standalone_marketing_mgr(reimpl_mgr_ptr);
                marketing_live_support::destroy_standalone_marketing(reimpl_marketing_ptr);

                prop_assert_eq!(
                    real_tick_accumulator,
                    reimpl_tick_accumulator,
                    "tick_accumulator mismatch for delta_ticks={}, current_funding_level={}, costs={:?}",
                    delta_ticks,
                    current_funding_level,
                    costs
                );
                prop_assert_eq!(
                    real_index,
                    reimpl_index,
                    "current_funding_level mismatch for delta_ticks={}, current_funding_level={}, costs={:?}",
                    delta_ticks,
                    current_funding_level,
                    costs
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
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// Deterministic single-case reproduction of the `ZTMARKETING_UPDATE` affordable-branch crash.
    /// Calls only the real `ZTMarketingMgr::update`, skipping the reimplemented side entirely, so a
    /// crash unambiguously means the real vanilla call path. `available_cash` is pinned to exactly
    /// `cash_delta` (the `<=` boundary itself, never exercised by `run_marketing_update_test` above),
    /// forcing the real side onto the affordable `spendMarketing`/`subtractCash` branch every time.
    ///
    /// Runs unconditionally as part of the normal battery. This whole module only compiles under the
    /// `reimplementation-tests` feature (never the shipped `openzt.dll`), so there's no production
    /// exposure to gate against. The crash this reproduces only occurs when `zoo.exe` is running under
    /// Windows' "Windows 7" compatibility-mode shim - with that shim off, this runs clean. If this test
    /// ever crashes the battery, check `zoo.exe`'s Compatibility tab before assuming a regression.
    ///
    /// This test's crash risk was always independent of the (now-fixed) `SUBTRACT_CASH` stack-imbalance
    /// bug documented on `run_marketing_update_test` above - it only ever calls genuine vanilla code via
    /// `.original()`, never routing through our own buggy `FunctionDef`. See
    /// `run_marketing_update_reimpl_boundary_test` below for the counterpart that exercises the
    /// reimplemented side, the path that *was* affected by that bug.
    fn run_marketing_update_boundary_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMARKETING_UPDATE_BOUNDARY_REPRO";

        if marketing_live_support::ztgamemgr_ptr_is_null() {
            info!("Skipping {}: GLOBAL_ZTGameMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTGameMgr not initialized)", test_name));
            return false;
        }

        const DAYS_TO_FUNDING_SCALE: f32 = 1.0 / 43200.0;
        let delta_ticks: u32 = 5000;
        let cost: f32 = 100.0;
        let levels: Vec<(i32, f32)> = vec![(0, cost)];
        let (_, days) = predict_mgr_update(0, delta_ticks);
        let cash_delta = days as f32 * cost * DAYS_TO_FUNDING_SCALE;

        let real_marketing_ptr = marketing_live_support::build_standalone_marketing_with_levels(0, &levels);
        let real_mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, real_marketing_ptr);

        info!(
            "{}: about to call real ZTMarketingMgr::update with available_cash == cash_delta ({}) - known to crash under Windows 7 compatibility mode, expected clean otherwise",
            test_name, cash_delta
        );
        marketing_live_support::with_ztgamemgr_cash(cash_delta, || unsafe {
            ZTMARKETINGMGR_UPDATE.original()((real_mgr_ptr as *mut ZTMarketingMgr) as *const u32, delta_ticks);
        });
        info!("{}: real call returned without crashing", test_name);

        marketing_live_support::destroy_standalone_marketing_mgr(real_mgr_ptr);
        marketing_live_support::destroy_standalone_marketing(real_marketing_ptr);

        write_success_line(failure_log, test_name);
        false
    }

    /// Deterministic single-case regression for the *reimplemented* side of `ZTMARKETING_UPDATE`'s affordable
    /// branch - the actual previously-buggy path (`ZTMarketing::update` -> `ZTGameMgr::spend_marketing`/
    /// `subtract_cash`, routed through our own `SPEND_MARKETING`/`SUBTRACT_CASH` `FunctionDef`s - unlike
    /// `run_marketing_update_boundary_test` above, which only ever calls genuine vanilla). See
    /// `ztmarketing-update-setmoneytext-crash-investigation.md`'s "Resolution" section for the fixed
    /// `SUBTRACT_CASH` signature bug, and its "Suggested next steps" item 7 for why this path specifically
    /// needed an independent check - it had never been exercised past the always-unaffordable branch before.
    ///
    /// Calls only the reimplemented `ZTMarketingMgr::update`, skipping the real vanilla side entirely.
    /// `available_cash` is pinned to exactly `cash_delta`, forcing the affordable branch every time. Runs
    /// unconditionally as part of the normal battery, same rationale as `run_marketing_update_boundary_test`.
    fn run_marketing_update_reimpl_boundary_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMARKETING_UPDATE_REIMPL_BOUNDARY_REPRO";

        if marketing_live_support::ztgamemgr_ptr_is_null() {
            info!("Skipping {}: GLOBAL_ZTGameMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTGameMgr not initialized)", test_name));
            return false;
        }

        const DAYS_TO_FUNDING_SCALE: f32 = 1.0 / 43200.0;
        let delta_ticks: u32 = 5000;
        let cost: f32 = 100.0;
        let levels: Vec<(i32, f32)> = vec![(0, cost)];
        let (_, days) = predict_mgr_update(0, delta_ticks);
        let cash_delta = days as f32 * cost * DAYS_TO_FUNDING_SCALE;

        let reimpl_marketing_ptr = marketing_live_support::build_standalone_marketing_with_levels(0, &levels);
        let reimpl_mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, reimpl_marketing_ptr);

        info!("{}: about to call reimplemented ZTMarketingMgr::update with available_cash == cash_delta ({})", test_name, cash_delta);
        marketing_live_support::with_ztgamemgr_cash(cash_delta, || unsafe { &mut *reimpl_mgr_ptr }.update(delta_ticks));
        info!("{}: reimplemented call returned without crashing", test_name);

        marketing_live_support::destroy_standalone_marketing_mgr(reimpl_mgr_ptr);
        marketing_live_support::destroy_standalone_marketing(reimpl_marketing_ptr);

        write_success_line(failure_log, test_name);
        false
    }

    /// ZTMARKETING_GET_FUNDING_TEXT: compares the real `ZTMarketing::getFundingText`'s output against
    /// the reimplemented `ZTMarketing::funding_text`, for a standalone marketing with a generated
    /// funding table and `current_funding_level` spanning negative/in-range/out-of-range relative to
    /// the table's length. Same shape as `run_funding_text_test` above, reusing its
    /// `funding_level_case_strategy` for the (name_id, cost) generation.
    fn run_marketing_funding_text_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETING_GET_FUNDING_TEXT";
        let mut fail_flag = false;

        match runner.run(&(-2i32..4i32, prop::collection::vec(funding_level_case_strategy(), 0..3)), |(current_funding_level, levels)| {
            let current_funding_level = current_funding_level as u32;
            let marketing_ptr = marketing_live_support::build_standalone_marketing_with_levels(current_funding_level, &levels);

            let mut buffer = [0u32; 3];
            unsafe {
                ztmarketing::GET_FUNDING_TEXT.original()(marketing_ptr as *const u32, buffer.as_mut_ptr() as *const u32);
            }
            let real_text = get_from_memory::<ZTBufferString>(buffer.as_ptr() as u32).copy_to_string();
            let reimpl_text = unsafe { &*marketing_ptr }.funding_text();

            marketing_live_support::destroy_standalone_marketing(marketing_ptr);

            prop_assert_eq!(
                real_text,
                reimpl_text,
                "funding_text mismatch for current_funding_level={}, levels={:?}",
                current_funding_level,
                levels
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETINGMGR_SAVE: compares the real `ZTMarketingMgr::save`'s captured output (via
    /// `io_redirect`) against the single little-endian `u32` funding-level index vanilla is expected to
    /// write - `0` when no `ZTMarketing` is owned.
    fn run_marketingmgr_save_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETINGMGR_SAVE";
        let mut fail_flag = false;

        match runner.run(&(any::<bool>(), 0u32..10), |(has_marketing, current_funding_level)| {
            let marketing_ptr = if has_marketing {
                marketing_live_support::build_standalone_marketing(current_funding_level, 0)
            } else {
                std::ptr::null_mut()
            };
            let mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, marketing_ptr);
            let mgr = unsafe { &mut *mgr_ptr };

            let dummy_file: u32 = 0;
            io_redirect::begin_capture();
            let _ = mgr.save(&dummy_file as *const u32);
            let captured_bytes = io_redirect::end_capture();

            marketing_live_support::destroy_standalone_marketing_mgr(mgr_ptr);
            marketing_live_support::destroy_standalone_marketing(marketing_ptr);

            let expected_index: u32 = if has_marketing { current_funding_level } else { 0 };
            let expected_bytes = expected_index.to_le_bytes().to_vec();

            prop_assert_eq!(
                captured_bytes,
                expected_bytes,
                "ZTMarketingMgr::save byte mismatch for has_marketing={}, current_funding_level={}",
                has_marketing,
                current_funding_level
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETINGMGR_LOAD: compares the real `ZTMarketingMgr::load`'s effect on the owned
    /// `ZTMarketing`'s funding-level index (and its own return value) against
    /// `marketing_save_reimplementation::predict_load`, for a generated funding-level table size,
    /// starting index, save-format version (spanning both sides of the `0x3a` threshold), and stream
    /// content - `bytes_present = false` supplies an empty replay buffer, exercising `load`'s
    /// read-failure abort path (`predict_load`'s `None` branch) when `version` is above the threshold.
    fn run_marketingmgr_load_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETINGMGR_LOAD";
        let mut fail_flag = false;

        match runner.run(
            &(0u32..0x50, any::<u32>(), 0usize..6, 0u32..8, any::<bool>()),
            |(version, saved_value, level_count, current_funding_level, bytes_present)| {
                let marketing_ptr = marketing_live_support::build_standalone_marketing(current_funding_level, level_count);
                let mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, marketing_ptr);
                let mgr = unsafe { &mut *mgr_ptr };

                let bytes = if bytes_present { saved_value.to_le_bytes().to_vec() } else { Vec::new() };
                let file_buffer = [0u32; 4];
                io_redirect::begin_replay(bytes);
                let load_ret = mgr.load(file_buffer.as_ptr(), version);
                io_redirect::end_replay();

                let real_index = unsafe { &*marketing_ptr }.current_funding_level();
                marketing_live_support::destroy_standalone_marketing_mgr(mgr_ptr);
                marketing_live_support::destroy_standalone_marketing(marketing_ptr);

                let read_value = bytes_present.then_some(saved_value);
                let (expected_ok, expected_index) = marketing_save_reimplementation::predict_load(version, read_value, level_count, current_funding_level);

                prop_assert_eq!(
                    load_ret,
                    expected_ok,
                    "return value mismatch for version={}, level_count={}, current_funding_level={}, bytes_present={}",
                    version,
                    level_count,
                    current_funding_level,
                    bytes_present
                );
                prop_assert_eq!(
                    real_index,
                    expected_index,
                    "current_funding_level mismatch for version={}, saved_value={}, level_count={}, current_funding_level={}, bytes_present={}",
                    version,
                    saved_value,
                    level_count,
                    current_funding_level,
                    bytes_present
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
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETINGMGR_CLEAR_CONFIGURATIONS: compares the real `ZTMarketingMgr::clearConfigurations`
    /// against the reimplemented `clear_configurations`, confirming both leave `tick_accumulator == 0`
    /// and `marketing_ptr` left dangling (non-null, pointing at now-freed memory) rather than nulled.
    /// The stale pointer is never dereferenced further here, only checked for non-nullness via
    /// `marketing_ptr_raw()`.
    ///
    /// Unlike every other real-vanilla call this file makes elsewhere (which only ever read memory),
    /// `clearConfigurations` actually **frees** the owned `ZTMarketing` - and freeing a Rust
    /// `Box`-allocated `ZTMarketing` through vanilla's own destructor/delete path is a cross-heap risk.
    /// So the "real" side's `ZTMarketing` is instead allocated via the native `standalone::OPERATOR_NEW`
    /// and initialized via `ztmarketing::CONSTRUCTOR`, keeping the real free heap-consistent with how
    /// the memory was allocated.
    fn run_marketingmgr_clear_configurations_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTMARKETINGMGR_CLEAR_CONFIGURATIONS";
        let mut fail_flag = false;

        match runner.run(&any::<u32>(), |tick_accumulator| {
            let real_raw = unsafe { standalone::OPERATOR_NEW.original()(size_of::<ZTMarketing>() as u32) };
            prop_assume!(!real_raw.is_null());
            let real_marketing_ptr = unsafe { ztmarketing::CONSTRUCTOR.original()(real_raw as *const u32) } as *mut ZTMarketing;

            let real_mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(tick_accumulator, real_marketing_ptr);
            unsafe { ZTMARKETINGMGR_CLEAR_CONFIGURATIONS.original()((real_mgr_ptr as *mut ZTMarketingMgr) as *const u32) };
            let real_mgr = unsafe { &*real_mgr_ptr };
            let real_tick_accumulator = real_mgr.tick_accumulator();
            let real_marketing_ptr_nonnull = real_mgr.marketing_ptr_raw() != 0;
            marketing_live_support::destroy_standalone_marketing_mgr(real_mgr_ptr);

            let reimpl_marketing_ptr = marketing_live_support::build_standalone_marketing(0, 0);
            let reimpl_mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(tick_accumulator, reimpl_marketing_ptr);
            unsafe { &mut *reimpl_mgr_ptr }.clear_configurations();
            let reimpl_mgr = unsafe { &*reimpl_mgr_ptr };
            let reimpl_tick_accumulator = reimpl_mgr.tick_accumulator();
            let reimpl_marketing_ptr_nonnull = reimpl_mgr.marketing_ptr_raw() != 0;
            marketing_live_support::destroy_standalone_marketing_mgr(reimpl_mgr_ptr);

            prop_assert_eq!(real_tick_accumulator, 0, "real tick_accumulator not reset for tick_accumulator={}", tick_accumulator);
            prop_assert_eq!(reimpl_tick_accumulator, 0, "reimplemented tick_accumulator not reset for tick_accumulator={}", tick_accumulator);
            prop_assert!(real_marketing_ptr_nonnull, "real marketing_ptr unexpectedly nulled for tick_accumulator={}", tick_accumulator);
            prop_assert!(reimpl_marketing_ptr_nonnull, "reimplemented marketing_ptr unexpectedly nulled for tick_accumulator={}", tick_accumulator);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTMARKETINGMGR_LOAD_CONFIGURATIONS: compares the real, live `globals().ztmarketingmgr()`'s
    /// funding table - populated by vanilla's own untouched boot-time `loadConfigurations` call, whose
    /// path was captured into `CAPTURED_MARKETING_PATH` - against this crate's own
    /// `ZTMarketingMgr::load_configurations` reimplementation, run directly on a standalone
    /// `ZTMarketingMgr` with the same captured path. Not a proptest - there's exactly one real path/one
    /// real answer to compare. Skipped (not failed) if the path was never captured, or if
    /// `GLOBAL_ZTMarketingMgr` itself isn't initialized yet.
    fn run_marketingmgr_load_configurations_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTMARKETINGMGR_LOAD_CONFIGURATIONS";

        let Some(path) = CAPTURED_MARKETING_PATH.get() else {
            info!("Skipping {}: no path captured from ZTMarketingMgr::loadConfigurations", test_name);
            write_success_line(failure_log, &format!("{} (skipped: no path captured)", test_name));
            return false;
        };

        if globals().ztmarketingmgr_ptr().is_null() {
            info!("Skipping {}: GLOBAL_ZTMarketingMgr not initialized at this injection point", test_name);
            write_success_line(failure_log, &format!("{} (skipped: ZTMarketingMgr not initialized)", test_name));
            return false;
        }

        let expected_marketing = globals().ztmarketingmgr().marketing();
        let expected_levels: Vec<(i32, i32, u32)> =
            expected_marketing.map(|m| m.funding_levels().iter().map(|l| (l.name_id(), l.benefit(), l.cost().to_bits())).collect()).unwrap_or_default();
        let expected_index = expected_marketing.map(|m| m.current_funding_level()).unwrap_or(0);

        let mgr_ptr = marketing_live_support::build_standalone_marketing_mgr(0, std::ptr::null_mut());
        let mgr = unsafe { &mut *mgr_ptr };
        mgr.load_configurations(path);

        let actual_marketing = mgr.marketing();
        let actual_levels: Vec<(i32, i32, u32)> =
            actual_marketing.map(|m| m.funding_levels().iter().map(|l| (l.name_id(), l.benefit(), l.cost().to_bits())).collect()).unwrap_or_default();
        let actual_index = actual_marketing.map(|m| m.current_funding_level()).unwrap_or(0);

        mgr.clear_configurations();
        marketing_live_support::destroy_standalone_marketing_mgr(mgr_ptr);

        if expected_levels == actual_levels && expected_index == actual_index {
            info!("{} passed for path '{}'", test_name, path);
            write_success_line(failure_log, test_name);
            false
        } else {
            error!(
                "{} failed for path '{}': expected_levels={:?}, actual_levels={:?}, expected_index={}, actual_index={}",
                test_name, path, expected_levels, actual_levels, expected_index, actual_index
            );
            if let Some(log_file) = failure_log {
                let _ = log_file.write_all(
                    format!(
                        "Test Failed {}: path '{}', expected_levels={:?}, actual_levels={:?}, expected_index={}, actual_index={}\n",
                        test_name, path, expected_levels, actual_levels, expected_index, actual_index
                    )
                    .as_bytes(),
                );
            }
            true
        }
    }

    /// Real vanilla's `daysRemainingOnProgram` runs its whole `FSUB`/`FMUL`/`FDIV` chain on the x87
    /// stack, which computes at 80-bit extended precision internally and only rounds down to a
    /// 32-bit `f32` once, at the very end (when the caller stores the `ST(0)` return value) - unlike
    /// `days_remaining_on_program`'s Rust arithmetic, which (per strict IEEE-754 `f32` semantics,
    /// with no `ST(0)`-equivalent extended-precision accumulator) rounds to `f32` after *each*
    /// intermediate operation. Confirmed live: for `target_cost=8832.339, current_progress=0.0,
    /// funding_rate=2.0866816`, real vanilla returns `126981.6` and this crate's arithmetic returns
    /// `126981.59` - a one-part-in-1.6e7 difference, consistent with a single-ULP x87-vs-strict-`f32`
    /// rounding difference, not a logic bug (a real formula/order-of-operations bug would produce a
    /// far larger, `target_cost`/`rate`-dependent difference, not a fixed few-ULP one). So this
    /// comparison uses a relative-tolerance check for `days`, not `prop_assert_eq!`'s exact equality.
    fn days_approximately_eq(real: Option<f32>, reimpl: Option<f32>) -> bool {
        match (real, reimpl) {
            (None, None) => true,
            (Some(real), Some(reimpl)) => {
                // 64 ULPs' worth of relative slack for a 3-operation `f32` chain - generous next to
                // the single-ULP-scale difference actually observed live, but still ~1500x tighter
                // than the smallest realistic logic-bug difference (e.g. a missing `* 30.0`, or
                // dividing by the wrong field, both of which change the result by orders of magnitude).
                let tolerance = 64.0 * f32::EPSILON * real.abs().max(reimpl.abs()).max(1.0);
                (real - reimpl).abs() <= tolerance
            }
            _ => false,
        }
    }

    /// ZTRESEARCHBRANCH_PCT_DAYS_REMAINING: compares the real `ZTResearchBranch::pctRemainingOnProgram`/
    /// `daysRemainingOnProgram` (`ztresearchbranch::PCT_REMAINING_ON_PROGRAM`/`DAYS_REMAINING_ON_PROGRAM`
    /// - a Ghidra regen has since fixed these `FunctionDef`s' auto-detected signatures, which were
    /// originally wrong: `-> i64` and no return type at all, respectively. See `pct_remaining_on_program`'s
    /// own doc comment in `ztresearch.rs` for the disassembly evidence that drove that fix) against the
    /// reimplemented `pct_remaining_on_program`/`days_remaining_on_program`, on a single branch built via
    /// `live_support::build_update_test_branch`. Both real and reimplemented sides read the exact same
    /// branch instance - these methods are `&self`-only with no side effects, so unlike the funding-level
    /// tests above there's no need to build two independent trees. `target_cost` includes an explicit
    /// `0.0` case alongside a general range: dividing by zero, `pct`'s only real edge case (`days`
    /// divides by `rate`, never `target_cost` - see its own doc comment in `ztresearch.rs`), produces a
    /// NaN/±Infinity that has to survive `pct`'s float-to-int conversion - this is exactly the case that
    /// originally caught the reimplementation's `f32 as i32` saturating cast disagreeing with vanilla's
    /// `FISTP`-based one (see `pct_remaining_on_program`'s own doc comment). Whether there's a real
    /// "None" for a given case is derived from `current_funding_rate() > 0.0` (the same guard both real
    /// and reimplemented code apply) rather than trusting `pct`'s raw `-1` return as a sentinel - `-1` is
    /// also a legitimate in-range percentage (e.g. progress just past target_cost), so it can't be told
    /// apart from the guard-failure sentinel by value alone.
    fn run_research_branch_pct_days_remaining_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTRESEARCHBRANCH_PCT_DAYS_REMAINING";
        let mut fail_flag = false;

        let target_cost_strategy = prop_oneof![Just(0.0f32), -10000.0f32..10000.0f32];

        match runner.run(&(target_cost_strategy, -10000.0f32..10000.0f32, -5.0f32..5.0f32), |(target_cost, current_progress, funding_rate)| {
            let branch_ptr = live_support::build_update_test_branch(target_cost, current_progress, funding_rate, 0.0);
            let branch_ref = unsafe { &*branch_ptr };

            let raw_real_pct = unsafe { ztresearchbranch::PCT_REMAINING_ON_PROGRAM.original()(branch_ptr as *const u32) };
            let raw_real_days = unsafe { ztresearchbranch::DAYS_REMAINING_ON_PROGRAM.original()(branch_ptr as *const u32) };
            let rate_contributing = branch_ref.current_funding_rate().is_some_and(|rate| rate > 0.0);
            let real_pct = rate_contributing.then_some(raw_real_pct);
            let real_days = rate_contributing.then_some(raw_real_days);

            let reimpl_pct = branch_ref.pct_remaining_on_program();
            let reimpl_days = branch_ref.days_remaining_on_program();

            live_support::destroy_update_test_branch(branch_ptr);

            prop_assert_eq!(
                real_pct,
                reimpl_pct,
                "pct_remaining_on_program mismatch for target_cost={}, current_progress={}, funding_rate={}",
                target_cost,
                current_progress,
                funding_rate
            );
            prop_assert!(
                days_approximately_eq(real_days, reimpl_days),
                "days_remaining_on_program mismatch for target_cost={}, current_progress={}, funding_rate={}: real={:?}, reimpl={:?}",
                target_cost,
                current_progress,
                funding_rate,
                real_days,
                reimpl_days
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    // ============================================================================================
    // Live comparison battery for ZTThoughtMgr/ZTThought. `ZTTHOUGHTMGR_ADD_THOUGHT`/
    // `_REMOVE_THOUGHTS_BY_*`/`_GET_THOUGHTS_BY_*`/`_SAVE`/`_LOAD` are self-contained (no
    // `GLOBAL_ZTWorldMgr`/string-table dependency) and run from the early battery above;
    // `_POPULATE_THOUGHTS` and `ZTTHOUGHT_GET_STRING` need `GLOBAL_ZTWorldMgr`/language DLLs
    // respectively, so they run from `run_on_completion_reset_test_and_exit`'s later chain instead.
    // ============================================================================================

    /// Field tuple used to compare two `ZTThought`s structurally via their existing public getters -
    /// `ZTThought` derives neither `PartialEq` nor a public constructor, so this is simpler than adding
    /// either just for these tests.
    fn thought_fields(t: &ZTThought) -> (u32, u32, u32, i32, i32, u32, u32, u32) {
        (t.string_id(), t.thinker_id(), t.object_id(), t.tile_x(), t.tile_y(), t.thinker_ptr(), t.object_ptr(), t.habitat_ptr())
    }

    /// ZTTHOUGHTMGR_ADD_THOUGHT: compares the real `ZTThoughtMgr::addThought`'s effect on list
    /// order/length against the reimplemented `add_thought`, across a generated sequence of calls and a
    /// small `max_thoughts` cap (so cap-trimming is actually exercised). Restricted to
    /// `thinker_ptr = object_ptr = habitat_ptr = 0` for every call - `ZTThought::new` dereferences all
    /// three when non-null, and there's no live entity/habitat this standalone test could safely point
    /// them at. This still fully exercises `addThought`'s own cap-trim/insertion-order logic, the part
    /// this test actually targets.
    fn run_thoughtmgr_add_thought_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_ADD_THOUGHT";
        let mut fail_flag = false;

        match runner.run(&(1u32..5, prop::collection::vec(any::<u32>(), 0..8)), |(max_thoughts, string_ids)| {
            let real_ptr = thought_live_support::build_standalone_mgr(max_thoughts);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(max_thoughts);

            for &string_id in &string_ids {
                unsafe {
                    gen_ztthoughtmgr::ADD_THOUGHT.original()(real_ptr as *const u32, string_id, std::ptr::null(), std::ptr::null(), std::ptr::null());
                }
                unsafe { &mut *reimpl_ptr }.add_thought(string_id, 0, 0, 0);
            }

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();
            let real_len = unsafe { &*real_ptr }.len();
            let reimpl_len = unsafe { &*reimpl_ptr }.len();

            thought_live_support::destroy_standalone_mgr_leaking_nodes(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_len, reimpl_len, "length mismatch for max_thoughts={}, string_ids={:?}", max_thoughts, string_ids);
            prop_assert_eq!(real_fields, reimpl_fields, "content mismatch for max_thoughts={}, string_ids={:?}", max_thoughts, string_ids);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// Seeds `mgr` with one `ZTThought` per `specs` entry (`(string_id, thinker_ptr, object_ptr,
    /// habitat_ptr)`), front-to-back, via `insert_front` - builds identical starting state for both
    /// sides of a removal/lookup comparison. `thinker_id`/`object_id`/`tile_x`/`tile_y` are left at
    /// ctor defaults since no consumer of this helper reads them.
    fn seed_thoughts(mgr: &mut ZTThoughtMgr, specs: &[(u32, u32, u32, u32)]) {
        for &(string_id, thinker_ptr, object_ptr, habitat_ptr) in specs {
            mgr.insert_front(thought_live_support::new_thought(string_id, 0, 0, -1, -1, thinker_ptr, object_ptr, habitat_ptr));
        }
    }

    fn thought_spec_strategy() -> impl Strategy<Value = (u32, u32, u32, u32)> {
        (any::<u32>(), 0u32..5, 0u32..5, 0u32..5)
    }

    /// ZTTHOUGHTMGR_REMOVE_THOUGHTS_BY_THINKER: compares the real `ZTThoughtMgr::removeThoughtsByThinker`
    /// against the reimplemented `remove_thoughts_by_thinker`, on two identically-seeded standalone
    /// managers. `thinker_ptr`/`object_ptr`/`habitat_ptr` are generated over a small `0..5` range so
    /// `target` collides with a seeded value often enough to exercise real removals, not just the no-op
    /// case.
    fn run_thoughtmgr_remove_thoughts_by_thinker_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_REMOVE_THOUGHTS_BY_THINKER";
        let mut fail_flag = false;

        match runner.run(&(prop::collection::vec(thought_spec_strategy(), 0..8), 0u32..5), |(specs, target)| {
            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            seed_thoughts(unsafe { &mut *real_ptr }, &specs);
            seed_thoughts(unsafe { &mut *reimpl_ptr }, &specs);

            unsafe {
                gen_ztthoughtmgr::REMOVE_THOUGHTS_BY_THINKER.original()(real_ptr as *const u32, target as *const u32);
            }
            unsafe { &mut *reimpl_ptr }.remove_thoughts_by_thinker(target);

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for specs={:?}, target={}", specs, target);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_REMOVE_THOUGHTS_BY_OBJECT: same shape as
    /// `run_thoughtmgr_remove_thoughts_by_thinker_test`, comparing `removeThoughtsByObject`/
    /// `remove_thoughts_by_object` instead.
    fn run_thoughtmgr_remove_thoughts_by_object_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_REMOVE_THOUGHTS_BY_OBJECT";
        let mut fail_flag = false;

        match runner.run(&(prop::collection::vec(thought_spec_strategy(), 0..8), 0u32..5), |(specs, target)| {
            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            seed_thoughts(unsafe { &mut *real_ptr }, &specs);
            seed_thoughts(unsafe { &mut *reimpl_ptr }, &specs);

            unsafe {
                gen_ztthoughtmgr::REMOVE_THOUGHTS_BY_OBJECT.original()(real_ptr as *const u32, target as *const u32);
            }
            unsafe { &mut *reimpl_ptr }.remove_thoughts_by_object(target);

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for specs={:?}, target={}", specs, target);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_REMOVE_THOUGHTS_BY_HABITAT: same shape again, additionally generating `force` -
    /// `removeThoughtsByHabitat` has a third outcome `removeThoughtsBy{Thinker,Object}` don't: a
    /// matching thought with a live `object_ptr` survives with its `habitat_ptr` link cleared instead of
    /// being removed outright, unless `force` is set.
    fn run_thoughtmgr_remove_thoughts_by_habitat_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_REMOVE_THOUGHTS_BY_HABITAT";
        let mut fail_flag = false;

        match runner.run(&(prop::collection::vec(thought_spec_strategy(), 0..8), 0u32..5, any::<bool>()), |(specs, target, force)| {
            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            seed_thoughts(unsafe { &mut *real_ptr }, &specs);
            seed_thoughts(unsafe { &mut *reimpl_ptr }, &specs);

            unsafe {
                gen_ztthoughtmgr::REMOVE_THOUGHTS_BY_HABITAT.original()(real_ptr as *const u32, target as *const i32, force as i8);
            }
            unsafe { &mut *reimpl_ptr }.remove_thoughts_by_habitat(target, force);

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for specs={:?}, target={}, force={}", specs, target, force);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_GET_THOUGHTS_BY_THINKER: compares the real, undetoured `getThoughtsByThinker`'s
    /// output - a real vanilla temporary list, walked read-only via
    /// `thought_live_support::read_only_wrap_vanilla_list` - against the reimplemented
    /// `get_thoughts_by_thinker`, on a single seeded standalone manager (both calls only read the
    /// manager's own list, so there's no need for two independent instances).
    fn run_thoughtmgr_get_thoughts_by_thinker_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_GET_THOUGHTS_BY_THINKER";
        let mut fail_flag = false;

        match runner.run(&(prop::collection::vec(thought_spec_strategy(), 0..8), 0u32..5, 1i32..5), |(specs, target, max_count)| {
            let mgr_ptr = thought_live_support::build_standalone_mgr(1000);
            seed_thoughts(unsafe { &mut *mgr_ptr }, &specs);
            let mgr = unsafe { &*mgr_ptr };

            let mut real_sentinel: u32 = 0;
            unsafe {
                gen_ztthoughtmgr::GET_THOUGHTS_BY_THINKER.original()(
                    mgr_ptr as *const u32,
                    &raw mut real_sentinel as *const i32,
                    target as *const i32,
                    max_count,
                );
            }
            let real_wrapper = thought_live_support::read_only_wrap_vanilla_list(real_sentinel);
            let real_fields: Vec<_> = real_wrapper.iter().map(thought_fields).collect();

            let reimpl_fields: Vec<_> = mgr.get_thoughts_by_thinker(target, max_count as usize).into_iter().map(|t| thought_fields(t)).collect();

            thought_live_support::destroy_standalone_mgr(mgr_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for specs={:?}, target={}, max_count={}", specs, target, max_count);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_GET_THOUGHTS_BY_OBJECT: same shape as
    /// `run_thoughtmgr_get_thoughts_by_thinker_test`, for `getThoughtsByObject`/`get_thoughts_by_object`.
    /// `max_count` is passed as `max_count as *const i32` - `GET_THOUGHTS_BY_OBJECT`'s `*const i32`
    /// signature is a Ghidra type-inference artifact; the real calling convention passes the count by
    /// value for all three `getThoughtsBy*` functions.
    fn run_thoughtmgr_get_thoughts_by_object_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_GET_THOUGHTS_BY_OBJECT";
        let mut fail_flag = false;

        match runner.run(&(prop::collection::vec(thought_spec_strategy(), 0..8), 0u32..5, 1i32..5), |(specs, target, max_count)| {
            let mgr_ptr = thought_live_support::build_standalone_mgr(1000);
            seed_thoughts(unsafe { &mut *mgr_ptr }, &specs);
            let mgr = unsafe { &*mgr_ptr };

            let mut real_sentinel: u32 = 0;
            unsafe {
                gen_ztthoughtmgr::GET_THOUGHTS_BY_OBJECT.original()(
                    mgr_ptr as *const u32,
                    &raw mut real_sentinel as *const i32,
                    target as *const i32,
                    max_count as *const i32,
                );
            }
            let real_wrapper = thought_live_support::read_only_wrap_vanilla_list(real_sentinel);
            let real_fields: Vec<_> = real_wrapper.iter().map(thought_fields).collect();

            let reimpl_fields: Vec<_> = mgr.get_thoughts_by_object(target, max_count as usize).into_iter().map(|t| thought_fields(t)).collect();

            thought_live_support::destroy_standalone_mgr(mgr_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for specs={:?}, target={}, max_count={}", specs, target, max_count);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_GET_THOUGHTS_BY_HABITAT: same shape as
    /// `run_thoughtmgr_get_thoughts_by_object_test`, including `max_count`'s own by-value passing - for
    /// `getThoughtsByHabitat`/`get_thoughts_by_habitat`.
    fn run_thoughtmgr_get_thoughts_by_habitat_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_GET_THOUGHTS_BY_HABITAT";
        let mut fail_flag = false;

        match runner.run(&(prop::collection::vec(thought_spec_strategy(), 0..8), 0u32..5, 1i32..20), |(specs, target, max_count)| {
            let mgr_ptr = thought_live_support::build_standalone_mgr(1000);
            seed_thoughts(unsafe { &mut *mgr_ptr }, &specs);
            let mgr = unsafe { &*mgr_ptr };

            let mut real_sentinel: u32 = 0;
            unsafe {
                gen_ztthoughtmgr::GET_THOUGHTS_BY_HABITAT.original()(
                    mgr_ptr as *const u32,
                    &raw mut real_sentinel as *const i32,
                    target as *const i32,
                    max_count as *const i32,
                );
            }
            let real_wrapper = thought_live_support::read_only_wrap_vanilla_list(real_sentinel);
            let real_fields: Vec<_> = real_wrapper.iter().map(thought_fields).collect();

            let reimpl_fields: Vec<_> = mgr.get_thoughts_by_habitat(target, max_count as usize).into_iter().map(|t| thought_fields(t)).collect();

            thought_live_support::destroy_standalone_mgr(mgr_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for specs={:?}, target={}, max_count={}", specs, target, max_count);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_SAVE: compares the real `ZTThoughtMgr::save`'s captured output (via `io_redirect`)
    /// against the reimplemented `save`, on two identically-seeded standalone managers.
    fn run_thoughtmgr_save_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_SAVE";
        let mut fail_flag = false;

        let record_strategy = (any::<u32>(), any::<u32>(), any::<u32>(), -2i32..4, -2i32..4);
        match runner.run(&prop::collection::vec(record_strategy, 0..6), |records| {
            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            for &(string_id, thinker_id, object_id, tile_x, tile_y) in &records {
                unsafe { &mut *real_ptr }.insert_front(thought_live_support::new_thought(string_id, thinker_id, object_id, tile_x, tile_y, 0, 0, 0));
                unsafe { &mut *reimpl_ptr }.insert_front(thought_live_support::new_thought(string_id, thinker_id, object_id, tile_x, tile_y, 0, 0, 0));
            }

            let dummy_file: u32 = 0;
            io_redirect::begin_capture();
            unsafe { gen_ztthoughtmgr::SAVE.original()(real_ptr as *const u32, &dummy_file as *const u32) };
            let real_bytes = io_redirect::end_capture();

            io_redirect::begin_capture();
            let _ = unsafe { &*reimpl_ptr }.save(&dummy_file as *const u32);
            let reimpl_bytes = io_redirect::end_capture();

            thought_live_support::destroy_standalone_mgr(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_bytes, reimpl_bytes, "save byte mismatch for records={:?}", records);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_LOAD: compares the real `ZTThoughtMgr::load`'s effect on list content/order (and its
    /// own return value) against the reimplemented `load`, for a generated stream of legacy-format
    /// `(string_id, object_id, thinker_id)` records and `version < 0x1e` - the pre-`0x1e` legacy branch.
    /// Restricted to this range: `version >= 0x1e` triggers `ZTThought::load`'s own
    /// `thinker_id`/`object_id` -> pointer resolution via `ZTWorldMgr::resolve_entity_by_id`, which
    /// needs `GLOBAL_ZTWorldMgr` initialized - not true yet at this early injection point.
    /// `object_id`/`thinker_id` are generated over a small `0..3` range to land on both sides of
    /// `ZTThoughtMgr::load`'s survival gate (a legacy-format record only splices into the list if both
    /// ids end up `0`), not just the trivially-true `id == 0` case `any::<u32>()` would mostly hit.
    fn run_thoughtmgr_load_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_LOAD";
        let mut fail_flag = false;

        let record_strategy = (any::<u32>(), 0u32..3, 0u32..3);
        match runner.run(&(prop::collection::vec(record_strategy, 0..6), 0u32..0x1e), |(records, version)| {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
            for &(string_id, object_id, thinker_id) in &records {
                bytes.extend_from_slice(&string_id.to_le_bytes());
                bytes.extend_from_slice(&object_id.to_le_bytes());
                bytes.extend_from_slice(&thinker_id.to_le_bytes());
            }

            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            let file_buffer = [0u32; 4];

            io_redirect::begin_replay(bytes.clone());
            let real_ret = unsafe { gen_ztthoughtmgr::LOAD.original()(real_ptr as *const u32, file_buffer.as_ptr(), version) };
            io_redirect::end_replay();

            io_redirect::begin_replay(bytes);
            let reimpl_ret = unsafe { &mut *reimpl_ptr }.load(file_buffer.as_ptr(), version);
            io_redirect::end_replay();

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr_leaking_nodes(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_ret, reimpl_ret, "load() return mismatch for records={:?}, version={}", records, version);
            prop_assert_eq!(real_fields, reimpl_fields, "loaded content mismatch for records={:?}, version={}", records, version);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_LOAD_MODERN: compares the real `ZTThoughtMgr::load`'s effect against the
    /// reimplemented `load` for the `version >= 0x1e` branch - the branch `ZTTHOUGHTMGR_LOAD` above
    /// never exercises, since it drives inline `ZTWorldMgr::resolve_entity_by_id`/
    /// `ZTHabitatMgr::get_habitat_ptr` resolution that needs both globals initialized. Runs from
    /// `run_on_completion_reset_test_and_exit`'s later chain, like
    /// `ZTTHOUGHTMGR_POPULATE_THOUGHTS`/`ZTTHOUGHT_GET_STRING` below.
    ///
    /// `thinker_id`/`object_id` stay in the same small `0..5` range `ZTTHOUGHTMGR_POPULATE_THOUGHTS`
    /// already uses (real entity ids are never this low in a fresh test process, so
    /// `resolve_entity_by_id` deterministically returns null on both sides). Every record's tile is
    /// fixed at the `(-1, -1)` "no tile" sentinel, not generated: `ZTHabitatMgr::get_habitat_ptr`
    /// performs no bounds-checking against its own `other_array_start`/`other_array_end` fields, and at
    /// this early injection point (before any zoo is loaded) that array is too small/empty for even
    /// single-digit tile coordinates to stay in-bounds, so tile-based exercise of that path is deferred
    /// rather than attempted here.
    ///
    /// `truncate_at`, when `Some`, cuts the serialized byte stream short - folds in short-read coverage
    /// `ZTTHOUGHTMGR_LOAD` doesn't have. `io_redirect::deallocate` just returns failure once the replay
    /// buffer runs out, never touching out-of-bounds memory.
    fn run_thoughtmgr_load_modern_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let test_name = "ZTTHOUGHTMGR_LOAD_MODERN";

        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let mut fail_flag = false;

        let record_strategy = (any::<u32>(), 0u32..5, 0u32..5);
        let strategy = prop::collection::vec(record_strategy, 0..6).prop_flat_map(|records| {
            let total_len = 4 + records.len() * 20;
            (Just(records), 0x1eu32..0x40, prop::option::of(0usize..total_len))
        });

        match runner.run(&strategy, |(records, version, truncate_at)| {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&(records.len() as u32).to_le_bytes());
            for &(string_id, thinker_id, object_id) in &records {
                bytes.extend_from_slice(&string_id.to_le_bytes());
                bytes.extend_from_slice(&thinker_id.to_le_bytes());
                bytes.extend_from_slice(&object_id.to_le_bytes());
                bytes.extend_from_slice(&(-1i32 as u32).to_le_bytes());
                bytes.extend_from_slice(&(-1i32 as u32).to_le_bytes());
            }
            if let Some(cut) = truncate_at {
                bytes.truncate(cut);
            }

            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            let file_buffer = [0u32; 4];

            io_redirect::begin_replay(bytes.clone());
            let real_ret = unsafe { gen_ztthoughtmgr::LOAD.original()(real_ptr as *const u32, file_buffer.as_ptr(), version) };
            io_redirect::end_replay();

            io_redirect::begin_replay(bytes);
            let reimpl_ret = unsafe { &mut *reimpl_ptr }.load(file_buffer.as_ptr(), version);
            io_redirect::end_replay();

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr_leaking_nodes(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(
                real_ret,
                reimpl_ret,
                "load() return mismatch for records={:?}, version={}, truncate_at={:?}",
                records,
                version,
                truncate_at
            );
            prop_assert_eq!(
                real_fields,
                reimpl_fields,
                "loaded content mismatch for records={:?}, version={}, truncate_at={:?}",
                records,
                version,
                truncate_at
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_ADD_THOUGHT_ANIMAL_OVERRIDE: exercises the animal-subtype override branch inside
    /// `add_thought` (`resolve_object_own_habitat_ptr`'s two vtable calls) - `ZTAnimalType::isCastClass`
    /// at `0x004020cd` and `ZTAnimal::getHabitat` at `0x00410685` (via `calcHabitat` ->
    /// `ZTUnit::getHabitat`).
    ///
    /// Both the "real" (`ADD_THOUGHT.original()`) and "reimplemented" (`add_thought`) sides dispatch
    /// through the exact same real vanilla function pointers here - `resolve_object_own_habitat_ptr` is a
    /// call-through wrapper around vanilla's own vtable slots, not reimplemented logic. So this test
    /// isn't validating a separate vanilla habitat-resolution algorithm; it's validating that our
    /// sequencing (vtable offsets, `this`/argument marshalling, override-vs-fallback logic) matches
    /// vanilla's own `addThought.asm` exactly.
    ///
    /// The fixture: a `ZTAnimal` (zeroed via `ZTAnimal::new_for_test`; entity type built via
    /// `ZTAnimalType::new_for_test` so its vtable is the real `0x00630268` - slot `0x1c` resolves to
    /// `isCastClass`) with its own vtable field overwritten to the real `ZTAnimal` vtable `0x0062ff54`
    /// via a raw memory write, so slot `0x24c` resolves to `getHabitat`. `isCastClass` always returns
    /// true when called on a genuine `ZTAnimalType`-vtabled object, so this fixture reliably exercises
    /// the override-taken path, not the fallback.
    ///
    /// `getHabitat`'s own chain reads `BFEntity::getTile()` (a real vanilla, non-virtual call) and a
    /// show-info flag at a `BFUnit` offset; the fixture's zeroed base leaves that flag `0`, skipping the
    /// `ZTShowMgr` branch, and `pos = (0, 0, 0)` is expected to miss real vanilla's own tile lookup at
    /// this injection point (no zoo loaded yet).
    fn run_thoughtmgr_add_thought_animal_override_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_ADD_THOUGHT_ANIMAL_OVERRIDE";
        let mut fail_flag = false;

        match runner.run(&(any::<u32>(), 0u32..5), |(string_id, fallback_habitat_ptr)| {
            let entity_type = ZTAnimalType::new_for_test(IVec3::default(), 0, IVec3::default(), IVec3::default());
            let animal = ZTAnimal::new_for_test(&entity_type as *const ZTAnimalType as u32, 0, 0, false, false);
            let animal_addr = &animal as *const ZTAnimal as u32;
            save_to_memory::<u32>(animal_addr, 0x0062_ff54u32);

            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);

            unsafe {
                gen_ztthoughtmgr::ADD_THOUGHT.original()(
                    real_ptr as *const u32,
                    string_id,
                    std::ptr::null(),
                    animal_addr as *const u32,
                    fallback_habitat_ptr as *const u32,
                );
            }
            unsafe { &mut *reimpl_ptr }.add_thought(string_id, 0, animal_addr, fallback_habitat_ptr);

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr_leaking_nodes(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(
                real_fields,
                reimpl_fields,
                "content mismatch for string_id={}, fallback_habitat_ptr={:#x}",
                string_id,
                fallback_habitat_ptr
            );
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// ZTTHOUGHTMGR_POPULATE_THOUGHTS: compares the real `ZTThoughtMgr::populateThoughts`'s effect on
    /// every thought's resolved `thinker_ptr`/`object_ptr`/`habitat_ptr`/`tile_x`/`tile_y` against the
    /// reimplemented `populate_thoughts`, on two identically-seeded standalone managers. Needs
    /// `GLOBAL_ZTWorldMgr` initialized (`ZTThought::populate` calls `ZTWorldMgr::resolve_entity_by_id`
    /// unconditionally), so this runs from `run_on_completion_reset_test_and_exit`'s later chain, not
    /// the early battery. `thinker_id`/`object_id` are generated over a small `0..5` range: real
    /// entities essentially never have ids this low, so `resolve_entity_by_id` returns null on both
    /// sides for the overwhelming majority of cases - a safe, deterministic "no match" - while still
    /// leaving room for a genuine match.
    fn run_thoughtmgr_populate_thoughts_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHTMGR_POPULATE_THOUGHTS";
        let mut fail_flag = false;

        let record_strategy = (any::<u32>(), 0u32..5, 0u32..5);
        match runner.run(&prop::collection::vec(record_strategy, 0..6), |records| {
            let real_ptr = thought_live_support::build_standalone_mgr(1000);
            let reimpl_ptr = thought_live_support::build_standalone_mgr(1000);
            for &(string_id, thinker_id, object_id) in &records {
                unsafe { &mut *real_ptr }.insert_front(thought_live_support::new_thought(string_id, thinker_id, object_id, -1, -1, 0, 0, 0));
                unsafe { &mut *reimpl_ptr }.insert_front(thought_live_support::new_thought(string_id, thinker_id, object_id, -1, -1, 0, 0, 0));
            }

            unsafe { gen_ztthoughtmgr::POPULATE_THOUGHTS.original()(real_ptr as *const u32) };
            unsafe { &mut *reimpl_ptr }.populate_thoughts();

            let real_fields: Vec<_> = unsafe { &*real_ptr }.iter().map(thought_fields).collect();
            let reimpl_fields: Vec<_> = unsafe { &*reimpl_ptr }.iter().map(thought_fields).collect();

            thought_live_support::destroy_standalone_mgr(real_ptr);
            thought_live_support::destroy_standalone_mgr(reimpl_ptr);

            prop_assert_eq!(real_fields, reimpl_fields, "mismatch for records={:?}", records);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }

    /// Writes `name` into `entity`'s `name` field (`+0x108`, a `ZTBufferString`) by raw offset write.
    /// `entity` must not move after this call - its address is baked into the write. Returns the
    /// backing byte buffer, which the caller must keep alive at least as long as `entity` is read
    /// through.
    fn set_bfentity_name(entity: &BFEntity, name: &str) -> Vec<u8> {
        let mut encoded = name.as_bytes().to_vec();
        let len = encoded.len() as u32;
        encoded.push(0);
        let start = encoded.as_ptr() as u32;
        let entity_addr = entity as *const BFEntity as u32;
        save_to_memory::<u32>(entity_addr + 0x108, start);
        save_to_memory::<u32>(entity_addr + 0x10c, start + len);
        save_to_memory::<u32>(entity_addr + 0x110, start + encoded.len() as u32);
        encoded
    }

    /// Writes `name` into `habitat`'s `exhibit_name` field (`+0x154`, a `ZTBoundedString`) by raw offset
    /// write - same technique as `set_bfentity_name`, but `ZTBoundedString` is a 2-pointer
    /// `start_ptr`/`end_ptr` pair with no separate `buffer_end_ptr`.
    fn set_habitat_exhibit_name(habitat: &ZTHabitat, name: &str) -> Vec<u8> {
        let mut encoded = name.as_bytes().to_vec();
        let len = encoded.len() as u32;
        encoded.push(0);
        let start = encoded.as_ptr() as u32;
        let habitat_addr = habitat as *const ZTHabitat as u32;
        save_to_memory::<u32>(habitat_addr + 0x154, start);
        save_to_memory::<u32>(habitat_addr + 0x158, start + len);
        encoded
    }

    /// Which of `get_string`'s three substitution branches a `ZTTHOUGHT_GET_STRING` case exercises
    /// (priority: object, then habitat, then no substitution).
    #[derive(Debug, Clone)]
    enum GetStringSubstitution {
        None,
        Object(String),
        Habitat(String),
    }

    fn get_string_substitution_strategy() -> impl Strategy<Value = GetStringSubstitution> {
        prop_oneof![
            Just(GetStringSubstitution::None),
            "[a-zA-Z0-9 ]{0,16}".prop_map(GetStringSubstitution::Object),
            "[a-zA-Z0-9 ]{0,16}".prop_map(GetStringSubstitution::Habitat),
        ]
    }

    /// ZTTHOUGHT_GET_STRING: compares the real `ZTThought::getString`'s output against the reimplemented
    /// `get_string`, across all three substitution branches: no substitution (`object_ptr = habitat_ptr
    /// = 0`), object-name substitution (a fixture `BFEntity` with its `name` field set via
    /// `set_bfentity_name`), and habitat exhibit-name substitution (a fixture `ZTHabitat` with
    /// `exhibit_name` set via `set_habitat_exhibit_name`). `get_string` only ever reads these two fields
    /// directly (no vtable dispatch), so a zeroed fixture with just the name field populated is a safe,
    /// complete stand-in for a real live `BFEntity`/`ZTHabitat`. Runs from
    /// `run_on_completion_reset_test_and_exit`'s later chain: language DLLs, which
    /// `load_string_by_id`/`BFApp::loadString` both depend on, aren't loaded yet at the early injection
    /// point.
    fn run_thought_get_string_test(failure_log: &mut Option<std::fs::File>) -> bool {
        let runner_config = ProptestConfig {
            failure_persistence: Some(Box::new(super::NoopFailurePersistence)),
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(runner_config);
        let test_name = "ZTTHOUGHT_GET_STRING";
        let mut fail_flag = false;

        match runner.run(&(any::<u32>(), get_string_substitution_strategy()), |(string_id, case)| {
            let entity_storage = BFEntity::new_for_test(0, 0, 0);
            let habitat_storage: ZTHabitat = unsafe { std::mem::zeroed() };
            let _name_buf: Option<Vec<u8>>;

            let (object_ptr, habitat_ptr) = match &case {
                GetStringSubstitution::None => {
                    _name_buf = None;
                    (0u32, 0u32)
                }
                GetStringSubstitution::Object(name) => {
                    _name_buf = Some(set_bfentity_name(&entity_storage, name));
                    (&entity_storage as *const BFEntity as u32, 0u32)
                }
                GetStringSubstitution::Habitat(name) => {
                    _name_buf = Some(set_habitat_exhibit_name(&habitat_storage, name));
                    (0u32, &habitat_storage as *const ZTHabitat as u32)
                }
            };

            let thought = thought_live_support::new_thought(string_id, 0, 0, -1, -1, 0, object_ptr, habitat_ptr);

            let mut buffer = [0u32; 3];
            unsafe {
                gen_ztthought::GET_STRING.original()(&thought as *const ZTThought as *const u32, buffer.as_mut_ptr() as *const u32);
            }
            let real_text = get_from_memory::<ZTBufferString>(buffer.as_ptr() as u32).copy_to_string();
            let reimpl_text = thought.get_string();

            prop_assert_eq!(real_text, reimpl_text, "get_string mismatch for string_id={}, case={:?}", string_id, case);
            Ok(())
        }) {
            Ok(_) => {
                info!("Proptest passed for {}", test_name);
                write_success_line(failure_log, test_name);
            }
            Err(e) => {
                error!("Proptest failed: {:?}", e);
                if let Some(log_file) = failure_log {
                    let _ = log_file.write_all(format!("Test Failed {}: {:?}\n", test_name, e).as_bytes());
                }
                fail_flag = true;
            }
        }

        fail_flag
    }
}
