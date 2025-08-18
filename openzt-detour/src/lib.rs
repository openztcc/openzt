pub mod gen;

use std::marker::PhantomData;

use retour::GenericDetour;

pub struct FunctionDef<T> {
    pub address: u32,
    function_type: PhantomData<T>,
}

impl<T> FunctionDef<T> where T: retour::Function {
    /// # Safety
    /// 
    /// This function will cause issues if the address or signature is not correct.
    pub unsafe fn detour(self, target: T) -> Result<GenericDetour<T>, retour::Error> {
        unsafe { GenericDetour::<T>::new(::retour::Function::from_ptr(self.address as *const ()), target) }
    }

    // TODO: Would be nice to have a `call` that calls the original function without having to detour it first.
    /// # Safety
    /// 
    /// This function will cause issues if the address is not correct
    pub unsafe fn original(&self) -> T {
        unsafe { ::retour::Function::from_ptr(self.address as *const ()) }
    }
}

// Re-export from generated definitions
pub use gen::bfapp::LOAD_STRING as BFAPP_LOADSTRING;
pub use gen::bfapp::LOAD_LANG_DLLS;
pub use gen::bfapp::WIN_MAIN as WINMAIN;
pub use gen::bfmap::GET_NEIGHBOR_1 as BFMAP_GET_NEIGHBOUR;
pub use gen::bfentity::GET_FOOTPRINT as BFENTITY_GET_FOOTPRINT;
pub use gen::bfentity::GET_BLOCKING_RECT as BFENTITY_GET_BLOCKING_RECT;
// TODO: BFENTITY_GET_BLOCKING_RECT_ZTPATH not found in gen - verify if this function exists
pub const BFENTITY_GET_BLOCKING_RECT_ZTPATH: FunctionDef<unsafe extern "thiscall" fn(u32, u32) -> u32> = FunctionDef{address: 0x004fbbee, function_type: PhantomData};
pub use gen::bfmap::TILE_TO_WORLD as BFMAP_TILE_TO_WORLD;
pub use gen::bfentity::IS_ON_TILE as BFENTITY_IS_ON_TILE;
pub use gen::zthabitat::GET_GATE_TILE_IN as ZTHABITAT_GET_GATE_TILE_IN;
pub use gen::ztmapview::CHECK_TANK_PLACEMENT as ZTMAPVIEW_CHECK_TANK_PLACEMENT;
pub use gen::bftile::GET_LOCAL_ELEVATION as BFTILE_GET_LOCAL_ELEVATION;
pub use gen::bfregistry::PTR_GET as BFREGISTRY_PRTGET;
pub use gen::bfmgr::REGISTERIT as BFREGISTRY_ADD;
// TODO: BFREGISTRY_ADDUI not found in gen - verify if this function exists
pub const BFREGISTRY_ADDUI: FunctionDef<unsafe extern "cdecl" fn(u32, u32) -> u32> = FunctionDef{address: 0x005774bf, function_type: PhantomData};
pub use gen::ztapp::UPDATE_SIM as ZTAPP_UPDATEGAME;
pub use gen::ztui_general::ENTITY_TYPE_IS_DISPLAYED as ZTUI_GENERAL_ENTITY_TYPE_IS_DISPLAYED;
pub use gen::ztui_expansionselect::SETUP as ZTUI_EXPANSIONSELECT_SETUP;
// TODO: BFUIMGR_DISPLAY_MESSAGE not found in gen with matching signature - verify function name/signature
pub const BFUIMGR_DISPLAY_MESSAGE: FunctionDef<unsafe extern "thiscall" fn(u32, u32, i32, u32, u32, bool, bool)> = FunctionDef{address: 0x0049ccc3, function_type: PhantomData};
pub use gen::standalone::LOAD_INI_DEBUG_SETTINGS as LOAD_DEBUG_SETTINGS_FROM_INI;
pub use gen::bfresource::ATTEMPT as BFRESOURCE_ATTEMPT;
pub use gen::bfresource::PREPARE as BFRESOURCE_PREPARE;
pub use gen::bfresourcemgr::CONSTRUCTOR as BFRESOURCEMGR_CONSTRUCTOR;
pub use gen::ztui_general::GET_INFO_IMAGE_NAME as ZTUI_GENERAL_GET_INFO_IMAGE_NAME;
pub use gen::bfversioninfo::GET_VERSION_STRING as BFVERSIONINFO_GET_VERSION_STRING;
pub use gen::bflog::LOG_MESSAGE as ZOOLOGGING_LOG;
pub use gen::ztui_general::GET_SELECTED_ENTITY as ZTUI_GET_SELECTED_ENTITY;
pub use gen::bfuimgr::GET_ELEMENT_0 as ZTUI_GET_ELEMENT;
