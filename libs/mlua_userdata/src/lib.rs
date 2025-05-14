// macro that expands to an mlua UserData implementation for a struct
// to avoid repeat boilerplate in large struct definitions like entitytypes
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(MluaUserData)]
pub fn derive_mlua_userdata(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // check if struct has named fields
    let fields = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields_named) => fields_named.named,
            _ => unimplemented!("Only named fields are supported"),
        },
        _ => unimplemented!("Only structs are supported"),
    };

    // add field getter methods to lua userdata
    let getters = fields.iter().map(|f| {
        let fname = &f.ident;
        let fname_str = fname.as_ref().unwrap().to_string();
        quote! {
            fields.add_field_method_get(#fname_str, |_, this| Ok(this.#fname.clone()));
        }
    });

    // add field setter methods to lua userdata
    let setters = fields.iter().map(|f| {
        let fname = &f.ident;
        let fname_str = fname.as_ref().unwrap().to_string();
        let ftype = &f.ty;
        quote! {
            fields.add_field_method_set(#fname_str, |_, this, val: #ftype| {
                this.#fname = val;
                Ok(())
            });
        }
    });

    TokenStream::from(quote! {
        impl mlua::UserData for #name {
            fn add_fields<'lua, F: mlua::UserDataMethods<'lua, Self>>(fields: &mut F) {
                #(#getters)*
                #(#setters)*
            }
        }
    })
}