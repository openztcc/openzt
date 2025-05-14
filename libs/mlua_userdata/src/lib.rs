use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Attribute};

#[proc_macro_derive(MluaEntityFields)]
pub fn derive_mlua_entity_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // struct needs to have named fields
    let fields = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields_named) => fields_named.named,
            _ => unimplemented!("Only named fields are supported"),
        },
        _ => unimplemented!("Only structs are supported"),
    };

    // skip fields with #[mlua(skip)]
    let filtered_fields = fields.clone().into_iter().filter(|f| {
        f.vis.to_token_stream().to_string() == "pub"
            && !f.attrs.iter().any(|a| is_mlua_skip(a))
    });

    // generate getters and setters for each field
    let getters = filtered_fields.clone().map(|f| {
        let fname = &f.ident;
        let fname_str = fname.as_ref().unwrap().to_string();
        quote! {
            f.add_field_method_get(#fname_str, |_, this| Ok(this.get().#fname));
        }
    });

    let setters = filtered_fields.clone().map(|f| {
        let fname = &f.ident;
        let fname_str = fname.as_ref().unwrap().to_string();
        let ftype = &f.ty;
        quote! {
            f.add_field_method_set(#fname_str, |_, this, val: #ftype| {
                this.get_mut().#fname = val;
                Ok(())
            });
        }
    });

    // dynamically add methods
    let dyn_methods = quote! {
        fn add_methods<M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_method_mut("set_value", |_, this, (key, val): (String, String)| {
                this.get_mut().set_config(&key, &val).map(|_| ()).map_err(mlua::Error::external)
            });

            methods.add_method("get_value", |_, this, key: String| {
                this.get().get_value(&key).map_err(mlua::Error::external)
            });
        }
    };

    // generate the impl block for lua::UserData
    TokenStream::from(quote! {
        impl<'lua> mlua::UserData for EntityRef<#name>
        where
            #name: crate::EntityType,
        {
            fn add_fields<F: mlua::UserDataMethods<'lua, Self>>(f: &mut F) {
                #(#getters)*
                #(#setters)*
            }

            #dyn_methods
        }
    })
}

// helper for skip
fn is_mlua_skip(attr: &Attribute) -> bool {
    attr.path().is_ident("mlua") && attr.tokens.to_string().contains("skip")
}
