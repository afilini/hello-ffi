use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::punctuated::Punctuated;
use syn::{
    parse_quote, Attribute, FnArg, Ident, ImplItem, ImplItemMethod, Item, ItemFn, Pat, PatIdent,
    PatType, Token,
};

use super::*;
use crate::types::*;

#[derive(Debug)]
pub struct Python;

impl Lang for Python {
    type Error = PythonError;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
        if mod_path.is_empty() {
            return Err(PythonError::NakedFunction);
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

        let ident_str = ident.to_string();
        *function = parse_quote! {
            #[pyo3::prelude::pyfunction]
            fn #ident(#args) #ret {
                use crate::{MapTo, MapFrom};

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
                }
                ModuleItem::Structure(ident) => {
                    quote! {
                        m.add_class::<#ident>()?;
                    }
                }
                ModuleItem::Module(ident) => {
                    let ident_str = ident.to_string();
                    quote! {
                        let submod = pyo3::types::PyModule::new(py, #ident_str)?;
                        #ident::#ident(py, submod)?;
                        m.add_submodule(submod)?;
                    }
                }
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

    fn expose_struct(
        structure: &mut ItemStruct,
        opts: Punctuated<ExposeStructOpts, Token![,]>,
        mod_path: &Vec<Ident>,
    ) -> Result<Ident, Self::Error> {
        structure
            .attrs
            .push(parse_quote!( #[pyo3::prelude::pyclass] ));

        Ok(structure.ident.clone())
    }

    fn expose_impl(
        implementation: &mut ItemImpl,
        mod_path: &Vec<Ident>,
    ) -> Result<(), Self::Error> {
        implementation
            .attrs
            .push(parse_quote!( #[pyo3::prelude::pymethods] ));

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
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn convert_input(ty: Type) -> Result<Input, Self::Error> {
        if let Type::BareFn(ref bare_fn) = ty {
            let inputs = bare_fn.inputs.clone();
            let output = bare_fn.output.clone();

            let args_names = bare_fn
                .inputs
                .iter()
                .map(|arg| arg.name.clone().unwrap().0)
                .collect::<Punctuated<Ident, Comma>>();

            Ok(Input::new_custom(
                ty,
                vec![parse_quote!(crate::python_callback::PyCb<'_>)],
                move |_, ident| {
                    let ts = quote! {
                        |#inputs| #output {
                            #ident.call1( (#args_names) ).unwrap().extract().unwrap()
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
        Ok(Output::new_unchanged(output))
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
