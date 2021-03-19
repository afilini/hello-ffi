use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Path, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExposeTraitOption {
    Original(Token![=], LitStr),
}

impl Parse for ExposeTraitOption {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.parse::<Path>()?.get_ident() {
            Some(s) if s == "original" => {
                Ok(ExposeTraitOption::Original(input.parse()?, input.parse()?))
            }
            _ => Err(syn::Error::new(input.span(), "expected `original = ...`")),
        }
    }
}
