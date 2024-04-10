extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(FieldAccessorAsString)]
pub fn set_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("Expected a struct with named fields"),
        },
        _ => panic!("Expected a struct"),
    };

    let mut field_names = vec![];
    let mut field_types = vec![];

    for field in fields {
        let field_name = field.ident.as_ref().expect("Expected field name");
        let field_type = &field.ty;

        field_names.push(field_name);
        field_types.push(field_type);
    }

    let expanded = quote! {
        impl #struct_name {
            fn set_field(&mut self, field_name: &str, value: &str) -> Result<(), String> {
                match field_name {
                    #(stringify!(#field_names) => {
                        match value.parse() {
                            Ok(value) => {
                                self.#field_names = value;
                                Ok(())
                            },
                            Err(err) => Err(format!("Failed to parse value: {}", err)),
                        }
                    },)*
                    _ => Err("Unknown field name"),
                }
            }
            fn get_field(&self, field_name: &str) -> String {
                match field_name {
                    #(stringify!(#field_names) => self.#field_names.to_string(),)*
                    _ => panic!("Unknown field name: {}", field_name),
                }
            }
            fn is_field(&self, field_name: &str) -> bool {
                match field_name {
                    #(stringify!(#field_names) => true,)*
                    _ => false,
                }
            }
        }
    };

    expanded.into()
}
