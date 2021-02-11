use std::convert::TryFrom;
use std::fmt;

use proc_macro::TokenStream;
use syn::{ItemFn, Ident, Path, FnArg, parse_quote, Pat, PatType, PatIdent, Type, ReturnType, ItemMod};

#[cfg(feature = "c")]
pub mod c;
#[cfg(feature = "python")]
pub mod python;

#[derive(Debug)]
pub enum LangError {
    UnknownFnArg(Box<FnArg>),
    UnknownOutputType(Box<Type>),
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LangError {}

// TODO add useful info inside, like the arg name
pub enum DataTypeIn {
    SelfRef,
    SelfMutRef,

    String,
}

impl TryFrom<&FnArg> for DataTypeIn {
    type Error = LangError;

    fn try_from(arg: &FnArg) -> Result<Self, Self::Error> {
        Ok(match arg {
            FnArg::Receiver(receiver) if receiver.mutability.is_none() => DataTypeIn::SelfRef,
            FnArg::Receiver(receiver) if receiver.mutability.is_some() => DataTypeIn::SelfMutRef,
            FnArg::Typed(typed) if typed.ty == parse_quote!(String) => DataTypeIn::String,
            x => return Err(LangError::UnknownFnArg(Box::new(x.clone())))
        })
    }
}

pub enum DataTypeOut {
    String,
}

impl TryFrom<&Type> for DataTypeOut {
    type Error = LangError;

    fn try_from(output: &Type) -> Result<Self, Self::Error> {
        if output == &parse_quote!(String) {
            return Ok(DataTypeOut::String);
        }

        Err(LangError::UnknownOutputType(Box::new(output.clone())))
    }
}

pub trait Lang {
    type Error: From<LangError> + std::error::Error;

    fn expose_fn(function: &mut ItemFn, mod_path: &Vec<Ident>) -> Result<(), Self::Error>;

    fn expose_mod(module: &mut ItemMod, mod_path: &Vec<Ident>) -> Result<(), Self::Error>;

    fn convert_arg(arg: FnArg, dt: DataTypeIn, arg_name: Option<Ident>) -> Result<(Vec<FnArg>, TokenStream), Self::Error>;

    fn convert_output(output: ReturnType) -> Result<(Type, TokenStream), Self::Error>;

    // provided methods
    fn convert_fn_args<I: IntoIterator<Item = FnArg>>(args: I) -> Result<(Vec<FnArg>, TokenStream), Self::Error> {
        Ok(args.into_iter()
            .map(|arg| {
                let dt = DataTypeIn::try_from(&arg)?;

                let arg_name = match &arg {
                    FnArg::Typed(PatType { pat, .. }) => {
                        if let Pat::Ident(PatIdent { ident, .. }) = *pat.clone() {
                            Some(ident)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                Self::convert_arg(arg, dt, arg_name)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .fold((vec![], TokenStream::default()), |(mut fold_args, mut fold_ts), (args, ts)| {
                fold_args.extend(args.into_iter());
                fold_ts.extend(ts);

                (fold_args, fold_ts)
            }))
    }
}
