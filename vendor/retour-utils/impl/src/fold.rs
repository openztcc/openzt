use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{fold::Fold, spanned::Spanned, Item, ItemFn, LitStr, Signature};

use crate::{
    crate_refs,
    helpers::{fn_arg_names, fn_type},
    parse::HookAttributeArgs,
};

pub struct Detours {
    module_name: LitStr,
    detours: Vec<DetourInfo>,
}

impl Detours {
    pub fn new(module_name: LitStr) -> Self {
        Self {
            module_name,
            detours: Vec::new(),
        }
    }

    pub fn generate_detour_decls(&self) -> Vec<Item> {
        self.detours
            .iter()
            .flat_map(|info| {
                if info.hook_attr.is_generic() {
                    info.get_generic_detour_items(&self.module_name)
                } else {
                    vec![info.get_static_detour()]
                }
            })
            .collect()
    }

    /// Returns the const expression containing the module name
    /// ```
    /// pub const MODULE_NAME: &str = "lua52.dll";
    /// ```
    pub fn get_module_name_decl(&self) -> Item {
        let module_name = &self.module_name;

        Item::Verbatim(quote_spanned! {self.module_name.span()=>
            #[allow(unused)]
            pub const MODULE_NAME: &str = #module_name;
        })
    }

    pub fn generate_init_detours(&self) -> Item {
        let krate_name = crate_refs::parent_crate();
        // Only initialize static detours by default
        let init_funcs: Vec<Item> = self
            .detours
            .iter()
            .filter(|func| !func.hook_attr.is_generic())
            .map(|func| func.generate_detour_init(&self.module_name))
            .collect();
        Item::Verbatim(quote::quote! {
            pub unsafe fn init_detours() -> Result<(), #krate_name::Error> {
                #(#init_funcs;)*

                Ok(())
            }
        })
    }
}

pub struct DetourInfo {
    pub hook_attr: HookAttributeArgs,
    pub fn_sig: Signature,
}

impl DetourInfo {
    fn get_generic_detour_items(&self, module_name: &LitStr) -> Vec<Item> {
        let vis = self.hook_attr.vis.clone();
        let detour_krate = crate_refs::retour_crate();
        let parent_krate = crate_refs::parent_crate();
        let detour_name = &self.hook_attr.detour_name;
        let fn_type_sig = fn_type(&self.fn_sig, &self.hook_attr);
        let orig_func_name = &self.fn_sig.ident;
        let arg_names = fn_arg_names(&self.fn_sig).unwrap();
        let arg_types = self.fn_sig.inputs.iter();
        
        // Storage variable name
        let storage_name = quote::format_ident!("{}_storage", detour_name);
        let enable_fn_name = quote::format_ident!("enable_{}", detour_name);
        let disable_fn_name = quote::format_ident!("disable_{}", detour_name);
        let call_original_fn_name = quote::format_ident!("call_original_{}", detour_name);
        
        
        // Generate the storage variable
        let storage_item = Item::Verbatim(quote_spanned! {self.hook_attr.span()=>
            #[allow(non_upper_case_globals)]
            static mut #storage_name: Option<::#detour_krate::GenericDetour<#fn_type_sig>> = None;
        });
        
        // Generate wrapper function declaration with correct ABI
        let wrapper_fn_decl = self.generic_wrapper_fn_decl();
        
        // Generate the enable function
        let enable_item = Item::Verbatim(quote_spanned! {self.hook_attr.span()=>
            #vis unsafe fn #enable_fn_name() -> Result<(), #parent_krate::Error> {
                if #storage_name.is_some() {
                    return Ok(()); // Already enabled
                }
                
                // Get the module handle and function address
                let handle = ::minidl::Library::load(::std::path::Path::new(MODULE_NAME))
                    .map_err(|_| ::#parent_krate::Error::ModuleNotLoaded)?;
                let addr = match {self.hook_attr.hook_info.get_lookup_data_new_fn(module_name)} {
                    ::#parent_krate::LookupData::Symbol { symbol, .. } => {
                        let c_symbol = ::std::ffi::CString::new(symbol)
                            .map_err(|_| ::#parent_krate::Error::ModuleNotLoaded)?;
                        let symbol_with_null = String::from_utf8(c_symbol.into_bytes_with_nul())
                            .map_err(|_| ::#parent_krate::Error::ModuleNotLoaded)?;
                        unsafe { handle.sym_opt(&symbol_with_null) }
                            .ok_or(::#parent_krate::Error::ModuleNotLoaded)?
                    }
                    ::#parent_krate::LookupData::Offset { offset, .. } => {
                        (handle.as_ptr() as usize + offset) as *const ()
                    }
                };
                
                // Create a wrapper function with the correct ABI
                #wrapper_fn_decl {
                    #orig_func_name(#(#arg_names),*)
                }
                
                let target_fn = ::#detour_krate::Function::from_ptr(addr);
                let mut detour = ::#detour_krate::GenericDetour::new(target_fn, __generic_detour_wrapper)?;
                detour.enable()?;
                #storage_name = Some(detour);
                Ok(())
            }
        });
        
