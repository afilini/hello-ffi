use std::fmt;
use std::convert::TryFrom;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{TokenStreamExt, quote};
use syn::{ItemFn, Ident, FnArg, parse_quote, Pat, PatType, PatIdent, ItemStruct};

use super::*;

#[derive(Debug)]
pub struct C;

impl Lang for C {
    type Error = CError;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        let ident = &function.sig.ident;
        let (mut inputs, input_conversion) = Self::convert_fn_args(function.sig.inputs.clone())?;
        let (output_type, extra_args, output_conversion) = Self::convert_output(function.sig.output.clone())?;
        let block = &function.block;

        inputs.extend(extra_args);

        let input_conversion = TokenStream2::from(input_conversion);
        let output_conversion = TokenStream2::from(output_conversion);

        *function = parse_quote! {
            #[no_mangle]
            pub extern "C" fn #ident(#inputs) -> #output_type {
                #input_conversion

                let output = { #block };

                #output_conversion
            }
        };

        Ok(())
    }

    fn expose_mod(module: &mut ItemMod, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        module.vis = parse_quote!(pub);

        Ok(())
    }

    fn expose_struct(structure: &mut ItemStruct, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        dbg!(&structure);
        structure.attrs.push(parse_quote!(#[repr(C)]));

        Ok(())
    }

    fn convert_arg(arg: FnArg, dt: DataTypeIn, arg_name: Option<Ident>) -> Result<(Vec<FnArg>, TokenStream), Self::Error> {
        match dt {
            DataTypeIn::SelfRef => Ok((vec![parse_quote!(&self)], TokenStream::default())),
            DataTypeIn::SelfMutRef => Ok((vec![parse_quote!(&mut self)], TokenStream::default())),
            DataTypeIn::SelfValue => {
                let arg_name = arg_name.expect("Missing `arg_name`");

                let args = vec![parse_quote!(#arg_name: *mut Self)];
                let convert = (quote!{
                    let #arg_name = unsafe { Box::from_raw(#arg_name) };
                }).into();

                Ok((args, convert))

            },
            DataTypeIn::String => {
                let arg_name = arg_name.expect("Missing `arg_name`");

                let args = vec![parse_quote!(#arg_name: *const libc::c_char)];
                let convert = (quote!{
                    let #arg_name = {
                        unsafe {
                            std::ffi::CStr::from_ptr(#arg_name).to_str().expect("Invalid incoming string").to_string()
                        }
                    };
                }).into();

                Ok((args, convert))
            },
        }
    }

    fn convert_output(output: ReturnType) -> Result<(Type, Vec<FnArg>, TokenStream), Self::Error> {
        let output_type = match output {
            ReturnType::Default => {
                let out = parse_quote!( () );
                let conv = (quote!( () )).into();

                return Ok((out, vec![], conv));
            },
            ReturnType::Type(_, ty) => *ty,
        };

        match DataTypeOut::try_from(&output_type)? {
            DataTypeOut::String => {
                let out = parse_quote!(*mut libc::c_char);
                let conv = (quote! {
                    let cstring = std::ffi::CString::new(output).expect("Invalid outgoing string");
                    let ptr = cstring.as_ptr();
                    std::mem::forget(cstring);

                    ptr as *mut libc::c_char
                }).into();

                Ok((out, vec![], conv))
            },
            DataTypeOut::SelfValue => {
                let out = parse_quote!( () );
                let extra_args = vec![parse_quote!(__ptr_out: *mut *mut Self)];
                let conv = (quote! {
                    unsafe {
                        *__ptr_out = Box::into_raw(Box::new(output));
                    }

                    ()
                }).into();

                Ok((out, extra_args, conv))

            },
        }
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
