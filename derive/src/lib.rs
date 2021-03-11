#![allow(unused_imports)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, parse_quote, Attribute, Fields, Ident, ImplItem, ImplItemMethod, Item,
    ItemFn, ItemImpl, ItemMod, ItemStruct, Token, Type, TypePath,
};

mod langs;
mod types;

use langs::Lang;
use types::*;

#[cfg(feature = "c")]
type CurrentLang = langs::c::C;
#[cfg(feature = "python")]
type CurrentLang = langs::python::Python;

fn check_struct(s: &ItemStruct) {
    if !matches!(s.fields, Fields::Named(_)) {
        panic!("Only named structs are supported");
    }
}

fn analyze_module(module: &mut ItemMod, mut path: Vec<Ident>) {
    path.push(module.ident.clone());

    let mut sub_items = vec![];

    for item in &mut module.content.as_mut().expect("Empty module").1 {
        match item {
            Item::Mod(inner_module) => {
                if let Some(pos) = inner_module
                    .attrs
                    .iter()
                    .position(|a| a.path.is_ident("expose_mod"))
                {
                    inner_module.attrs.remove(pos);
                    analyze_module(inner_module, path.clone());

                    sub_items.push(ModuleItem::Module(inner_module.ident.clone()));
                }
            }
            Item::Fn(function) => {
                if let Some(pos) = function
                    .attrs
                    .iter()
                    .position(|a| a.path.is_ident("expose_fn"))
                {
                    function.attrs.remove(pos);
                    sub_items.push(ModuleItem::Function(
                        CurrentLang::expose_fn(function, &path).unwrap(),
                    ));
                }
            }
            Item::Struct(structure) => {
                if let Some(pos) = structure
                    .attrs
                    .iter()
                    .position(|a| a.path.is_ident("expose_struct"))
                {
                    let parser = Punctuated::<ExposeStructOpts, Token![,]>::parse_terminated;
                    let opts = structure.attrs[pos].parse_args_with(parser).unwrap();

                    structure.attrs.remove(pos);
                    check_struct(structure);

                    sub_items.push(ModuleItem::Structure(
                        CurrentLang::expose_struct(structure, opts, &path).unwrap(),
                    ));
                }
            }
            Item::Impl(implementation) => {
                if let Some(pos) = implementation
                    .attrs
                    .iter()
                    .position(|a| a.path.is_ident("expose_impl"))
                {
                    implementation.attrs.remove(pos);
                    CurrentLang::expose_impl(implementation, &path).unwrap();
                }
            }
            _ => {}
        }
    }

    CurrentLang::expose_mod(module, &path, sub_items).unwrap();
}

#[proc_macro_attribute]
pub fn expose_mod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemMod);
    analyze_module(&mut input, vec![]);

    (quote! {
        #input
    })
    .into()
}

#[proc_macro_attribute]
pub fn expose_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    CurrentLang::expose_fn(&mut input, &vec![]).unwrap();

    (quote! {
        #input
    })
    .into()
}

#[proc_macro_attribute]
pub fn expose_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    check_struct(&input);

    let attr: TokenStream2 = attr.into();
    let attr: Attribute = parse_quote! { #attr };

    let parser = Punctuated::<ExposeStructOpts, Token![,]>::parse_terminated;
    let opts = attr.parse_args_with(parser).unwrap();

    CurrentLang::expose_struct(&mut input, opts, &vec![]).unwrap();

    (quote! {
        #input
    })
    .into()
}

#[proc_macro_attribute]
pub fn expose_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);
    CurrentLang::expose_impl(&mut input, &vec![]).unwrap();

    (quote! {
        #input
    })
    .into()
}
