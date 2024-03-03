class ZTGameMgr
{
public:

    ZTGameMgr();
    ~ZTGameMgr();
    void addCash(float);
	void ZTUIMainSetMoneyText();
	static void __fastcall addCash_Detour(void* ptr, float amount);
	static void BFUIMgrSetControlForeColor(void* ptr, int param_1, DWORD color);
	static void BFInternatSetMoneyText(int, int, char);
	static void init();
	static ZTGameMgr &shared_instance() {
		static ZTGameMgr instance;
		return instance;
	}

}; //Size: 0x10B4 4528

struct ZTGameMgr {
    vtable: u32, //0x10B0
    pad_0004: [u8; 8], //0x0004
    zoo_budget: f32, //0x000C
    pad_0010: [u8; 32], //0x0010
    num_animals: i32, //0x0030
    pad_0034: [u8; 4], //0x0034
    num_species: i32, //0x0038
    pad_003C: [u8; 24], //0x003C
    guest_count: i32, //0x0054
    pad_0058: [u8; 364], //0x0058
    animal_purchase_costs: [f32; 12], //0x01C4
    construction_costs: [f32; 12], //0x01F4
    pad_0224: [u8; 48], //0x0224
    admissions_income: [f32; 12], //0x0254
    concessions_benefits: [f32; 12], //0x0284
    pad_02B4: [u8; 336], //0x02B4
    zoo_profits: [f32; 12], //0x0404
    zoo_values: [f32; 12], //0x0434
    zoo_ratings: [f32; 12], //0x0464
    pad_0494: [u8; 384], //0x0494
    number_of_guests: [f32; 12], //0x0614
    pad_0644: [u8; 2928], //0x0644
} // Range: 0x10B0 Size: 4528 bytes

impl ZTGameMgr {
    pub fn add_cash(amount: f32) {
        info!("Adding ${:.2} to zoo budget", amount);
        self.zoo_budget += amount;
    }
    pub fn set_money_text(&mut self) {
        unsafe {
            ZTGameMgr::BFInternatSetMoneyText(0, 0, format!("${:.2}", self.zoo_budget).as_ptr() as *mut i8);
        }
    }
    pub fn init() {
        unsafe {
            ZTGameMgr::init();
        }
    }
}