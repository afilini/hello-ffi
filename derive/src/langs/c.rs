use std::fmt;
use std::convert::TryFrom;

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, Span};
use quote::{TokenStreamExt, quote, format_ident};
use syn::{ItemFn, Ident, FnArg, parse_quote, Pat, PatType, PatIdent, ItemStruct, ImplItem, ImplItemMethod, Token, BareFnArg, TypeBareFn};
use syn::spanned::Spanned;
use syn::punctuated::Punctuated;

use crate::types::*;
use super::*;

#[derive(Debug)]
pub struct C;

impl Lang for C {
    type Error = CError;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
        let ident = &function.sig.ident;
        // let (mut inputs, input_conversion) = Self::convert_fn_args(function.sig.inputs.clone())?;

        let (mut args, input_conversion) = function.sig.inputs
            .clone().into_iter().map(|i| Argument(i).expand(Self::convert_input))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter().fold((Punctuated::<FnArg, Comma>::default(), TokenStream2::default()), |(mut fold_args, mut fold_conv), ExpandedArgument { args, conv }| {
                fold_args.extend(args);
                fold_conv.extend(conv.into_inner());

                (fold_args, fold_conv)
            });

        let ExpandedReturn { ret, extra_args, conv: output_conversion } = Return(function.sig.output.clone()).expand(&format_ident!("__output"), &format_ident!("__ptr_out"), Self::convert_output)?;
        args.extend(extra_args);

        let block = &function.block;

        *function = parse_quote! {
            #[no_mangle]
            pub extern "C" fn #ident(#args) #ret {
                use crate::{MapFrom, MapTo};

                #input_conversion

                let __output = { #block };
                #output_conversion

                __output
            }
        };

