use std::marker::PhantomData;

use retour::GenericDetour;

pub struct FunctionDef<T> {
    pub address: u32,
    function_type: PhantomData<T>,
}

impl<T> FunctionDef<T> where T: retour::Function {
    pub unsafe fn detour(self, target: T) -> Result<GenericDetour<T>, retour::Error> {
        GenericDetour::<T>::new(::retour::Function::from_ptr(self.address as *const ()), target)
    }
}

pub const LOAD_LANG_DLLS: FunctionDef<unsafe extern "thiscall" fn(u32) -> u32> = FunctionDef{address: 0x00537333, function_type: PhantomData};