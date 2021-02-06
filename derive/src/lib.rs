use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::env;

use syn::*;
use proc_macro::*;
use quote::quote;

#[proc_macro_derive(Expose, attributes(expose))]
pub fn derive_expose(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);

    dbg!(&input);

    let name = &input.ident;
    let name_string = name.to_string();
    let output = quote! {
        impl #name {
            pub fn struct_name() -> &'static str {
                #name_string
            }
        }
    };

    // Return output TokenStream so your custom derive behavior will be attached.
    TokenStream::from(output)
}

#[proc_macro_attribute]
pub fn expose(attr: TokenStream, item: TokenStream) -> TokenStream {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

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
                let fn_name = syn::Ident::new(&full_name, proc_macro2::Span::call_site());

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
