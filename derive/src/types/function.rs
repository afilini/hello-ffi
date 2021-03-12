use std::ops::Deref;

use proc_macro2::TokenStream as TokenStream2;

use quote::{format_ident, quote, ToTokens};

use syn::punctuated::Punctuated;
use syn::token::{Comma, RArrow};
use syn::{
    parse_quote, BareFnArg, FnArg, GenericArgument, Ident, Pat, PatIdent, PatType, Path,
    PathArguments, PathSegment, ReturnType, Type, TypePath,
};

use crate::langs::LangError;

/// Transform a list of types into a tuple
///
/// The result is:
/// - `()` if the list is empty
/// - `T` if the list only contains one element
/// - `(T1, T2, ...)` otherwise
pub trait AsTuple {
    fn as_tuple(self) -> Type;
}

impl<T: IntoIterator<Item = Type>> AsTuple for T {
    fn as_tuple(self) -> Type {
        let punctuated = self.into_iter().collect::<Punctuated<_, Comma>>();

        match punctuated.len() {
            0 => parse_quote! { () },
            1 => parse_quote! { #punctuated },
            _ => parse_quote! { ( #punctuated ) },
        }
    }
}

pub fn match_fixed_type(ty: &Type, type_path: Path) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) if path == &type_path => true,
        _ => false,
    }
}

pub fn match_generic_type(ty: &Type, type_path: Path) -> Option<Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        let mut path = path.clone();
        path.segments
            .iter_mut()
            .last()
            // Remove the generic from the last path segment and return it
            .and_then(
                |PathSegment {
                     ref mut arguments, ..
                 }| {
                    let original_arguments = arguments.clone();
                    *arguments = PathArguments::None;

                    Some(original_arguments)
                },
            )
            // Compare the path without generic to the required one
            .filter(|_| path == type_path)
            // Return the content of the angle brackets, if present
            .and_then(|arguments| match arguments {
                PathArguments::AngleBracketed(inner) => Some(inner.args),
                _ => None,
            })
            // Map the content to a list of types, only if all the items are types
            .and_then(|content| {
                content
                    .into_iter()
                    .try_fold(Punctuated::<Type, Comma>::default(), |mut acc, f| match f {
                        GenericArgument::Type(ty) => {
                            acc.push(ty);
                            Some(acc)
                        }
                        _ => None,
                    })
            })
            .map(AsTuple::as_tuple)
    } else {
        None
    }
}

/// Transform a `ReturnType` into a `Type`, regardless of its variant
pub trait ReturnTypeTyped {
    fn as_type(&self) -> Type;
}

impl ReturnTypeTyped for ReturnType {
    fn as_type(&self) -> Type {
        match self {
            ReturnType::Default => parse_quote! { () },
            ReturnType::Type(_, ty) => (**ty).clone(),
        }
    }
}

// #[derive(Debug)]
pub enum Input {
    /// Leave the type unchanged, doesn't perform any conversion
    Unchanged(Box<Type>),
    /// Map from one or more different types
    MapFrom {
        target: Box<Type>,
        sources: Vec<Box<Type>>,
    },
    /// Custom mapping
    Custom {
        target: Box<Type>,
        sources: Vec<Box<Type>>,

        expand: Box<dyn Fn(&Type, &Ident) -> ExpandedInputConversion>,
    },
}

#[derive(Debug)]
pub struct ExpandedInputConversion(TokenStream2);

impl ExpandedInputConversion {
    pub fn pass_through(ident: &Ident) -> Self {
        let ts = quote! {
            #ident
        };
        ts.into()
    }

    pub fn map_from(ty: &Type, ident: &Ident) -> Self {
        let ts = quote! {
            <#ty>::map_from(#ident)
        };
        ts.into()
    }
}

#[derive(Debug)]
pub struct ExpandedInput {
    pub types: Vec<Box<Type>>,
    pub conv: ExpandedInputConversion,
}

impl Input {
    pub fn new_unchanged(ty: Type) -> Self {
        Input::Unchanged(Box::new(ty))
    }

