use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    parse_quote, Field, Fields, FieldsNamed, FnArg, Ident, Item, ItemFn, ItemImpl, ItemMod,
    ItemStruct, ItemTrait, Lit, LitStr, ParenthesizedGenericArguments, Pat, PatIdent, PatType,
    Path, PathArguments, ReturnType, Token, Type, ImplItemMethod
};

use crate::types::*;

#[macro_use]
pub mod common_mapping;

#[cfg(feature = "c")]
pub mod c;
#[cfg(feature = "python")]
pub mod python;

pub trait Lang {
    type Error: From<LangError> + std::error::Error;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error>;

    fn expose_mod(
        module: &mut ItemMod,
        mod_path: &Vec<Ident>,
        sub_items: Vec<ModuleItem>,
    ) -> Result<Ident, Self::Error>;

    fn expose_struct(
        structure: &mut ItemStruct,
        opts: Punctuated<ExposeStructOpts, Token![,]>,
        mod_path: &Vec<Ident>,
        extra: &mut Vec<Item>,
    ) -> Result<Ident, Self::Error>;

    fn expose_impl(implementation: &mut ItemImpl, mod_path: &Vec<Ident>)
        -> Result<(), Self::Error>;

    fn expose_trait(
        tr: &mut ItemTrait,
        mod_path: &Vec<Ident>,
        extra: &mut Vec<Item>,
    ) -> Result<Ident, Self::Error>;

    fn convert_input(ty: Type) -> Result<Input, Self::Error>;

    fn convert_output(output: Type) -> Result<Output, Self::Error>;

    // By default links directly to the `WrappedStructField` trait, but it might bave to be
    // overridden on some languages (like C)
    //
    // When overridden this must match the various implementations of WrappedStructField
    fn convert_getter_setter_ty(ty: Type) -> Result<(Type, Type), Self::Error> {
        Ok((parse_quote!(<#ty as crate::common::WrappedStructField>::Getter), parse_quote!(<#ty as crate::common::WrappedStructField>::Setter)))
    }

    fn expose_getter(
        structure: &Ident,
        field: &mut Field,
        is_opaque: bool,
        impl_block: &mut ItemImpl,
    ) -> Result<(), Self::Error> {
        if !is_opaque {
            return Ok(());
        }

        let field_ty = &field.ty;
        let getter_ty = Self::convert_getter_setter_ty(field.ty.clone())?.0;
        let field_ident = field.ident.as_ref().expect("Missing field ident");
        let getter_name = format_ident!("get_{}", field_ident);
        let getter: ImplItemMethod = parse_quote! {
            #[getter]
            fn #getter_name(&mut self) -> #getter_ty {
                use crate::common::WrappedStructField;
                #field_ty::wrap_get(&mut self.#field_ident)
            }
        };
        impl_block.items.push(getter.into());

        Ok(())
    }

    fn expose_setter(
        structure: &Ident,
        field: &mut Field,
        is_opaque: bool,
        impl_block: &mut ItemImpl,
    ) -> Result<(), Self::Error> {
        if !is_opaque {
            return Ok(());
        }

        let field_ty = &field.ty;
        let setter_ty = Self::convert_getter_setter_ty(field.ty.clone())?.1;
        let field_ident = field.ident.as_ref().expect("Missing field ident");
        let setter_name = format_ident!("set_{}", field_ident);
        let setter: ImplItemMethod = parse_quote! {
            #[setter]
            fn #setter_name(&mut self, #field_ident: #setter_ty) {
                use crate::common::WrappedStructField;
                self.#field_ident = #field_ty::wrap_set(#field_ident);
            }
        };
        impl_block.items.push(setter.into());

        Ok(())
    }

    fn convert_fn_args<I: IntoIterator<Item = FnArg>>(
        args: I,
    ) -> Result<(Punctuated<FnArg, Comma>, TokenStream2), Self::Error> {
        Ok(args
            .into_iter()
            .map(|i| Argument(i).expand(Self::convert_input))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .fold(
                (
                    Punctuated::<FnArg, Comma>::default(),
                    TokenStream2::default(),
                ),
                |(mut fold_args, mut fold_conv), ExpandedArgument { args, conv }| {
                    fold_args.extend(args);
                    fold_conv.extend(conv.into_inner());

                    (fold_args, fold_conv)
                },
            ))
    }

    fn generate_getters_setters(
        structure: &mut ItemStruct,
        is_opaque: bool,
        mod_path: &Vec<Ident>,
    ) -> Result<ItemImpl, Self::Error> {
        let structure_ident = structure.ident.clone();

        let mut impl_block: ItemImpl = parse_quote! {
            impl #structure_ident {
            }
        };

        if let Fields::Named(FieldsNamed { named, .. }) = &mut structure.fields {
            for mut field in named {
                if let Some(pos) = field
                    .attrs
                    .iter()
                    .position(|a| a.path.is_ident("expose_struct"))
                {
                    let parser = Punctuated::<ExposeStructOpts, Token![,]>::parse_terminated;
                    let parsed_attrs = field.attrs[pos]
                        .parse_args_with(parser)
                        .map_err(LangError::ExposeTraitAttrError)?;
                    let parsed_attrs = parsed_attrs.into_iter().collect::<HashSet<_>>();
                    field.attrs.remove(pos);

                    let mut wrap_type = false;
                    if parsed_attrs.contains(&ExposeStructOpts::Get) {
                        wrap_type = true;
                        Self::expose_getter(
                            &structure.ident,
                            &mut field,
                            is_opaque,
                            &mut impl_block,
                        )?;
                    }
                    if parsed_attrs.contains(&ExposeStructOpts::Set) {
                        wrap_type = true;
                        Self::expose_setter(
                            &structure.ident,
                            &mut field,
                            is_opaque,
                            &mut impl_block,
                        )?;
                    }

                    if wrap_type {
                        let field_ty = &field.ty;
                        field.ty = parse_quote!(<#field_ty as crate::common::WrappedStructField>::Store);

                        field.vis = parse_quote!( pub(crate) );

                        let field_ident = field.ident.as_ref().expect("Missing field ident");
                    }
                }
            }
        }

        Self::expose_impl(&mut impl_block, mod_path)?;

        Ok(impl_block)
    }
}

pub trait ToSnakeCase {
    fn to_snake_case(&self) -> String;
}

impl<T: AsRef<str> + ?Sized> ToSnakeCase for T {
    fn to_snake_case(&self) -> String {
        let mut s = String::with_capacity(self.as_ref().len());

        for (i, c) in self.as_ref().char_indices() {
            if c.is_uppercase() {
                if i > 0 {
                    s.push('_');
                }
                s.extend(c.to_lowercase());
            } else {
                s.push(c);
            }
        }

        s
    }
}

#[derive(Debug)]
pub enum LangError {
    /// Complex pattern in function argument.
    ///
    /// Only basic patterns like `foo: u32` are supported
    ComplexPatternFnArg,

    /// Trying to return multiple different types by reference
    MultipleTypesByReference,

    /// Invalid attributes given to `#[expose_trait]`
    ExposeTraitAttrError(syn::Error),

    /// Invalid attribute options in `#[expose_struct]`
    ExposeStructAttrError(syn::Error),
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LangError {}
