use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{parse::Parse, token::Unsafe, Abi, Ident, LitInt, LitStr, Token, Visibility};

use crate::crate_refs::parent_crate;

pub mod kw {
    syn::custom_keyword!(hook);
    syn::custom_keyword!(offset);
    syn::custom_keyword!(symbol);
    syn::custom_keyword!(generic);
}

pub struct HookAttributeArgs {
    pub vis: Visibility,
    pub unsafety: Option<Unsafe>,
    pub abi: Option<Abi>,
    pub detour_name: Ident,
    pub comma: Token![,],
    pub hook_info: HookArg,
    pub generic: Option<(Token![,], kw::generic)>,
}

impl Parse for HookAttributeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let vis = input.parse()?;
        let unsafety = input.parse()?;
        let abi = input.parse()?;
        let detour_name = input.parse()?;
        let comma = input.parse()?;
        let hook_info = input.parse()?;
        
        // Check for optional ", generic" at the end
        let generic = if input.peek(Token![,]) && input.peek2(kw::generic) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };
        
        Ok(Self {
            vis,
            unsafety,
            abi,
            detour_name,
            comma,
            hook_info,
            generic,
        })
    }
}

impl HookAttributeArgs {
    pub fn is_generic(&self) -> bool {
        self.generic.is_some()
    }
}

impl ToTokens for HookAttributeArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.detour_name.to_tokens(tokens);
        self.comma.to_tokens(tokens);
        self.hook_info.to_tokens(tokens);
        if let Some((comma, generic)) = &self.generic {
            comma.to_tokens(tokens);
            generic.to_tokens(tokens);
        }
    }
}

pub enum HookArg {
    Offset {
        offset_token: kw::offset,
        eq: Token![=],
        value: LitInt,
    },
    Symbol {
        symbol_token: kw::symbol,
        eq: Token![=],
        value: LitStr,
    },
}

impl HookArg {
    pub fn get_lookup_data_new_fn(&self, module_name: &LitStr) -> TokenStream {
        let krate_name = parent_crate();
        match self {
            Self::Offset { value, .. } => {
                quote::quote! {
                    ::#krate_name::LookupData::from_offset(#module_name, #value)
                }
            }
            Self::Symbol { value, .. } => {
                quote::quote! {
                    ::#krate_name::LookupData::from_symbol(#module_name, #value)
                }
            }
        }
    }
}

impl Parse for HookArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::offset) {
            Ok(Self::Offset {
                offset_token: input.parse::<kw::offset>()?,
                eq: input.parse()?,
                value: input.parse()?,
            })
        } else if lookahead.peek(kw::symbol) {
            Ok(Self::Symbol {
                symbol_token: input.parse::<kw::symbol>()?,
                eq: input.parse()?,
                value: input.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for HookArg {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            HookArg::Offset {
                offset_token,
                eq,
                value,
            } => {
                offset_token.to_tokens(tokens);
                eq.to_tokens(tokens);
                value.to_tokens(tokens);
            }
            HookArg::Symbol {
                symbol_token,
                eq,
                value,
            } => {
                symbol_token.to_tokens(tokens);
                eq.to_tokens(tokens);
                value.to_tokens(tokens);
            }
        }
    }
}