    pub fn new_map_from(target: Type, sources: Vec<Type>) -> Self {
        Input::MapFrom {
            target: Box::new(target),
            sources: sources.into_iter().map(Box::new).collect(),
        }
    }

    pub fn new_custom<F: 'static + Fn(&Type, &Ident) -> ExpandedInputConversion>(
        target: Type,
        sources: Vec<Type>,
        expand: F,
    ) -> Self {
        Input::Custom {
            target: Box::new(target),
            sources: sources.into_iter().map(Box::new).collect(),
            expand: Box::new(expand),
        }
    }

    pub fn get_sources(&self) -> Vec<&Box<Type>> {
        match self {
            Input::Unchanged(ty) => vec![ty],
            Input::MapFrom { sources, .. } | Input::Custom { sources, .. } => {
                sources.iter().collect()
            }
        }
    }

    pub fn expand(self, ident: &Ident) -> ExpandedInput {
        match self {
            Input::Unchanged(ty) => ExpandedInput {
                conv: ExpandedInputConversion::pass_through(ident),
                types: vec![ty],
            },
            Input::MapFrom { target, sources } => ExpandedInput {
                conv: ExpandedInputConversion::map_from(&target, ident),
                types: sources,
            },
            Input::Custom {
                target,
                sources,
                expand,
            } => ExpandedInput {
                conv: expand(&target, ident),
                types: sources,
            },
        }
    }
}

#[derive(Debug)]
pub struct Argument(pub FnArg);

#[derive(Debug)]
pub struct ExpandedArgumentConversion(TokenStream2);

impl ExpandedArgumentConversion {
    fn empty() -> Self {
        ExpandedArgumentConversion(TokenStream2::default())
    }
}

#[derive(Debug)]
pub struct ExpandedArgument {
    pub args: Punctuated<FnArg, Comma>,
    pub conv: ExpandedArgumentConversion,
}

