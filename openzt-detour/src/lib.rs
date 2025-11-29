use std::marker::PhantomData;

use retour::GenericDetour;

pub mod gen;

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

// World manager hooks
pub const BFENTITY_GET_BLOCKING_RECT_ZTPATH: FunctionDef<unsafe extern "thiscall" fn(u32, u32) -> u32> = FunctionDef{address: 0x004fbbee, function_type: PhantomData};

// TODO: Actually UIElement::registerit
pub const BFREGISTRY_ADDUI: FunctionDef<unsafe extern "cdecl" fn(u32, u32) -> u32> = FunctionDef{address: 0x005774bf, function_type: PhantomData};

// Experimental
pub const BFUIMGR_DISPLAY_MESSAGE: FunctionDef<unsafe extern "thiscall" fn(u32, u32, i32, u32, u32, bool, bool)> = FunctionDef{address: 0x0049ccc3, function_type: PhantomData};

// Settings
pub const LOAD_DEBUG_SETTINGS_FROM_INI: FunctionDef<unsafe extern "cdecl" fn() -> u32> = FunctionDef{address: 0x00579f4c, function_type: PhantomData};

// Resource manager
pub const BFRESOURCE_ATTEMPT: FunctionDef<unsafe extern "thiscall" fn(u32, u32) -> u8> = FunctionDef{address: 0x00403891, function_type: PhantomData};
pub const BFRESOURCE_PREPARE: FunctionDef<unsafe extern "thiscall" fn(u32, u32) -> u8> = FunctionDef{address: 0x004047f4, function_type: PhantomData};
pub const BFRESOURCEMGR_CONSTRUCTOR: FunctionDef<unsafe extern "thiscall" fn(u32) -> u32> = FunctionDef{address: 0x0052903f, function_type: PhantomData};
pub const ZTUI_GENERAL_GET_INFO_IMAGE_NAME: FunctionDef<unsafe extern "cdecl" fn(u32) -> u32> = FunctionDef{address: 0x004f85d2, function_type: PhantomData};

// Version
pub const BFVERSIONINFO_GET_VERSION_STRING: FunctionDef<unsafe extern "cdecl" fn(u32, u32, u32) -> u32> = FunctionDef{address: 0x004bdfd4, function_type: PhantomData};

// Logging
pub const ZOOLOGGING_LOG: FunctionDef<unsafe extern "cdecl" fn(u32, u32, u32, u8, u32, u32, u32)> = FunctionDef{address: 0x00401363, function_type: PhantomData};

// UI
pub const ZTUI_GET_SELECTED_ENTITY: FunctionDef<unsafe extern "stdcall" fn() -> u32> = FunctionDef{address: 0x00410f84, function_type: PhantomData};
pub const ZTUI_GET_ELEMENT: FunctionDef<unsafe extern "thiscall" fn(u32, u32) -> u32> = FunctionDef{address: 0x0040157d, function_type: PhantomData};
