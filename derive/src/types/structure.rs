use syn::parse::{Parse, ParseStream};
use syn::{Lit, LitStr, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExposeStructOpts {
    Opaque,
    Get,
    Set,

    #[cfg(feature = "python")]
    Subclass,
}

impl Parse for ExposeStructOpts {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Lit)
            && input.peek(LitStr)
            && input.parse::<LitStr>().unwrap().value() == "opaque"
        {
            Ok(ExposeStructOpts::Opaque)
        } else if let Ok(path) = input.parse::<Path>() {
            match path.get_ident() {
                Some(s) if s == "get" => Ok(ExposeStructOpts::Get),
                Some(s) if s == "set" => Ok(ExposeStructOpts::Set),
                _ => Err(syn::Error::new(
                    input.span(),
                    "expected one of `get` or `set`",
                )),
            }
        } else {
            Err(lookahead.error())
        }
    }
}
