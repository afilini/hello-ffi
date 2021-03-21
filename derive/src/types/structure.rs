use syn::parse::{Parse, ParseStream};
use syn::{Lit, LitStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposeStructOpts {
    Opaque,
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
        } else {
            Err(lookahead.error())
        }
    }
}
