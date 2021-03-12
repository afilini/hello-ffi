use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, TokenStreamExt};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_quote, BareFnArg, FnArg, Ident, ImplItem, ImplItemMethod, ItemFn, ItemStruct, Pat,
    PatIdent, PatType, Token, TypeBareFn,
};

use super::*;
use crate::types::*;

#[derive(Debug)]
pub struct C;

impl Lang for C {
    type Error = CError;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
        if let Some(pos) = function
            .attrs
            .iter()
            .position(|a| a.path.is_ident("destructor"))
        {
            // replace the type with `Destroy<T>`
            function.attrs.remove(pos);

            for input in &mut function.sig.inputs {
                match input {
                    FnArg::Typed(PatType { ty, .. }) => {
                        *ty = Box::new(parse_quote!( Destroy<#ty> ));
                    }
                    FnArg::Receiver(_) => {
                        return Err(CError::DestructorReceiverArgument(input.span()));
                    }
                }
            }
        }

        let ident = &function.sig.ident;

        let (mut args, input_conversion) = Self::convert_fn_args(function.sig.inputs.clone())?;

        let ExpandedReturn {
            ret,
            extra_args,
            conv: output_conversion,
        } = Return(function.sig.output.clone()).expand(
            &format_ident!("__output"),
            &format_ident!("__ptr_out"),
            Self::convert_output,
        )?;
        args.extend(extra_args);

        let block = &function.block;

        *function = parse_quote! {
            #[no_mangle]
            #[allow(non_snake_case)]
            pub extern "C" fn #ident(#args) #ret {
                use crate::{MapFrom, MapTo};
                use crate::c_destroy::Destroy;

                #input_conversion

                let __output = { #block };
                #output_conversion
            }
        };

        Ok(function.sig.ident.clone())
    }

    fn expose_mod(
        module: &mut ItemMod,
        mod_path: &Vec<Ident>,
        sub_items: Vec<ModuleItem>,
    ) -> Result<Ident, Self::Error> {
        module.vis = parse_quote!(pub);

        Ok(module.ident.clone())
    }

    fn expose_struct(
        structure: &mut ItemStruct,
        opts: Punctuated<ExposeStructOpts, Token![,]>,
        mod_path: &Vec<Ident>,
    ) -> Result<Ident, Self::Error> {
        if opts
            .iter()
            .find(|o| **o == ExposeStructOpts::Opaque)
            .is_none()
        {
            structure.attrs.push(parse_quote!(#[repr(C)]));
        }

        Ok(structure.ident.clone())
    }

    fn expose_impl(
        implementation: &mut ItemImpl,
        mod_path: &Vec<Ident>,
    ) -> Result<(), Self::Error> {
        for item in &mut implementation.items {
            match item {
                ImplItem::Method(ImplItemMethod {
                    sig,
                    vis,
                    attrs,
                    block,
                    ..
                }) => {
                    let mut as_fn = ItemFn {
                        sig: sig.clone(),
                        vis: vis.clone(),
                        attrs: attrs.clone(),
                        block: Box::new(block.clone()),
                    };
                    Self::expose_fn(&mut as_fn, mod_path)?;

                    *sig = as_fn.sig;
                    *vis = as_fn.vis;
                    *attrs = as_fn.attrs;
                    *block = *as_fn.block;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn convert_input(ty: Type) -> Result<Input, Self::Error> {
        if match_fixed_type(&ty, parse_quote!(String)) {
            Ok(Input::new_map_from(
                ty,
                vec![parse_quote!(*const libc::c_char)],
            ))
        } else if let Some(inner) = match_generic_type(&ty, parse_quote!(Vec)) {
            let inner = Self::convert_input(inner)?;
            let sources = inner
                .get_sources()
                .into_iter()
                .collect::<Punctuated<_, Comma>>(); // TODO: as_tuple() ?

            Ok(Input::new_map_from(
                ty,
                vec![parse_quote!(*const #sources), parse_quote!(usize)],
            ))
        } else if let Some(inner) = match_generic_type(&ty, parse_quote!(Destroy)) {
            let inner = Self::convert_input(inner)?;
            let sources = inner
                .get_sources()
                .into_iter()
                .collect::<Punctuated<_, Comma>>(); // TODO: as_tuple() ?

            Ok(Input::new_map_from(ty, vec![parse_quote!(*mut #sources)]))
        } else if let Type::BareFn(ref old_bare_fn) = ty {
            if !old_bare_fn.inputs.iter().all(|arg| arg.name.is_some()) {
                return Err(CError::UnnamedCallbackArguments(old_bare_fn.span()));
            }

            let mut new_bare_fn: TypeBareFn = parse_quote!(unsafe extern "C" fn());

            let (new_inputs, arg_conv): (Vec<_>, Vec<_>) = old_bare_fn
                .inputs
                .iter()
                .map(|arg| {
                    let arg_name = arg.name.clone().unwrap().0;
                    let converted =
                        CallbackArgument(arg.clone()).expand(&arg_name, Self::convert_output)?;

                    Ok((converted.args, converted.conv.into_inner()))
                })
                .collect::<Result<Vec<_>, Self::Error>>()?
                .into_iter()
                .unzip();

            let arg_conv = arg_conv.into_iter().flatten().collect::<TokenStream2>();

            new_bare_fn.inputs = new_inputs.into_iter().flatten().collect();
            let args_names = new_bare_fn
                .inputs
                .iter()
                .map(|arg| arg.name.clone().unwrap().0)
                .collect::<Punctuated<Ident, Comma>>();

            let ExpandedCallbackReturn {
                ret,
                conv: result_conv,
            } = CallbackReturn(old_bare_fn.output.clone())
                .expand(&format_ident!("result"), Self::convert_input)?;
            new_bare_fn.output = ret;

            let old_inputs = old_bare_fn.inputs.clone();
            Ok(Input::new_custom(
                ty,
                vec![new_bare_fn.into()],
                move |_, ident| {
                    let ts = quote! {
                        |#old_inputs| {
                            #arg_conv

                            let result = unsafe { #ident(#args_names) };
                            let result = { #result_conv };

                            result
                        }
                    };
                    ts.into()
                },
            ))
        } else {
            Ok(Input::new_unchanged(ty))
        }
    }

    fn convert_output(output: Type) -> Result<Output, Self::Error> {
        if output == parse_quote!(String) {
            Ok(Output::new_map_to_single(
                output,
                parse_quote!(*mut libc::c_char),
            ))
        } else if output == parse_quote!(Self) {
            Ok(Output::ByReference(Box::new(parse_quote!(*mut Self))))
        } else if let Some(inner) = match_generic_type(&output, parse_quote!(Vec)) {
            let inner = Self::convert_output(inner)?;
            let targets = inner
                .get_targets()
                .into_iter()
                .collect::<Punctuated<_, Comma>>(); // TODO: as_tuple() ?

            Ok(Output::new_map_to_suffix(
                output,
                vec![
                    (parse_quote!(*mut #targets), "arr".into()),
                    (parse_quote!(usize), "len".into()),
                ],
            ))
        } else {
            Ok(Output::new_unchanged(output))
        }
    }
}

#[derive(Debug)]
pub enum CError {
    Lang(LangError),

    UnnamedCallbackArguments(Span),
    DestructorReceiverArgument(Span),
}

impl fmt::Display for CError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CError {}

impl From<LangError> for CError {
    fn from(e: LangError) -> Self {
        CError::Lang(e)
    }
}