        // Generate the disable function
        let disable_item = Item::Verbatim(quote_spanned! {self.hook_attr.span()=>
            #vis unsafe fn #disable_fn_name() {
                if let Some(mut detour) = #storage_name.take() {
                    let _ = detour.disable();
                }
            }
        });
        
        // Generate the call_original function
        let output_type = &self.fn_sig.output;
        let call_original_item = Item::Verbatim(quote_spanned! {self.hook_attr.span()=>
            #vis unsafe fn #call_original_fn_name(#(#arg_types),*) #output_type {
                if let Some(detour) = &#storage_name {
                    detour.call(#(#arg_names),*)
                } else {
                    panic!("Detour not enabled");
                }
            }
        });
        
        vec![storage_item, enable_item, disable_item, call_original_item]
    }
    
    fn get_static_detour(&self) -> Item {
        let vis = self.hook_attr.vis.clone();

        let detour_krate = crate_refs::retour_crate();
        let detour_name: &proc_macro2::Ident = &self.hook_attr.detour_name;
        let fn_type_sig = fn_type(&self.fn_sig, &self.hook_attr);
        let target_fn_decl = self.target_fn_decl();
        let arg_names = fn_arg_names(&self.fn_sig).unwrap();

        Item::Verbatim(quote_spanned! {self.hook_attr.span()=>
            #[allow(non_upper_case_globals)]
            #vis static #detour_name: ::#detour_krate::StaticDetour<#fn_type_sig> = {
                #[inline(never)]
                #[allow(unused_unsafe)]
                #target_fn_decl {
                    #[allow(unused_unsafe)]
                    (#detour_name.__detour())(#(#arg_names),*)
                }
                ::#detour_krate::StaticDetour::__new(__ffi_detour)
            };
        })
    }

    fn target_fn_decl(&self) -> TokenStream {
        let input_types = self.fn_sig.inputs.iter();
        // output includes the `->` in the return type
        let output_type = &self.fn_sig.output;
        let abi = &self.hook_attr.abi;
        let unsafety = &self.hook_attr.unsafety;

        quote::quote_spanned! {self.hook_attr.span()=>
            #unsafety #abi fn __ffi_detour(#(#input_types),*) #output_type
        }
    }
    
    fn generic_wrapper_fn_decl(&self) -> TokenStream {
        let input_types = self.fn_sig.inputs.iter();
        // output includes the `->` in the return type
        let output_type = &self.fn_sig.output;
        // Use the specified ABI or default to "C" for GenericDetour compatibility
        let abi = self.hook_attr.abi.as_ref()
            .map(|a| quote::quote! { #a })
            .unwrap_or_else(|| quote::quote! { extern "C" });
        let unsafety = &self.hook_attr.unsafety;

        quote::quote_spanned! {self.hook_attr.span()=>
            #unsafety #abi fn __generic_detour_wrapper(#(#input_types),*) #output_type
        }
    }

    fn generate_detour_init(&self, module_name: &LitStr) -> Item {
        let lookup_new_fn = (self.hook_attr.hook_info).get_lookup_data_new_fn(module_name);
        let detour_name = &self.hook_attr.detour_name;
        let orig_func_name = &self.fn_sig.ident;
        let parent_krate = crate_refs::parent_crate();
        let detour_krate = crate_refs::retour_crate();
        Item::Verbatim(quote_spanned! {self.hook_attr.span()=>
            ::#parent_krate::init_detour(
                #lookup_new_fn,
                |addr| {
                    #detour_name
                        .initialize(::#detour_krate::Function::from_ptr(addr), #orig_func_name)?
                        .enable()?;
                    Ok(())
                }
            )?
        })
    }
}

impl Fold for Detours {
    fn fold_item_fn(&mut self, item_fn: syn::ItemFn) -> syn::ItemFn {
        let mut attrs = Vec::new();

        for attr in item_fn.attrs {
            if !attr.path().is_ident("hook") {
                attrs.push(attr);
                continue;
            }
            let Ok(hook_attrs) = attr.parse_args::<HookAttributeArgs>() else {
                continue;
            };
            self.detours.push(DetourInfo {
                hook_attr: hook_attrs,
                fn_sig: item_fn.sig.clone(),
            })
        }
        ItemFn { attrs, ..item_fn }
    }
}
