use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_quote, BareFnArg, Field, Fields, FieldsNamed, FnArg, Ident, ImplItem, ImplItemMethod,
    Item, ItemFn, ItemStruct, ItemTrait, Pat, PatIdent, PatType, Token, TraitItem, TraitItemMethod,
    TypeBareFn, TypePath, TypeReference,
};

use super::*;
use crate::types::*;

#[derive(Debug)]
pub struct C;

impl Lang for C {
    type Error = CError;

    fn expose_fn(function: &mut ItemFn, _mod_path: &Vec<Ident>) -> Result<Ident, Self::Error> {
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
        for ignore_attr in &["constructor", "getter", "setter"] {
            if let Some(pos) = function
                .attrs
                .iter()
                .position(|a| a.path.is_ident(ignore_attr))
            {
                function.attrs.remove(pos);
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
        let attrs = &function.attrs;

        *function = parse_quote! {
            #[no_mangle]
            #[allow(non_snake_case)]
            #(#attrs)*
            pub extern "C" fn #ident(#args) #ret {
                use crate::mapping::{MapFrom, MapTo};
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
        _mod_path: &Vec<Ident>,
        _sub_items: Vec<ModuleItem>,
    ) -> Result<Ident, Self::Error> {
        module.vis = parse_quote!(pub);

        Ok(module.ident.clone())
    }

    fn expose_struct(
        structure: &mut ItemStruct,
        opts: Punctuated<ExposeStructOpts, Token![,]>,
        mod_path: &Vec<Ident>,
        extra: &mut Vec<Item>,
    ) -> Result<Ident, Self::Error> {
        let ident = structure.ident.clone();
        let is_opaque = opts
            .iter()
            .find(|o| **o == ExposeStructOpts::Opaque)
            .is_some();

        if !is_opaque {
            structure.attrs.push(parse_quote!(#[repr(C)]));
        }
        structure.vis = parse_quote!(pub);

        let impl_block =
            Self::generate_getters_setters(structure, is_opaque, mod_path)?;
        extra.push(impl_block.into());

        Ok(ident)
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
                    if let Type::Path(TypePath { path, .. }) = implementation.self_ty.as_ref() {
                        // Add the struct name as prefix
                        as_fn.sig.ident = format_ident!(
                            "{}_{}",
                            path.segments
                                .iter()
                                .map(|s| s.ident.to_string().to_snake_case())
                                .collect::<Vec<_>>()
                                .join("_"),
                            as_fn.sig.ident
                        );
                    }
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

    fn expose_trait(
        tr: &mut ItemTrait,
        _mod_path: &Vec<Ident>,
        extra: &mut Vec<Item>,
    ) -> Result<Ident, Self::Error> {
        let ident = tr.ident.clone();

        let mut callbacks = vec![];
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
                    })
                    .unwrap_or(ident.clone());

                let mut inputs = sig.inputs.iter().cloned().collect::<Vec<_>>();
                inputs[0] = parse_quote!(this: *mut libc::c_void);

                let output = &sig.output;
                let ty: Type = parse_quote!(fn(#(#inputs),*) #output);
                let converted = Self::convert_input(ty)?.expand(ident);

                if let Type::BareFn(bare_fn) = converted.types[0].as_ref() {
                    callbacks.push((sig, bare_fn.clone(), converted.conv, original_ident));
                }
            }
        }

        // Define a structure containing all the callback for the methods
        let struct_methods = callbacks.iter().map(|(sig, _, _, _)| {
            let ident = &sig.ident;
            let output = &sig.output;
            let input_types = sig.inputs.iter().map(|arg| match arg {
                FnArg::Receiver(_) => Box::new(parse_quote!(*mut libc::c_void)),
                FnArg::Typed(PatType { ty, .. }) => ty.clone(),
            });

            quote!(#ident: Box<dyn Fn(#(#input_types),*) #output>)
        });
        let trait_struct_ident = format_ident!("{}Struct", ident);
        let trait_struct: ItemStruct = parse_quote! {
            pub struct #trait_struct_ident {
                this: *mut libc::c_void,
                destroy: Box<dyn Fn(*mut libc::c_void)>,

                #(#struct_methods),*
            }
        };
        extra.push(trait_struct.into());

        // Define a constructor for our struct
        let constructor_ident =
            format_ident!("{}_new", trait_struct_ident.to_string().to_snake_case());
        let destructor_ident =
            format_ident!("{}_destroy", trait_struct_ident.to_string().to_snake_case());
        let (mut constructor_args, fields): (Vec<_>, Vec<_>) = callbacks
            .iter()
            .map(|(sig, bare_fn, conv, _)| {
                let ident = &sig.ident;

                (
                    quote!(#ident: #bare_fn),
                    quote! {
                        #ident: Box::new(#conv)
                    },
                )
            })
            .unzip();
        constructor_args.push(quote!(ptr_out: *mut *mut Self));
        let constructor: ItemImpl = parse_quote! {
            impl #trait_struct_ident {
                #[no_mangle]
                pub extern "C" fn #constructor_ident(this: *mut libc::c_void, destroy: unsafe extern "C" fn(*mut libc::c_void), #(#constructor_args),*) {
                    use crate::mapping::{MapFrom, MapTo};
                    use crate::langs::*;

                    let s = #trait_struct_ident {
                        this,
                        destroy: Box::new(move |this: *mut libc::c_void| {
                            if this != std::ptr::null_mut() {
                                unsafe { destroy(this) }
                            }
                        }),
                        #(#fields),*
                    };

                    unsafe {
                        *ptr_out = Box::into_raw(Box::new(s));
                    }
                }

                #[no_mangle]
                pub unsafe extern "C" fn #destructor_ident(s: *mut Self) {
                    Box::from_raw(s);
                }
            }
        };
        extra.push(constructor.into());

        // Impl the trait on the trait structure
        let impl_methods = callbacks.iter().map(|(sig, _, _, _)| {
            let method_ident = &sig.ident;
            let call_args = sig.inputs.iter().map(|arg| match arg {
                FnArg::Receiver(_) => quote!(self.this),
                FnArg::Typed(PatType { pat, .. }) => pat.to_token_stream(),
            });

            quote! {
                #sig {
                    (self.#method_ident)(#(#call_args),*)
                }
            }
        });
        let impl_on_trait_struct: ItemImpl = parse_quote! {
            impl #ident for #trait_struct_ident {
                #(#impl_methods)*
            }
        };
        extra.push(impl_on_trait_struct.into());

        // Define a custom destructor that calls `destroy`
        let destructor: ItemImpl = parse_quote! {
            impl std::ops::Drop for #trait_struct_ident {
                fn drop(&mut self) {
                    (self.destroy)(self.this)
                }
            }
        };
        extra.push(destructor.into());

