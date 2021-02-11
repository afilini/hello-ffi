use std::fmt;
use std::convert::TryFrom;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{TokenStreamExt, ToTokens, quote};
use syn::{ItemFn, Ident, FnArg, parse_quote, Pat, PatType, PatIdent, Item, Attribute, ImplItemMethod, ImplItem, Token};
use syn::punctuated::Punctuated;

use super::*;

#[derive(Debug)]
pub struct Python;

impl Lang for Python {
    type Error = PythonError;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
        if mod_path.is_empty() {
            return Err(PythonError::NakedFunction);
        }

        let ident = &function.sig.ident;
        let (mut inputs, input_conversion) = Self::convert_fn_args(function.sig.inputs.clone())?;
        let (output_type, extra_args, output_conversion) = Self::convert_output(function.sig.output.clone())?;
        let block = &function.block;

        inputs.extend(extra_args);

        let input_conversion = TokenStream2::from(input_conversion);
        let output_conversion = TokenStream2::from(output_conversion);

        let ident_str = ident.to_string();

        *function = parse_quote! {
            #[pyo3::prelude::pyfunction]
            fn #ident(#inputs) -> #output_type {
                #input_conversion

                let output = { #block };

                #output_conversion
            }
        };

        Ok(function.sig.ident.clone())
    }

    fn expose_mod(module: &mut ItemMod, mod_path: &Vec<Ident>, sub_items: Vec<ModuleItem>) -> Result<Ident, Self::Error> {
        let ident = &module.ident;
        let content = &mut module.content.as_mut().expect("Empty module").1;

        let mut content_tokens = TokenStream2::default();
        content_tokens.append_all(content);

        let mut export_tokens = TokenStream2::default();
        for sub_item in sub_items {
            let tokens = match sub_item {
                ModuleItem::Function(ident) => {
                    quote! {
                        m.add_function(pyo3::wrap_pyfunction!(#ident, m)?)?;
                    }
                },
                ModuleItem::Structure(ident) => {
                    quote! {
                        m.add_class::<#ident>()?;
                    }
                },
                ModuleItem::Module(ident) => {
                    let ident_str = ident.to_string();
                    quote! {
                        let submod = pyo3::types::PyModule::new(py, #ident_str)?;
                        #ident::#ident(py, submod)?;
                        m.add_submodule(submod)?;
                    }
                },
            };

            export_tokens.extend(tokens);
        }

        let mut extra_attrs = TokenStream2::default();
        if mod_path.len() == 1 {
            let attr: Attribute = parse_quote!( #[pyo3::prelude::pymodule] );
            extra_attrs.append_all(&[attr]);
        }

        *module = parse_quote! {
            mod #ident {
                #extra_attrs
                pub(super) fn #ident(py: pyo3::Python, m: &pyo3::types::PyModule) -> pyo3::PyResult<()> {
                    #export_tokens
                    Ok(())
                }
                #content_tokens
            }
        };

        Ok(module.ident.clone())
    }

    fn expose_struct(structure: &mut ItemStruct, opts: Punctuated<ExposeStructOpts, Token![,]>, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
        structure.attrs.push(parse_quote!( #[pyo3::prelude::pyclass] ));

        Ok(structure.ident.clone())
    }

    fn expose_impl(implementation: &mut ItemImpl, mod_path: &Vec<Ident>) -> Result<(), Self::Error> {
        implementation.attrs.push(parse_quote!( #[pyo3::prelude::pymethods] ));

        // remove items marked as "destructors" because pyo3 handles them automatically
        implementation.items.retain(|item| {
            if let ImplItem::Method(ImplItemMethod { sig, attrs, .. }) = item {
                if let Some(pos) = attrs.iter().position(|a| a.path.is_ident("destructor")) {
                    return false;
                }
            }

            true
        });

        for item in &mut implementation.items {
            if let ImplItem::Method(ImplItemMethod { sig, attrs, .. }) = item {
                if let Some(pos) = attrs.iter().position(|a| a.path.is_ident("constructor")) {
                    attrs.remove(pos);
                    attrs.push(parse_quote!( #[new] ));

                    continue;
                }

                match sig.inputs.first() {
                    // the first argument is not some kind of "self", so this is a static method
                    None | Some(FnArg::Typed(_)) => attrs.push(parse_quote!( #[staticmethod] )),
                    _ => {},
                }

            }
        }

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
            DataTypeIn::SelfValue => {
                let arg_name = arg_name.expect("Missing `arg_name`");

                Ok((vec![parse_quote!(#arg_name: Self)], TokenStream::default()))
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
                let out = parse_quote!(String);
                let conv = (quote! {
                    output
                }).into();

                Ok((out, vec![], conv))
            }
            DataTypeOut::SelfValue => {
                let out = parse_quote!(Self);
                let conv = (quote! {
                    output
                }).into();

                Ok((out, vec![], conv))
            }
        }
    }
}

#[derive(Debug)]
pub enum PythonError {
    NakedFunction,

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
