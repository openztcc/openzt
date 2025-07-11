use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

#[proc_macro_derive(StoreSkipArrays)]
pub fn store_skip_arrays_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    let expanded = match &input.data {
        Data::Struct(data_struct) => {
            let field_stores = generate_field_stores(&data_struct.fields);
            let field_loads = generate_field_loads(&data_struct.fields);
            
            quote! {
                impl #impl_generics lrpc::Store for #name #ty_generics #where_clause {
                    fn store(&self, que: &mut lrpc::ByteQue) {
                        #field_stores
                    }
                    
                    fn restore(que: &mut lrpc::ByteQue) -> Self {
                        Self {
                            #field_loads
                        }
                    }
                }
            }
        }
        Data::Enum(_) => {
            panic!("StoreSkipArrays does not support enums yet");
        }
        Data::Union(_) => {
            panic!("StoreSkipArrays does not support unions");
        }
    };
    
    TokenStream::from(expanded)
}

fn is_array_type(ty: &Type) -> bool {
    if let Type::Array(_) = ty {
        true
    } else {
        false
    }
}

fn generate_field_stores(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(fields_named) => {
            let stores = fields_named.named.iter()
                .filter(|field| !is_array_type(&field.ty))
                .map(|field| {
                    let field_name = &field.ident;
                    quote! {
                        self.#field_name.store(que);
                    }
                });
            quote! { #(#stores)* }
        }
        Fields::Unnamed(fields_unnamed) => {
            let stores = fields_unnamed.unnamed.iter()
                .enumerate()
                .filter(|(_, field)| !is_array_type(&field.ty))
                .map(|(i, _)| {
                    let index = syn::Index::from(i);
                    quote! {
                        self.#index.store(que);
                    }
                });
            quote! { #(#stores)* }
        }
        Fields::Unit => quote! {},
    }
}

fn generate_field_loads(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(fields_named) => {
            let loads = fields_named.named.iter()
                .map(|field| {
                    let field_name = &field.ident;
                    let field_ty = &field.ty;
                    
                    if is_array_type(field_ty) {
                        // For arrays, use zeroed memory instead of Default
                        quote! {
                            #field_name: unsafe { std::mem::zeroed() },
                        }
                    } else {
                        quote! {
                            #field_name: lrpc::Store::restore(que),
                        }
                    }
                });
            quote! { #(#loads)* }
        }
        Fields::Unnamed(fields_unnamed) => {
            let loads = fields_unnamed.unnamed.iter()
                .map(|field| {
                    let field_ty = &field.ty;
                    
                    if is_array_type(field_ty) {
                        // For arrays, use zeroed memory instead of Default
                        quote! {
                            unsafe { std::mem::zeroed() },
                        }
                    } else {
                        quote! {
                            lrpc::Store::restore(que),
                        }
                    }
                });
            quote! { #(#loads)* }
        }
        Fields::Unit => quote! {},
    }
}