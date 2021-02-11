use std::fmt;
use std::convert::TryFrom;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{TokenStreamExt, quote};
use syn::{ItemFn, Ident, FnArg, parse_quote, Pat, PatType, PatIdent, Item};

use super::*;

#[derive(Debug)]
pub struct Python;

impl Lang for Python {
    type Error = PythonError;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        let ident = &function.sig.ident;
        let (inputs_fn_arg, input_conversion) = Self::convert_fn_args(function.sig.inputs.clone())?;
        let (output_type, output_conversion) = Self::convert_output(function.sig.output.clone())?;
        let block = &function.block;

        let mut inputs = TokenStream2::default();
        inputs.append_all(inputs_fn_arg);

        let input_conversion = TokenStream2::from(input_conversion);
        let output_conversion = TokenStream2::from(output_conversion);

        let ident_str = ident.to_string();

        *function = parse_quote! {
            #[pyfn(m, #ident_str)]
            fn #ident(#inputs) -> #output_type {
                #input_conversion

                let output = { #block };

                #output_conversion
            }
        };

        Ok(())
    }

    fn expose_mod(module: &mut ItemMod, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        let ident = &module.ident;
        let content = &mut module.content.as_mut().expect("Empty module").1;

        let mut content_tokens = TokenStream2::default();
        content_tokens.append_all(content);

        *module = parse_quote! {
            mod #ident {
                #[pyo3::prelude::pymodule]
                fn #ident(py: pyo3::Python, m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
                    
                    #content_tokens

                    Ok(())
                }
            }
        };

        Ok(())
    }

    fn convert_arg(arg: FnArg, dt: DataTypeIn, arg_name: Option<Ident>) -> Result<(Vec<FnArg>, TokenStream), Self::Error> {
        match dt {
            DataTypeIn::SelfRef => Ok((vec![parse_quote!(&self)], TokenStream::default())),
            DataTypeIn::SelfMutRef => Ok((vec![parse_quote!(&mut self)], TokenStream::default())),
            DataTypeIn::String => {
                let arg_name = arg_name.expect("Missing `arg_name`");

                Ok((vec![parse_quote!(#arg_name: String)], TokenStream::default()))
            },
        }
    }

    fn convert_output(output: ReturnType) -> Result<(Type, TokenStream), Self::Error> {
        let output_type = match output {
            ReturnType::Default => {
                let out = parse_quote!( () );
                let conv = (quote!( () )).into();

                return Ok((out, conv));
            },
            ReturnType::Type(_, ty) => *ty,
        };

        match DataTypeOut::try_from(&output_type)? {
            DataTypeOut::String => {
                let out = parse_quote!(String);
                let conv = (quote! {
                    output
                }).into();

                Ok((out, conv))
            }
        }
    }
}

#[derive(Debug)]
pub enum PythonError {
    Lang(LangError),
}

impl fmt::Display for PythonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for PythonError {}

impl From<LangError> for PythonError {
    fn from(e: LangError) -> Self {
        PythonError::Lang(e)
    }
}