impl Argument {
    pub fn expand<F, E>(self, convert_input: F) -> Result<ExpandedArgument, E>
    where
        E: From<LangError>,
        F: Fn(Type) -> Result<Input, E>,
    {
        let (ident, ty) = match self.0 {
            r @ FnArg::Receiver(_) => {
                return Ok(ExpandedArgument {
                    args: vec![r].into_iter().collect(),
                    conv: ExpandedArgumentConversion::empty(),
                });
            }
            FnArg::Typed(PatType { pat, ty, .. }) => match *pat {
                Pat::Ident(PatIdent { ident, .. }) => (ident, ty),
                _ => return Err(LangError::ComplexPatternFnArg.into()),
            },
        };

        let temp_ident = format_ident!("_temp_{}", ident);
        let expanded = convert_input(*ty)?.expand(&temp_ident);
        let input_conv = &expanded.conv;

        let idents = expanded
            .types
            .iter()
            .enumerate()
            .map(|(i, _)| Ident::new(&format!("__{}_{}", ident, i), ident.span()))
            .collect::<Punctuated<_, Comma>>();

        let conv = quote! {
            let #temp_ident = (#idents);
            let #ident = #input_conv;
        };

        let args = expanded
            .types
            .into_iter()
            .zip(idents.into_iter())
            .map::<FnArg, _>(|(ty, ident)| parse_quote!(#ident: #ty))
            .collect();

        Ok(ExpandedArgument {
            args,
            conv: ExpandedArgumentConversion::from(conv),
        })
    }
}

#[derive(Debug)]
pub enum Output {
    /// Leave the type unchanged, doesn't perform any conversion
    Unchanged(Box<Type>),
    /// Map to another one or more types. This will call `MapTo::map_to()`
    MapTo {
        original: Box<Type>,
        targets: Vec<(Box<Type>, String)>,
    },
    /// Move the value to the heap and return a pointer
    ByReference(Box<Type>),
    // /// Return as "result", which has different meanings based on the language
    // Result {
    //     ok: Box<Type>,
    //     err: Box<Type>,
    // },
}

#[derive(Debug)]
pub struct ExpandedOutputConversion(TokenStream2);

impl ExpandedOutputConversion {
    pub fn pass_through(ident: &Ident) -> Self {
        let ts = quote! {
            let #ident = #ident;
        };
        ts.into()
    }

    pub fn map_to(ident: &Ident, original: &Type) -> Self {
        let ts = quote! {
            let #ident: #original = #ident;
            let #ident = #ident.map_to();
        };
        ts.into()
    }

    pub fn by_reference(ident: &Ident) -> Self {
        let ts = quote! {
            let #ident = Box::into_raw(Box::new(#ident));
        };
        ts.into()
    }
}

#[derive(Debug)]
pub struct ExpandedOutput {
    pub ty: Vec<Box<Type>>,
    pub suffix: Vec<String>,
    pub conv: ExpandedOutputConversion,
}

impl Output {
    pub fn new_unchanged(ty: Type) -> Self {
        Output::Unchanged(Box::new(ty))
    }

    pub fn new_map_to_suffix(original: Type, targets: Vec<(Type, String)>) -> Self {
        Output::MapTo {
            original: Box::new(original),
            targets: targets.into_iter().map(|(t, s)| (Box::new(t), s)).collect(),
        }
    }

    pub fn new_map_to(original: Type, targets: Vec<Type>) -> Self {
        Self::new_map_to_suffix(
            original,
            targets
                .into_iter()
                .enumerate()
                .map(|(i, t)| (t, i.to_string()))
                .collect(),
        )
    }

    pub fn new_map_to_single(original: Type, target: Type) -> Self {
        Self::new_map_to_suffix(original, vec![(target, String::new())])
    }

    pub fn get_targets(&self) -> Vec<Box<Type>> {
        match self {
            Output::Unchanged(ty) | Output::ByReference(ty) => vec![ty.clone()],
            Output::MapTo { targets, .. } => targets.iter().map(|(t, _)| t.clone()).collect(),
        }
    }

    pub fn expand(&self, ident: &Ident) -> ExpandedOutput {
        match self {
            Output::Unchanged(ty) => ExpandedOutput {
                ty: vec![ty.clone()],
                suffix: vec![String::new()],
                conv: ExpandedOutputConversion::pass_through(ident),
            },
            Output::MapTo { original, targets } => {
                let (targets, suffix) = targets.iter().cloned().unzip();

                ExpandedOutput {
                    ty: targets,
                    suffix,
                    conv: ExpandedOutputConversion::map_to(ident, &original),
                }
            }
            Output::ByReference(ty) => ExpandedOutput {
                ty: vec![parse_quote! { *mut #ty }],
                suffix: vec![String::new()],
                conv: ExpandedOutputConversion::by_reference(ident),
            },
        }
    }
}

#[derive(Debug)]
pub struct Return(pub ReturnType);

#[derive(Debug)]
pub struct ExpandedReturnConversion(TokenStream2);

impl ExpandedReturnConversion {
    fn ret(ident: &Ident, conv: ExpandedOutputConversion) -> Self {
        let ts = quote! {
            #conv
            #ident
        };

        ts.into()
    }
}

#[derive(Debug)]
pub struct ExpandedReturn {
    pub ret: ReturnType,
    pub extra_args: Vec<FnArg>,
    pub conv: ExpandedReturnConversion,
}

impl Return {
    pub fn expand<F, E>(
        self,
        ident: &Ident,
        arg_name: &Ident,
        convert_output: F,
    ) -> Result<ExpandedReturn, E>
    where
        E: From<LangError>,
        F: Fn(Type) -> Result<Output, E>,
    {
        let ty = self.0.as_type();
        let converted = convert_output(ty)?;

        let ExpandedOutput { ty, conv, .. } = converted.expand(&ident);
        let ty = ty.into_iter().map(|t| *t).as_tuple();

        match converted {
            Output::ByReference(_) => {
                let extra_arg = parse_quote!(#arg_name: #ty);

                Ok(ExpandedReturn {
                    ret: ReturnType::Default,
                    extra_args: vec![extra_arg],
                    conv: ExpandedReturnConversion::from(quote! {
                        #conv
                        unsafe { *#arg_name = #ident; }
                    }),
                })
            }
            _ => Ok(ExpandedReturn {
                ret: ReturnType::Type(Default::default(), Box::new(ty)),
                extra_args: vec![],
                conv: ExpandedReturnConversion::ret(ident, conv),
            }),
        }
    }
}

#[derive(Debug)]
pub struct CallbackArgument(pub BareFnArg);

#[derive(Debug)]
pub struct ExpandedCallbackArgumentConversion(TokenStream2);

impl ExpandedCallbackArgumentConversion {
    fn group_sub_args(
        ident: &Ident,
        arg_names: Vec<Ident>,
        original: ExpandedOutputConversion,
    ) -> Self {
        let grouped = arg_names.into_iter().collect::<Punctuated<_, Comma>>();
        let original = original.into_inner();

        let ts = quote! {
            #original
            let (#grouped) = #ident;
        };
        ts.into()
    }
}

#[derive(Debug)]
pub struct ExpandedCallbackArgument {
    pub args: Vec<BareFnArg>,
    pub conv: ExpandedCallbackArgumentConversion,
}

impl CallbackArgument {
    pub fn expand<F, E>(
        self,
        ident: &Ident,
        convert_output: F,
    ) -> Result<ExpandedCallbackArgument, E>
    where
        E: From<LangError>,
        F: Fn(Type) -> Result<Output, E>,
    {
        let converted = convert_output(self.0.ty)?;

        let ExpandedOutput { ty, suffix, conv } = converted.expand(&ident);
        let (args, arg_names): (Vec<_>, Vec<_>) = ty
            .into_iter()
            .zip(suffix.into_iter())
            .map(|(t, s)| {
                let arg_name = match s.is_empty() {
                    true => ident.clone(),
                    false => format_ident!("{}_{}", ident, s),
                };

                (parse_quote!(#arg_name: #t), arg_name)
            })
            .unzip();

        Ok(ExpandedCallbackArgument {
            args,
            conv: ExpandedCallbackArgumentConversion::group_sub_args(ident, arg_names, conv),
        })
    }
}

#[derive(Debug)]
pub struct CallbackReturn(pub ReturnType);

#[derive(Debug)]
pub struct ExpandedCallbackReturnConversion(TokenStream2);

#[derive(Debug)]
pub struct ExpandedCallbackReturn {
    pub ret: ReturnType,
    pub conv: ExpandedCallbackReturnConversion,
}

impl CallbackReturn {
    pub fn expand<F, E>(self, ident: &Ident, convert_input: F) -> Result<ExpandedCallbackReturn, E>
    where
        E: From<LangError>,
        F: Fn(Type) -> Result<Input, E>,
    {
        let expanded = convert_input(self.0.as_type())?.expand(&ident);

        let ret = match expanded.types.is_empty() {
            true => ReturnType::Default,
            false => ReturnType::Type(
                Default::default(),
                Box::new(expanded.types.into_iter().map(|t| *t).as_tuple()),
            ),
        };

        Ok(ExpandedCallbackReturn {
            ret,
            conv: ExpandedCallbackReturnConversion::from(expanded.conv.into_inner()),
        })
    }
}

macro_rules! impl_common_traits {
    ($ty:ident) => {
        impl From<TokenStream2> for $ty {
            fn from(ts: TokenStream2) -> Self {
                $ty(ts)
            }
        }

        impl Deref for $ty {
            type Target = TokenStream2;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ToTokens for $ty {
            fn to_tokens(&self, tokens: &mut TokenStream2) {
                self.deref().to_tokens(tokens)
            }
        }

        impl $ty {
            #[allow(dead_code)]
            pub fn into_inner(self) -> TokenStream2 {
                self.0
            }
        }
    };
}

impl_common_traits!(ExpandedInputConversion);
impl_common_traits!(ExpandedArgumentConversion);
impl_common_traits!(ExpandedOutputConversion);
impl_common_traits!(ExpandedReturnConversion);
impl_common_traits!(ExpandedCallbackArgumentConversion);
impl_common_traits!(ExpandedCallbackReturnConversion);
