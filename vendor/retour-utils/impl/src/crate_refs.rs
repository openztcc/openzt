use proc_macro2::Span;
use proc_macro_crate::{crate_name, FoundCrate};
use syn::Ident;

/// Get crate name for used for detours
///
/// Should detect if retour is renamed, like in the Cargo.toml
pub fn retour_crate() -> Ident {
    let found_crate = crate_name("retour").expect("retour is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => Ident::new("retour", Span::call_site()),
        FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
    }
}

/// Get crate name for the crate that this proc-macro belongs to
///
/// Should detect if crate is renamed, like in the Cargo.toml
pub fn parent_crate() -> Ident {
    let found_crate = crate_name("retour-utils").expect("retour-utils is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => Ident::new("retour_utils", Span::call_site()),
        FoundCrate::Name(name) => Ident::new(&name.replace('-', "_"), Span::call_site()),
    }
}
