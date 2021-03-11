use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use syn::{ItemFn, Ident, Path, FnArg, parse_quote, Pat, PatType, PatIdent, Type, ReturnType, ItemMod, ItemStruct, ItemImpl, Token, Lit, LitStr, PathArguments, ParenthesizedGenericArguments};
use syn::parse::{Parse, ParseStream};
use syn::token::Comma;
use syn::punctuated::Punctuated;

use crate::types::*;

#[cfg(feature = "c")]
pub mod c;
#[cfg(feature = "python")]
pub mod python;

pub trait Lang {
    type Error: From<LangError> + std::error::Error;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error>;

    fn expose_mod(module: &mut ItemMod, mod_path: &Vec<Ident>, sub_items: Vec<ModuleItem>) -> Result<Ident, Self::Error>;

    fn expose_struct(structure: &mut ItemStruct, opts: Punctuated<ExposeStructOpts, Token![,]>, mod_path: &Vec<Ident>) -> Result<Ident, Self::Error>;

    fn expose_impl(implementation: &mut ItemImpl, mod_path: &Vec<Ident>) -> Result<(), Self::Error>;

    fn convert_input(ty: Type) -> Result<Input, Self::Error>;

    fn convert_output(output: Type) -> Result<Output, Self::Error>;

    // provided methods
    // fn convert_fn_args<I: IntoIterator<Item = FnArg>>(args: I) -> Result<(Punctuated<FnArg, Comma>, TokenStream), Self::Error> {
    //     let (args, ts) = args.into_iter()
    //         .map(|arg| {
    //             let dt = DataTypeIn::try_from(&arg)?;

    //             let arg_name = match &arg {
    //                 FnArg::Typed(PatType { pat, .. }) => {
    //                     if let Pat::Ident(PatIdent { ident, .. }) = *pat.clone() {
    //                         Some(ident)
    //                     } else {
    //                         None
    //                     }
    //                 }
    //                 _ => None,
    //             };

    //             Self::convert_arg(arg, dt, arg_name)
    //         })
    //         .collect::<Result<Vec<_>, _>>()?
    //         .into_iter()
    //         .fold((vec![], TokenStream::default()), |(mut fold_args, mut fold_ts), (args, ts)| {
    //             fold_args.extend(args.into_iter());
    //             fold_ts.extend(ts);

    //             (fold_args, fold_ts)
    //         });

    //     Ok((args.into_iter().collect(), ts))
    // }
}

#[derive(Debug)]
pub enum LangError {
    /// Complex pattern in function argument.
    ///
    /// Only basic patterns like `foo: u32` are supported
    ComplexPatternFnArg,
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LangError {}