        Ok(function.sig.ident.clone())
    }

    fn expose_mod(module: &mut ItemMod, mod_path: &Vec<Ident>, sub_items: Vec<ModuleItem>) -> Result<Ident, Self::Error> {
        module.vis = parse_quote!(pub);

        Ok(module.ident.clone())
    }

    fn expose_struct(structure: &mut ItemStruct, opts: Punctuated<ExposeStructOpts, Token![,]>, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
        if opts.iter().find(|o| **o == ExposeStructOpts::Opaque).is_none() {
            structure.attrs.push(parse_quote!(#[repr(C)]));
        }

        Ok(structure.ident.clone())
    }

    fn expose_impl(implementation: &mut ItemImpl, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        for item in &mut implementation.items {
            match item {
                ImplItem::Method(ImplItemMethod { sig, vis, attrs, block, .. }) => {
                    let mut as_fn = ItemFn{ sig: sig.clone(), vis: vis.clone(), attrs: attrs.clone(), block: Box::new(block.clone()) };
                    Self::expose_fn(&mut as_fn, mod_path)?;

                    *sig = as_fn.sig;
                    *vis = as_fn.vis;
                    *attrs = as_fn.attrs;
                    *block = *as_fn.block;
                },
                _ => {}
            }
        }

        Ok(())
    }

    fn convert_input(ty: Type) -> Result<Input, Self::Error> {
        if match_fixed_type(&ty, parse_quote!(String)) {
            Ok(Input::new_map_from(ty, vec![parse_quote!(*const libc::c_char)]))
        } else if let Some(inner) = match_generic_type(&ty, parse_quote!(Vec)) {
            let inner = Self::convert_input(inner)?;
            let sources = inner.get_sources().into_iter().collect::<Punctuated<_, Comma>>();

            Ok(Input::new_map_from(ty, vec![parse_quote!(*const #sources), parse_quote!(usize)]))
        } else if let Type::BareFn(ref old_bare_fn) = ty {
            let mut new_bare_fn: TypeBareFn = parse_quote!(unsafe extern "C" fn());

            let mut arg_conv = TokenStream2::default();
            let (new_args, old_args): (Vec<_>, Punctuated<_, Comma>) = old_bare_fn.inputs.iter().enumerate().map(|(i, arg)| {
                let arg_name = format_ident!("arg_{}", i);

                let mut converted = Self::convert_output(arg.ty.clone())?;
                converted.set_arg_name(format_ident!("{}_out", arg_name));
                let converted = converted.expand(&arg_name);
                dbg!(&converted);

                let mut all_args = Vec::new();
                if converted.ty != parse_quote!{ () } {
                    // Skip null-type
                    all_args.push(BareFnArg{ attrs: vec![], name: Some((arg_name.clone(), Default::default())), ty: *converted.ty });
                }
                all_args.extend(converted.extra_args.into_iter().map(|fn_arg| parse_quote!(#fn_arg)));

                // if converted.extra_args.len() > 0 {
                //     // TODO: support extra_args
                //     unimplemented!("Types that require extra args are not supported as callback arguments");
                // }

                let mut old_arg = arg.clone();
                old_arg.name = Some((arg_name, Default::default()));

                arg_conv.extend(converted.conv.into_inner());

                Ok((all_args, old_arg))
            }).collect::<Result<Vec<_>, Self::Error>>()?.into_iter().unzip();

            new_bare_fn.inputs = new_args.into_iter().flatten().collect::<Punctuated<BareFnArg, Comma>>();
            let args_names = new_bare_fn.inputs.iter().map(|arg| arg.name.clone().unwrap().0).collect::<Punctuated<Ident, Comma>>();

            let return_type = old_bare_fn.output.as_type();
            let ExpandedInput { types: mut result_types, conv: result_conversion } = Self::convert_input(return_type)?.expand(&format_ident!("result"));
            match result_types.pop() {
                Some(item) if result_types.is_empty() => new_bare_fn.output = ReturnType::Type(Default::default(), item),
                _ => unimplemented!("Return types that are expanded to zero or more than one type are not supported as callback arguments"),
            }

            Ok(Input::new_custom(ty, vec![new_bare_fn.into()], move |_, ident| {
                let ts = quote! {
                    |#old_args| {
                        #arg_conv
                        
                        let result = unsafe { #ident(#args_names) };
                        let result = { #result_conversion };

                        result
                    }
                };
                ts.into()
            }))
        } else {
            Ok(Input::new_unchanged(ty))
        }

        // match arg {
        //     // FnArg::Receiver(receiver) => Ok(Argument::Unchanged),
        //     FnArg::Typed(typed) if typed.ty == parse_quote!(String) => Ok(Input::MapFrom(Box::new(parse_quote!(*const libc::c_char)), ident)),
        //     // _ => Ok(Input::Unchanged),
        //     _ => unimplemented!()
        // }

        // match dt {
        //     DataTypeIn::SelfRef => Ok((vec![parse_quote!(&self)], TokenStream::default())),
        //     DataTypeIn::SelfMutRef => Ok((vec![parse_quote!(&mut self)], TokenStream::default())),
        //     DataTypeIn::SelfValue => {
        //         let arg_name = arg_name.expect("Missing `arg_name`");

        //         let args = vec![parse_quote!(#arg_name: *mut Self)];
        //         let convert = (quote!{
        //             let #arg_name = unsafe { Box::from_raw(#arg_name) };
        //         }).into();

        //         Ok((args, convert))

        //     },
        //     DataTypeIn::String => {
        //         let arg_name = arg_name.expect("Missing `arg_name`");

        //         let args = vec![parse_quote!(#arg_name: *const libc::c_char)];
        //         let convert = (quote!{
        //             let #arg_name = {
        //                 unsafe {
        //                     std::ffi::CStr::from_ptr(#arg_name).to_str().expect("Invalid incoming string").to_string()
        //                 }
        //             };
        //         }).into();

        //         Ok((args, convert))
        //     },
        //     DataTypeIn::Callback(cb_args) => {
        //         let arg_name = arg_name.expect("Missing `arg_name`");
        //         let arg_name_orig = Ident::new(&format!("__orig_{}", arg_name.to_string()), arg_name.span());

        //         //unsafe extern "C" fn(*const c_void, u32) -> u32,
        //         let inputs = cb_args.inputs.into_iter().enumerate().map(|(i, ty)| {
        //             let ident = Ident::new(&format!("__arg_{}", i), ty.span());
        //             let pat = Pat::Ident(PatIdent{ ident, attrs: vec![], by_ref: None, mutability: None, subpat: None });

        //             let fn_arg = FnArg::Typed(PatType{ pat: Box::new(pat), ty: Box::new(ty), attrs: vec![], colon_token: Default::default() });
        //             fn_arg
        //         });

        //         // TODO: return type

        //         let (new_args, args_conversion) = Self::convert_fn_args(inputs.clone())?;
        //         let args_conversion: TokenStream2 = args_conversion.into();

        //         let inputs: Punctuated<FnArg, Comma> = inputs.collect();

        //         let args = vec![parse_quote!(#arg_name_orig: unsafe extern "C" fn(#new_args))];
        //         let convert = (quote!{
        //             fn #arg_name(#inputs) {
        //                 #args_conversion

        //                 #arg_name_orig()
        //             }

        //             // let #arg_name = {
        //             //     unsafe {
        //             //         std::ffi::CStr::from_ptr(#arg_name).to_str().expect("Invalid incoming string").to_string()
        //             //     }
        //             // };
        //         }).into();

        //         Ok((args, convert))
        //     },
        // }
    }

    fn convert_output(output: Type) -> Result<Output, Self::Error> {
        if output == parse_quote!(String) {
            Ok(Output::new_map_to(output, parse_quote!(*mut libc::c_char)))
        } else if output == parse_quote!(Self) {
            Ok(Output::ByReference(Box::new(parse_quote!(*mut *mut Self))))
        } else if let Some(inner) = match_generic_type(&output, parse_quote!(Vec)) {
            Ok(Output::ByReference(Box::new(parse_quote!(*mut *mut Vec::<#inner>))))
        } else {
            Ok(Output::new_unchanged(output))
        }

        // let output_type = match output {
        //     ReturnType::Default => {
        //         let out = parse_quote!( () );
        //         let conv = (quote!( () )).into();

        //         return Ok((out, vec![], conv));
        //     },
        //     ReturnType::Type(_, ty) => *ty,
        // };

        // match DataTypeOut::try_from(&output_type)? {
        //     DataTypeOut::String => {
        //         let out = parse_quote!(*mut libc::c_char);
        //         let conv = (quote! {
        //             let cstring = std::ffi::CString::new(output).expect("Invalid outgoing string");
        //             let ptr = cstring.as_ptr();
        //             std::mem::forget(cstring);

        //             ptr as *mut libc::c_char
        //         }).into();

        //         Ok((out, vec![], conv))
        //     },
        //     DataTypeOut::SelfValue => {
        //         let ident = Ident::new(&format!("__ptr_out_{}", nonce), Span::call_site());

        //         let out = parse_quote!( () );
        //         let extra_args = vec![parse_quote!(#ident: *mut *mut Self)];
        //         let conv = (quote! {
        //             unsafe {
        //                 *#ident = Box::into_raw(Box::new(output));
        //             }

        //             ()
        //         }).into();

        //         Ok((out, extra_args, conv))

        //     },
        // }
    }
}

#[derive(Debug)]
pub enum CError {
    Lang(LangError),
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
