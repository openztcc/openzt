use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{fold::Fold, spanned::Spanned, ItemMod, LitStr};

use crate::fold::Detours;

pub fn expand(mod_block: ItemMod, attribute_meta: LitStr) -> Result<TokenStream, syn::Error> {
    let mut detours = Detours::new(attribute_meta);
    let mut result = detours.fold_item_mod(mod_block);

    let Some((_, content)) = result.content.as_mut() else {
        return Err(syn::Error::new(result.span(), "Could not get content inside `mod`"))
    };
    content.push(detours.get_module_name_decl());
    let decls = detours.generate_detour_decls();
    content.extend(decls);
    content.push(detours.generate_init_detours());

    Ok(result.to_token_stream())
}
