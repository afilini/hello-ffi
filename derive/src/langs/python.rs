use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::punctuated::Punctuated;
use syn::{
    parse_quote, Attribute, FnArg, Ident, ImplItem, ImplItemMethod, Item, ItemFn, Pat, PatIdent,
    PatType, Token, TraitItem, TraitItemMethod,
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
                use crate::mapping::{MapTo, MapFrom};
                use crate::langs::*;

                #input_conversion

                let mut block_closure = move || { #block };
                let __output = block_closure();

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
                ModuleItem::Trait(ident) => {
                    quote! {
                        m.add_class::<#ident>()?;
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
            pub mod #ident {
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
        let attr = if opts
            .iter()
            .find(|o| **o == ExposeStructOpts::Subclass)
            .is_some()
        {
            parse_quote!( #[pyo3::prelude::pyclass(subclass)] )
        } else {
            parse_quote!( #[pyo3::prelude::pyclass] )
        };

        structure.attrs.push(attr);

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
            if let ImplItem::Method(ImplItemMethod {
                sig, attrs, block, ..
            }) = item
            {
                let ident = &sig.ident;

                let (mut args, input_conversion) = Self::convert_fn_args(sig.inputs.clone())?;
                let ExpandedReturn {
                    ret,
                    extra_args,
                    conv: output_conversion,
                } = Return(sig.output.clone()).expand(
                    &format_ident!("__output"),
                    &format_ident!("__ptr_out"),
                    Self::convert_output,
                )?;
                args.extend(extra_args);

                sig.inputs = args;
                sig.output = ret;
                block.stmts = parse_quote! {
                    use crate::mapping::{MapTo, MapFrom};
                    use crate::langs::*;

                    #input_conversion

                    let mut block_closure = move || { #block };
                    let __output = block_closure();

                    #output_conversion
                };

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

    fn expose_trait(
        tr: &mut ItemTrait,
        mod_path: &Vec<Ident>,
        extra: &mut Vec<Item>,
    ) -> Result<Ident, Self::Error> {
        let ident = tr.ident.clone();

        let mut methods = vec![];
        for item in &mut tr.items {
            if let TraitItem::Method(TraitItemMethod { attrs, sig, .. }) = item {
                let ident = &sig.ident;

                let expose_trait_opts = match attrs.iter().position(
                    |attr| matches!(attr.path.get_ident(), Some(s) if s == "expose_trait"),
                ) {
                    Some(pos) => {
                        let attr = attrs.remove(pos);
                        attr.parse_args_with(
                            Punctuated::<ExposeTraitOption, Comma>::parse_separated_nonempty,
                        )
                        .map_err(LangError::ExposeTraitAttrError)?
                    }
                    None => Default::default(),
                };
                let original_ident = expose_trait_opts
                    .iter()
                    .find_map(|opt| match opt {
                        ExposeTraitOption::Original(_, i) => Some(Ident::new(&i.value(), i.span())),
                        _ => None,
                    })
                    .unwrap_or(ident.clone());

                let mut inputs = sig.inputs.iter().cloned().collect::<Vec<_>>();
                // inputs[0] = parse_quote!(this: &pyo3::PyObject);

                // let output = &sig.output;
                // let ty: Type = parse_quote!(fn(#(#inputs),*) #output);
                // let converted = Self::convert_input(ty)?.expand(ident);

                let inner_ident = format_ident!("rust_{}", original_ident);

                methods.push((sig, inner_ident, original_ident));
            }
        }
        let trait_struct_ident = format_ident!("{}Struct", ident);
        let supertrait = &tr.supertraits[0];
        let mut trait_struct: ItemStruct = parse_quote! {
            pub struct #trait_struct_ident {
                native: Option<Box<dyn #supertrait + Send>>,
                #[pyo3(set)]
                python: Option<pyo3::PyObject>,
            }
        };
        Self::expose_struct(
            &mut trait_struct,
            vec![ExposeStructOpts::Subclass].into_iter().collect(),
            mod_path,
        )?;
        extra.push(trait_struct.into());

        let wrap_fns = methods
            .iter()
            .map(|(sig, inner_ident, original_ident)| {
                let inner_ident_str = inner_ident.to_string();
                let output = &sig.output;
                let inputs = sig.inputs.iter();
                let arg_names = sig.inputs.iter().filter_map(|arg| match arg {
                    FnArg::Receiver(_) => None,
                    FnArg::Typed(PatType { pat, .. }) => Some(pat.clone()),
                }).collect::<Vec<_>>();

                let (map_output, out_ty) = match output {
                    ReturnType::Type(_, ty) => (quote!{ .extract(py)? }, quote! { #ty }),
                    _ => (quote! { ; Ok(()) }, quote!{ () }),
                };

                    quote! {
                        pub fn #inner_ident(#(#inputs),*) #output {
                            if let Some(native) = &self.native {
                                native.#original_ident(#(#arg_names),*)
                            } else if let Some(python) = &self.python {
                                pyo3::prelude::Python::with_gil(|py| -> pyo3::PyResult<#out_ty> {
                                    Ok(python.call_method1(py, #inner_ident_str, (#(#arg_names),* ,))?#map_output)
                                }).expect("Python call failed")
                            } else {
                                panic!("`self` reference not found. In your subclass constructor add: `self.python = self`")
                            }
                        }
                    }
            });
        let mut impl_block: ItemImpl = parse_quote! {
            impl #trait_struct_ident {
                #[constructor]
                pub fn new() -> Self {
                    #trait_struct_ident {
                        native: None,
                        python: None,
                    }
                }

                #(#wrap_fns)*
            }
        };
        Self::expose_impl(&mut impl_block, mod_path)?;
        extra.push(impl_block.into());

        // Impl the trait on the trait structure
        let impl_methods = methods.iter().map(|(sig, inner_ident, original_ident)| {
            let call_args = sig.inputs.iter().filter_map(|arg| match arg {
                FnArg::Receiver(_) => None,
                FnArg::Typed(PatType { pat, .. }) => Some(pat.to_token_stream()),
            });

            quote! {
                #sig {
                    self.#inner_ident(#(#call_args),*)
                }
            }
        });
        let impl_on_trait_struct: ItemImpl = parse_quote! {
            impl #ident for #trait_struct_ident {
                #(#impl_methods)*
            }
        };
        extra.push(impl_on_trait_struct.into());

        let into_trait_struct: ItemImpl = parse_quote! {
            impl<T: 'static + #supertrait + Sized + Send> crate::langs::IntoTraitStruct for T {
                type Target = #trait_struct_ident;

                fn into_trait_struct(self) -> Self::Target {
                    #trait_struct_ident {
                        native: Some(Box::new(self)),
                        python: None,
                    }
                }
            }
        };
        extra.push(into_trait_struct.into());

        Ok(trait_struct_ident)
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
        } else if let Some(inner) = match_generic_type(&ty, parse_quote!(Vec)) {
            match inner.as_slice() {
                &[Type::Reference(ref reference)] => {
                    let inner_ty = &reference.elem;
                    Ok(Input::new_custom(
                        ty.clone(),
                        vec![parse_quote!(Vec<pyo3::PyRef<#inner_ty>>)],
                        move |_, ident| {
                            let ts = quote! {
                                {
                                    use std::ops::Deref;
                                    #ident.iter().map(|r| r.deref()).collect::<#ty>()
                                }
                            };
                            ts.into()
                        },
                    ))
                }
                _ => Ok(Input::new_unchanged(ty)),
            }
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