        // Impl `IntoTraitStruct<Target = TraitStruct>` for the supertrait
        let supertrait = &tr.supertraits[0];
        let (wrap_fns, struct_members): (Vec<_>, Vec<_>) = callbacks
            .iter()
            .map(|(sig, _, _, original_ident)| {
                let ident = &sig.ident;
                let output = &sig.output;
                let inputs = sig.inputs.iter().map(|arg| match arg {
                    FnArg::Receiver(_) => parse_quote!(this: *mut libc::c_void),
                    ty @ FnArg::Typed(_) => ty.clone(),
                });
                let arg_names = sig.inputs.iter().filter_map(|arg| match arg {
                    FnArg::Receiver(_) => None,
                    FnArg::Typed(PatType { pat, .. }) => Some(pat.clone()),
                });

                (
                    quote! {
                        fn #ident<T: #supertrait>(#(#inputs),*) #output {
                            let this = take_ptr::<T>(this);

                            let result = this.#original_ident(#(#arg_names),*);

                            std::mem::forget(this);
                            return result;
                        }
                    },
                    quote! { #ident: Box::new(#ident::<Self>) },
                )
            })
            .unzip();

        let into_trait_struct: ItemImpl = parse_quote! {
            impl<T: 'static + #supertrait + Sized + Send> crate::langs::IntoTraitStruct for T {
                type Target = #trait_struct_ident;

                fn into_trait_struct(self) -> Self::Target {
                    use crate::langs::take_ptr;

                    let this = Box::into_raw(Box::new(self)) as *mut libc::c_void;

                    fn destroy<T>(this: *mut libc::c_void) {
                        let _this = take_ptr::<T>(this);
                    }

                    #(#wrap_fns)*

                    #trait_struct_ident {
                        this,
                        destroy: Box::new(destroy::<Self>),

                        #(#struct_members),*
                    }
                }
            }
        };
        extra.push(into_trait_struct.into());

        Ok(ident)
    }

    fn convert_getter_setter_ty(ty: Type) -> Result<(Type, Type), Self::Error> {
        for t in types_arr!(i8, u8, i16, u16, i32, u32, i64, u64) {
            if &ty == t {
                return Ok((parse_quote!(#ty), parse_quote!(#ty)));
            }
        }

        Ok((parse_quote!(*mut #ty), parse_quote!(&#ty)))
    }

    fn convert_input(ty: Type) -> Result<Input, Self::Error> {
        match ty {
            Type::Reference(TypeReference { ref elem, .. })
                if elem.as_ref() == &parse_quote!(Inner) =>
            {
                let elem = elem.clone();

                return Ok(Input::new_custom(
                    parse_quote!(*mut #elem),
                    vec![ty.clone()],
                    move |_, ident| {
                        let ts = quote! {
                            {
                                let #ident: #elem = #ident.clone();
                                MapFrom::map_from(#ident)
                            }
                        };
                        ts.into()
                    },
                ));
            }
            _ => {}
        }

        if match_fixed_type(&ty, parse_quote!(String)) {
            Ok(Input::new_map_from(
                ty,
                vec![parse_quote!(*const libc::c_char)],
            ))
        } else if ty == parse_quote!([u8; 32]) {
            // TODO: make more generic
            Ok(Input::new_map_from(ty, vec![parse_quote!(*const u8)]))
        } else if let Some(inner) = match_generic_type(&ty, parse_quote!(Vec)) {
            let inner = inner
                .into_iter()
                .collect::<Punctuated<_, Comma>>()
                .as_tuple();
            let inner = Self::convert_input(inner)?;
            let sources = inner
                .get_sources()
                .into_iter()
                .map(|t| *t.clone())
                .as_tuple();

            Ok(Input::new_map_from(
                ty,
                vec![parse_quote!(crate::langs::Arr<#sources>)],
            ))
        } else if let Some(inner) = match_generic_type(&ty, parse_quote!(Destroy)) {
            let inner = inner
                .into_iter()
                .collect::<Punctuated<_, Comma>>()
                .as_tuple();
            let inner = Self::convert_input(inner)?;
            let sources = inner
                .get_sources()
                .into_iter()
                .map(|t| *t.clone())
                .as_tuple();

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
                        move |#old_inputs| {
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
        // Return our opaque types by reference
        for t in &our_opaque_types!() {
            if t == &output {
                return Ok(Output::ByReference(Box::new(parse_quote!(*mut #t))));
            }
        }

        if output == parse_quote!(Self) {
            Ok(Output::ByReference(Box::new(parse_quote!(*mut Self))))
        } else if output == parse_quote!(String) {
            Ok(Output::new_map_to_single(
                output,
                parse_quote!(*mut libc::c_char),
            ))
        } else if output == parse_quote!(&[u8]) {
            Ok(Output::new_map_to_single(output, parse_quote!(*const u8)))
        } else if output == parse_quote!(BitcoinError) {
            Ok(Output::new_map_to_single(output, parse_quote!(i32)))
        } else if let Some(inner) = match_generic_type(&output, parse_quote!(Vec)) {
            let inner = inner
                .into_iter()
                .collect::<Punctuated<_, Comma>>()
                .as_tuple();
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
        } else if let Some(inner) = match_generic_type(&output, parse_quote!(Option)) {
            let inner = inner
                .into_iter()
                .collect::<Punctuated<_, Comma>>()
                .as_tuple();
            let inner_output = Self::convert_output(inner.clone())?;
            let targets = inner_output
                .get_targets()
                .into_iter()
                .map(|t| *t)
                .as_tuple();

            Ok(Output::new_option(inner, targets))
        } else if let Some(inner) = match_generic_type(&output, parse_quote!(Result)) {
            let inner: [_; 2] = inner
                .try_into()
                .map_err(|_| CError::InvalidResult(output.span()))?;

            let ok_type = Self::convert_output(inner[0].clone())?;
            let ok_targets = ok_type.get_targets().into_iter().map(|t| *t).collect();
            let err_type = Self::convert_output(inner[1].clone())?;
            let err_target = err_type
                .get_targets()
                .into_iter()
                .map(|t| *t)
                .collect::<Punctuated<_, Comma>>()
                .as_tuple(); // the error must always be a single type

            Ok(Output::new_result(
                inner[0].clone(),
                inner[1].clone(),
                ok_targets,
                err_target,
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
    InvalidResult(Span),
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
