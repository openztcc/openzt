use syn::{
    punctuated::Punctuated, spanned::Spanned, BareFnArg, FnArg, Pat, Signature, Type, TypeBareFn,
};

use crate::parse::HookAttributeArgs;

pub fn fn_type(fn_sig: &Signature, hook_info: &HookAttributeArgs) -> Type {
    let mut args = Punctuated::new();
    let mut errs: Option<syn::Error> = None;
    for arg in &fn_sig.inputs {
        match arg {
            FnArg::Typed(arg) => args.push(BareFnArg {
                attrs: arg.attrs.clone(),
                name: None,
                ty: *arg.ty.clone(),
            }),
            FnArg::Receiver(_) => {
                let err = syn::Error::new(
                    arg.span(),
                    "`self` is not currently supported by this macro",
                );
                match &mut errs {
                    Some(errs) => errs.combine(err),
                    None => errs = Some(err),
                }
            }
        }
    }

    Type::BareFn(TypeBareFn {
        lifetimes: None, // TODO: maybe support lifetimes
        unsafety: hook_info.unsafety,
        abi: hook_info.abi.clone(),
        fn_token: fn_sig.fn_token,
        paren_token: fn_sig.paren_token,
        inputs: args,
        variadic: fn_sig.variadic.clone().map(|var| syn::BareVariadic {
            attrs: var.attrs,
            name: None,
            dots: var.dots,
            comma: var.comma,
        }),
        output: fn_sig.output.clone(),
    })
}

pub fn fn_arg_names(fn_sig: &Signature) -> Result<Vec<&Pat>, syn::Error> {
    let mut args = Vec::new();
    let mut errs: Option<syn::Error> = None;
    for arg in &fn_sig.inputs {
        match arg {
            FnArg::Typed(arg) => args.push(arg.pat.as_ref()),
            FnArg::Receiver(_) => {
                let err = syn::Error::new(
                    arg.span(),
                    "`self` is not currently supported by this macro",
                );
                match &mut errs {
                    Some(errs) => errs.combine(err),
                    None => errs = Some(err),
                }
            }
        }
    }
    if let Some(e) = errs {
        Err(e)
    } else {
        Ok(args)
    }
}

pub fn fn_types(fn_sig: &Signature) -> Result<Vec<&Type>, syn::Error> {
    let mut types = Vec::new();
    let mut errs: Option<syn::Error> = None;
    for arg in &fn_sig.inputs {
        match arg {
            FnArg::Typed(arg) => types.push(arg.ty.as_ref()),
            FnArg::Receiver(_) => {
                let err = syn::Error::new(
                    arg.span(),
                    "`self` is not currently supported by this macro",
                );
                match &mut errs {
                    Some(errs) => errs.combine(err),
                    None => errs = Some(err),
                }
            }
        }
    }
    if let Some(e) = errs {
        Err(e)
    } else {
        Ok(types)
    }
}
