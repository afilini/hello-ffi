use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    parse_quote, FnArg, Ident, Item, ItemFn, ItemImpl, ItemMod, ItemStruct, ItemTrait, Lit, LitStr,
    ParenthesizedGenericArguments, Pat, PatIdent, PatType, Path, PathArguments, ReturnType, Token,
    Type,
};

use crate::types::*;

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

    // provided methods
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
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LangError {}
