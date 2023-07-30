

#[repr(C)]
struct zt_game_mgr {
    zoo_budget: f32,
    num_animals: i32,
    num_exhibits: i32,
    num_guests: i32,
    admissions_income_by_month: [f32; 12],
    concessions_benefit_by_month: [f32; 12],
    recycling_benefit_by_month: [f32; 12],
    // net_income: f32,
    income_by_month: [f32; 12],
    income_expense_totals_by_month: [f32; 12],
    zoo_rating_by_month: [f32; 12],
    zoo_rating_average_by_month: [f32; 12],
    construction_cost_by_month: [f32; 12],
}

fn read_zt_game_mgr(zt_game_mgr_prt: u32) -> zt_game_mgr {
    zt_game_mgr{
        zoo_budget: get_from_memory::<i32>(zt_game_mgr_prt + 0xc),
        num_animals: get_from_memory::<i32>(zt_game_mgr_prt + 0x30),
        num_exhibits: get_from_memory::<i32>(zt_game_mgr_prt + 0x38),
        num_guests: get_from_memory::<i32>(zt_game_mgr_prt + 0x54),
        admissions_income_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x254),
        concessions_benefit_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x29c),
        recycling_benefit_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x340),
        // net_income: get_from_memory::<i32>(zt_game_mgr_prt + 0x404),
        income_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x404),
        income_expense_totals_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x44c),
        zoo_rating_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x464),
        unknown_array: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x4c4),
        construction_cost_by_month: get_from_memory::<[f32; 12]>(zt_game_mgr_prt + 0x824),
    }
}

fn log_zt_game_mgr(zt_game_mgr: zt_game_mgr) {
    info!("zoo_budget: {}", zt_game_mgr.zoo_budget);
    info!("num_animals: {}", zt_game_mgr.num_animals);
    info!("num_exhibits: {}", zt_game_mgr.num_exhibits);
    info!("num_guests: {}", zt_game_mgr.num_guests);
    info!("admissions_income_by_month: {:?}", zt_game_mgr.admissions_income_by_month);
    info!("concessions_benefit_by_month: {:?}", zt_game_mgr.concessions_benefit_by_month);
    info!("recycling_benefit_by_month: {:?}", zt_game_mgr.recycling_benefit_by_month);
    // info!("net_income: {}", zt_game_mgr.net_income);
    info!("income_by_month: {:?}", zt_game_mgr.income_by_month);
    info!("income_expense_totals_by_month: {:?}", zt_game_mgr.income_expense_totals_by_month);
    info!("zoo_rating_by_month: {:?}", zt_game_mgr.zoo_rating_by_month);
    info!("unknown_array: {:?}", zt_game_mgr.unknown_array);
    info!("construction_cost_by_month: {:?}", zt_game_mgr.construction_cost_by_month);
}