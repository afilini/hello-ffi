use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::env;

use syn::{Ident, ItemStruct, ItemMod, ItemFn, Item, ImplItem, parse_macro_input, ItemImpl, Type, TypePath, ImplItemMethod, Fields};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};

mod langs;

use langs::{Lang};
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

    for item in &mut module.content.as_mut().expect("Empty module").1 {
        match item {
            Item::Mod(inner_module) => {
                if let Some(pos) = inner_module.attrs.iter().position(|a| a.path.is_ident("expose_mod")) {
                    inner_module.attrs.remove(pos);
                    analyze_module(inner_module, path.clone());
                }
            },
            Item::Fn(function) => {
                if let Some(pos) = function.attrs.iter().position(|a| a.path.is_ident("expose_fn")) {
                    function.attrs.remove(pos);
                    CurrentLang::expose_fn(function, &path).unwrap();
                }
            },
            Item::Struct(structure) => {
                if let Some(pos) = structure.attrs.iter().position(|a| a.path.is_ident("expose_struct")) {
                    structure.attrs.remove(pos);
                    check_struct(structure);
                    CurrentLang::expose_struct(structure, &path).unwrap();
                }
            },
            Item::Impl(implementation) => {
                if let Some(pos) = implementation.attrs.iter().position(|a| a.path.is_ident("expose_impl")) {
                    implementation.attrs.remove(pos);
                    do_expose_impl(implementation, &path);
                }
            },
            _ => {}
        }
    }

    CurrentLang::expose_mod(module, &path).unwrap();
}

fn do_expose_impl(implementation: &mut ItemImpl, mod_path: &Vec<Ident>) {
    for item in &mut implementation.items {
        dbg!(&item);

        match item {
            ImplItem::Method(ImplItemMethod { sig, vis, attrs, block, .. }) => {
                let mut as_fn = ItemFn{ sig: sig.clone(), vis: vis.clone(), attrs: attrs.clone(), block: Box::new(block.clone()) };
                CurrentLang::expose_fn(&mut as_fn, mod_path).unwrap();

                *sig = as_fn.sig;
                *vis = as_fn.vis;
                *attrs = as_fn.attrs;
                *block = *as_fn.block;
            },
            _ => {}
        }
    }
}

#[proc_macro_attribute]
pub fn expose_mod(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemMod);
    analyze_module(&mut input, vec![]);

    (quote! {
        #input
    }).into()
}

#[proc_macro_attribute]
pub fn expose_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);
    CurrentLang::expose_fn(&mut input, &vec![]).unwrap();

    (quote! {
        #input
    }).into()
}

#[proc_macro_attribute]
pub fn expose_struct(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    check_struct(&input);
    CurrentLang::expose_struct(&mut input, &vec![]).unwrap();

    (quote! {
        #input
    }).into()
}

#[proc_macro_attribute]
pub fn expose_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);
    do_expose_impl(&mut input, &vec![]);

    (quote! {
        #input
    }).into()
}



#[proc_macro_attribute]
pub fn expose(attr: TokenStream, item: TokenStream) -> TokenStream {
    // let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let input = parse_macro_input!(item as ItemImpl);

    let ty_name = match input.self_ty.as_ref() {
        Type::Path(TypePath { ref path, .. }) => path.segments.iter().map(|s| s.ident.to_string()).fold(String::new(), |a, b| a + &b + "_"),
        _ => unimplemented!(),
    };

    dbg!(&input);
    dbg!(&ty_name);

    let mut output = proc_macro2::TokenStream::new();

    for item in &input.items {
        match item {
            ImplItem::Method(ImplItemMethod { sig, .. }) => {
                let full_name = ty_name.clone() + &sig.ident.to_string();
                let fn_name = Ident::new(&full_name, proc_macro2::Span::call_site());

                let inputs = sig.inputs.iter().map(|input| {
                }).collect::<Vec<_>>();

                output.extend(quote!{
                    #[no_mangle]
                    pub extern "C" fn #fn_name() {

                    }
                });
            },
            _ => {}
        }
    }

    // let mut f = File::create(PathBuf::from(crate_dir).join("lib.h")).unwrap();
    // let ts: TokenStream = output.into();

    // f.write(ts.to_string().as_bytes()).unwrap();

    (quote! {
        #input
        #output
    }).into()
}
