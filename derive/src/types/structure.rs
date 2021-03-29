use syn::parse::{Parse, ParseStream};
use syn::{Lit, LitStr, Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExposeStructOpts {
    Opaque,
    Get {
        is_simple: bool,
    },
    Set {
        is_simple: bool,
    },

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
                Some(s) if s == "get" => Ok(ExposeStructOpts::Get { is_simple: false }),
                Some(s) if s == "set" => Ok(ExposeStructOpts::Set { is_simple: false }),
                Some(s) if s == "get_simple" => Ok(ExposeStructOpts::Get { is_simple: true }),
                Some(s) if s == "set_simple" => Ok(ExposeStructOpts::Set { is_simple: true }),
                _ => Err(syn::Error::new(
                    input.span(),
                    "expected one of `get` or `set`, `get_simple`, `set_simple`",
                )),
            }
        } else {
            Err(lookahead.error())
        }
    }
}
